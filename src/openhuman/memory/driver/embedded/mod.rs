//! The embedded `tinycortex` memory driver — the in-process engine behind the
//! [`MemoryProvider`] contract.
//!
//! This is the driver bound for [`DriverClass::Embedded`](crate::core::subsystem::DriverClass),
//! replacing the `NullMemoryProvider` placeholder M2b used to prove the binding
//! plumbing.
//!
//! ## It re-shapes, it does not re-implement
//!
//! Every method here delegates to an existing host call. There is no
//! retrieval, ranking, chunking, or storage logic in this directory — if a
//! change to a file under `driver/embedded/` starts to *decide* something about
//! memory rather than translate a call, it belongs in the engine instead.
//!
//! ## Construction is synchronous and does no I/O
//!
//! [`crate::openhuman::memory::binding::for_workspace`] is reached from
//! [`CoreContext::memory_binding`](crate::core::runtime::CoreContext::memory_binding),
//! which is documented as synchronous and I/O-free and is called from roughly
//! four thousand pre-boot unit tests with **no tokio runtime**. Meanwhile
//! [`MemoryClient::from_workspace_dir`](crate::openhuman::memory::store::MemoryClient::from_workspace_dir)
//! opens SQLite, runs migrations, and `tokio::spawn`s the ingestion worker — it
//! panics outside a runtime.
//!
//! So the driver holds a [`tokio::sync::OnceCell`] and resolves its client on
//! the first *async contract call*, not at bind time. [`Self::new`] touches
//! nothing on disk; `constructing_the_driver_does_no_io_and_needs_no_runtime`
//! pins that.
//!
//! ## Capability honesty
//!
//! [`MemoryProvider::capabilities`] must advertise only what is reachable —
//! [`tinycortex_api::provider::audit_provider`] compares the advertised set
//! against the `as_*` accessors and fails on either kind of disagreement.
//! [`advertised_capabilities`] and the `as_*` overrides therefore widen
//! together, once per M3 step: M3a landed the mandatory three, M3b documents /
//! graph / tool memory, M3c ingest / tree / entities, and M3d the last four —
//! diff, goals, sources and maintenance. The advertised set is now
//! [`Capabilities::all`], which is what the whole milestone existed to reach: a
//! bound context and an unbound one finally agree, so `memory_capabilities()`
//! becomes safe to gate on (M4).

mod core_family;
#[cfg(feature = "memory-git")]
mod diff;
mod documents;
mod entities;
mod goals;
mod graph;
mod ingest;
mod maintenance;
mod portability;
mod recall;
mod sources;
mod tool_memory;
mod tree;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use tinycortex_api::capabilities::Capabilities;
#[cfg(not(feature = "memory-git"))]
use tinycortex_api::capabilities::Capability;
use tinycortex_api::error::MemoryError;
use tinycortex_api::health::MemoryHealth;
use tinycortex_api::provider::MemoryProvider;
use tokio::sync::OnceCell;

use crate::openhuman::config::schema::MemoryHooksConfig;
use crate::openhuman::config::Config;
use crate::openhuman::memory::global;
use crate::openhuman::memory::store::MemoryClientRef;
use crate::openhuman::memory::Memory;

/// The stable [`MemoryProvider::driver_id`] of this driver.
///
/// Matches the `[subsystems.memory] driver` default (`default_memory_driver`
/// in `config::schema::subsystems`), so a default-configured host reports the
/// same id from config and from the bound provider.
///
/// **Re-exported, not re-declared.** The same id is what
/// [`tinymemory::registry`] reserves at [`DriverClass::Embedded`], and
/// admission is decided there. Two independent string literals that must agree
/// is precisely the kind of pair that silently stops agreeing: the day one is
/// edited, `admit` would stop recognising this driver and every bind would fall
/// back to the null placeholder — loudly in the logs, but with memory writes
/// discarded for the whole run. One constant, one definition, no drift.
pub use tinymemory::registry::TINYCORTEX_DRIVER_ID as EMBEDDED_DRIVER_ID;

/// The families this driver advertises: **all thirteen**.
///
/// Written as [`Capabilities::all`] rather than a `mandatory().with(…)` chain
/// of thirteen terms, because the two are now the same value and the equality is
/// the point — `embedded_driver_advertises_every_capability` asserts exactly
/// that. `Capability` is deliberately not `#[non_exhaustive]`, so a fourteenth
/// family added to the contract widens `all()` here and fails
/// `audit_provider` until its accessor lands, which is the intended pressure.
fn advertised_capabilities() -> Capabilities {
    // Without `memory-git` there is no git ledger, so the diff family has no
    // implementation to reach. Dropping it here is not cosmetic: a provider
    // that advertises a capability whose accessor returns `None` fails
    // `audit_provider`, and callers are entitled to trust the advertised set
    // rather than probing every accessor.
    #[cfg(not(feature = "memory-git"))]
    {
        Capabilities::all().without(Capability::Diff)
    }
    #[cfg(feature = "memory-git")]
    {
        Capabilities::all()
    }
}

/// The in-process tinycortex driver for one workspace.
pub struct EmbeddedMemoryProvider {
    workspace_dir: PathBuf,
    /// Hook budgets from `[subsystems.memory.hooks]`. Carried so the
    /// auto-recall / auto-capture guard (M4) has them without re-reading
    /// config; no family in M3a consults them.
    hooks: MemoryHooksConfig,
    /// Resolved lazily — see the module docs for why this is not a plain
    /// `MemoryClientRef`.
    client: OnceCell<MemoryClientRef>,
    /// Resolved lazily, for the same reason as [`Self::client`] — see
    /// [`Self::config`].
    config: OnceCell<Config>,
}

impl EmbeddedMemoryProvider {
    /// Build a driver for `workspace_dir`.
    ///
    /// Synchronous, infallible, and I/O-free by contract: the workspace is not
    /// created, SQLite is not opened, and no task is spawned until the first
    /// async contract call.
    pub fn new(workspace_dir: impl Into<PathBuf>, hooks: MemoryHooksConfig) -> Self {
        Self {
            workspace_dir: workspace_dir.into(),
            hooks,
            client: OnceCell::new(),
            config: OnceCell::new(),
        }
    }

    /// The workspace this driver is bound to.
    pub fn workspace_dir(&self) -> &Path {
        &self.workspace_dir
    }

    /// The configured hook budgets.
    pub fn hooks(&self) -> MemoryHooksConfig {
        self.hooks
    }

    /// The backing client, constructing it on first use.
    ///
    /// Goes through [`global::client_for_workspace`] rather than
    /// `MemoryClient::from_workspace_dir` so a workspace that already has the
    /// process-global client reuses it — two clients over one workspace means
    /// two ingestion workers against the same SQLite file.
    async fn client(&self) -> Result<&MemoryClientRef, MemoryError> {
        self.client
            .get_or_try_init(|| async {
                global::client_for_workspace(&self.workspace_dir).map_err(|error| {
                    log::warn!(
                        "[memory:driver:embedded] workspace={} client init failed: {error}",
                        self.workspace_dir.display()
                    );
                    MemoryError::Other(anyhow::anyhow!(error))
                })
            })
            .await
    }

    /// The `Memory` handle every mandatory family delegates through.
    pub(super) async fn memory(&self) -> Result<Arc<dyn Memory>, MemoryError> {
        Ok(self.client().await?.memory_handle())
    }

    /// The host [`Config`] the chunk / tree / ingest layers are addressed
    /// through, constructing it on first use.
    ///
    /// ## Why the driver needs a `Config` at all
    ///
    /// The families landed in M3a/M3b reach storage through `MemoryClient`,
    /// which is rooted at a workspace directory and needs nothing else. The
    /// three M3c families do not have that luxury: every entry point under
    /// `memory::tree::tree_runtime`, `memory::store::chunks` and
    /// `memory::ingest_pipeline` takes `&Config` and funnels it through
    /// [`tinycortex::engine_config`](crate::openhuman::memory::tinycortex::engine_config),
    /// which derives the engine's `MemoryConfig` — workspace root, embedding
    /// model/dimensions, tree token budgets — from it. There is no
    /// workspace-only door into those layers, and inventing one would be
    /// engine logic.
    ///
    /// ## Why it is loaded, not defaulted
    ///
    /// `Config::default()` would silently substitute default embedding
    /// dimensions and would report "no summarization provider" for a host that
    /// has one configured. So the real config is loaded — the one belonging to
    /// **this driver's** workspace, via
    /// [`load_config_for_workspace_with_timeout`](crate::openhuman::config::load_config_for_workspace_with_timeout).
    ///
    /// Re-anchoring `workspace_dir` after a process-global load is not enough,
    /// and that is what this used to do: everything *else* in the snapshot —
    /// embedding routes, model dimensions, provider credentials, tree budgets —
    /// would still be whichever workspace the process-global active-user /
    /// `OPENHUMAN_WORKSPACE` resolution named at first use. A driver bound to B
    /// would then run A's settings over B's files, sending data to the wrong
    /// endpoint or writing an index at the wrong dimension. The loader resolves
    /// the config file beside `self.workspace_dir` instead, and only falls back
    /// to the process-global one when the workspace has no config of its own.
    ///
    /// Lazy for the same reason as [`Self::client`] — loading is async and
    /// touches disk, and bind time is neither.
    pub(super) async fn config(&self) -> Result<&Config, MemoryError> {
        self.config
            .get_or_try_init(|| async {
                let config = crate::openhuman::config::load_config_for_workspace_with_timeout(
                    &self.workspace_dir,
                )
                .await
                .map_err(|error| {
                    log::warn!(
                        "[memory:driver:embedded] workspace={} config load failed: {error}",
                        self.workspace_dir.display()
                    );
                    MemoryError::Other(anyhow::anyhow!("memory driver config load: {error}"))
                })?;
                debug_assert_eq!(config.workspace_dir, self.workspace_dir);
                Ok(config)
            })
            .await
    }
}

/// Maps an engine `anyhow` failure onto the contract's error type.
///
/// The engine's [`Memory`] trait is deliberately `anyhow`-typed (it is an
/// internal storage abstraction with heterogeneous backends), so everything it
/// returns is opaque and lands in [`MemoryError::Other`]. The typed variants —
/// `Invalid`, `NotFound`, `Unsupported` — are constructed *here*, by the
/// driver, where the reason is actually known.
pub(super) fn engine_error(error: anyhow::Error) -> MemoryError {
    MemoryError::Other(error)
}

/// Maps a host-layer `Result<_, String>` failure onto the contract's error
/// type, tagging it with the contract method that produced it.
///
/// The host's memory layers are `String`-typed end to end, so nothing about the
/// failure is machine-readable and [`MemoryError::Other`] is the honest
/// variant. A family that can genuinely identify a caller error — see
/// `tool_memory`'s `classify_put_rule` — constructs [`MemoryError::Invalid`]
/// itself rather than widening this helper.
pub(super) fn host_error(context: &str, error: String) -> MemoryError {
    log::warn!("[memory:driver:embedded] {context} failed: {error}");
    MemoryError::Other(anyhow::anyhow!("{context}: {error}"))
}

#[async_trait]
impl MemoryProvider for EmbeddedMemoryProvider {
    fn driver_id(&self) -> &str {
        EMBEDDED_DRIVER_ID
    }

    fn capabilities(&self) -> Capabilities {
        advertised_capabilities()
    }

    async fn health(&self) -> MemoryHealth {
        // Deliberately does **not** force the client. `health` is called on
        // bind and for status output; making it the thing that opens SQLite
        // would move the I/O the lazy `OnceCell` exists to defer back onto the
        // status path — and, worse, would create the workspace as a side
        // effect of asking whether it exists.
        let Some(client) = self.client.get() else {
            return MemoryHealth::Ready;
        };
        if client.memory_handle().health_check().await {
            MemoryHealth::Ready
        } else {
            // No path in the reason: this string is logged and rendered in
            // `subsystems_status`.
            MemoryHealth::down("memory workspace or database file is missing")
        }
    }

    // The optional-family accessors. Each must move in lockstep with
    // `advertised_capabilities`; `audit_provider` fails on either half alone.
    fn as_documents(&self) -> Option<&dyn tinycortex_api::provider::MemoryDocuments> {
        Some(self)
    }

    fn as_graph(&self) -> Option<&dyn tinycortex_api::provider::MemoryGraph> {
        Some(self)
    }

    fn as_tool_memory(&self) -> Option<&dyn tinycortex_api::provider::MemoryToolMemory> {
        Some(self)
    }

    fn as_ingest(&self) -> Option<&dyn tinycortex_api::provider::MemoryIngest> {
        Some(self)
    }

    fn as_tree(&self) -> Option<&dyn tinycortex_api::provider::MemoryTree> {
        Some(self)
    }

    fn as_entities(&self) -> Option<&dyn tinycortex_api::provider::MemoryEntities> {
        Some(self)
    }

    /// `None` without `memory-git`, in lockstep with
    /// [`advertised_capabilities`] — `audit_provider` fails on either half
    /// alone, which is exactly the check that keeps these two from drifting.
    fn as_diff(&self) -> Option<&dyn tinycortex_api::provider::MemoryDiff> {
        #[cfg(not(feature = "memory-git"))]
        {
            None
        }
        #[cfg(feature = "memory-git")]
        {
            Some(self)
        }
    }

    fn as_goals(&self) -> Option<&dyn tinycortex_api::provider::MemoryGoals> {
        Some(self)
    }

    fn as_sources(&self) -> Option<&dyn tinycortex_api::provider::MemorySourceSink> {
        Some(self)
    }

    fn as_maintenance(&self) -> Option<&dyn tinycortex_api::provider::MemoryMaintenance> {
        Some(self)
    }

    // `shutdown` keeps the contract's no-op default on purpose. The ingestion
    // worker belongs to the shared `MemoryClient`, which other subsystems hold
    // too; a driver must not tear down a handle it does not own. The default is
    // idempotent by construction, which the contract requires.
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;

#[cfg(test)]
pub(super) mod test_support {
    use super::*;
    use tempfile::TempDir;

    /// A driver over a fresh temp workspace. The `TempDir` must outlive the
    /// provider, so it is returned alongside.
    ///
    /// The [`Config`] `OnceCell` is **seeded** rather than left to resolve
    /// through [`crate::openhuman::config::load_config_with_timeout`]: that
    /// path reads the real config file and the process-global
    /// `OPENHUMAN_WORKSPACE`, which would make every M3c test non-hermetic and
    /// order-dependent. The seeded shape is the same one
    /// `tree::retrieval::source_scope_tests::test_config` uses — a default
    /// `Config` re-rooted at the temp workspace, with an inert embedder.
    pub fn fresh_driver() -> (TempDir, EmbeddedMemoryProvider) {
        let tmp = TempDir::new().expect("temp workspace");
        let workspace = tmp.path().join("ws");
        let provider = EmbeddedMemoryProvider::new(workspace.clone(), MemoryHooksConfig::default());

        let mut config = Config::default();
        config.workspace_dir = workspace;
        config.config_path = tmp.path().join("config.toml");
        config.memory_tree.embedding_endpoint = None;
        config.memory_tree.embedding_model = None;
        config.memory_tree.embedding_strict = false;
        provider
            .config
            .set(config)
            .map_err(|_| "config cell already seeded")
            .expect("seed test config");

        (tmp, provider)
    }
}
