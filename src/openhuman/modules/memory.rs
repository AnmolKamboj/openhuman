//! The host half of `ai.tinyhumans.tinymemory.Memory`.
//!
//! [`ModuleMemoryProvider`] implements `MemoryProvider` by forwarding each method
//! to the loaded module. Because the wire surface mirrors the trait one method
//! for one method, there is no translation layer here — only the bus call, the
//! error mapping, and the two decisions below.
//!
//! # Construction is synchronous and does no I/O
//!
//! `memory::binding::build` is called from `CoreContext::memory_binding`, which
//! roughly four thousand pre-boot tests invoke with no tokio runtime at all. So
//! [`ModuleMemoryProvider::new`] cannot load the module, cannot dial the bus, and
//! cannot await anything. It stores its configuration and resolves on first use,
//! the same contract `driver::embedded` follows.
//!
//! That has one consequence worth stating plainly, because it looks like a
//! shortcut and is not:
//!
//! ## `capabilities()` is answered statically
//!
//! `MemoryProvider::capabilities` is a **synchronous** method, and the module can
//! only answer it over the bus. It therefore cannot be asked here.
//!
//! It does not need to be. The module serves exactly the mandatory three —
//! `tinymemory-tinycortex` advertises Core, Recall and Portability because the
//! optional families need a host's configuration, embedding compute and job
//! queue — and that is a property of the artifact's *source*, fixed at the
//! version the registry pins, not something to discover at runtime. So this
//! returns [`Capabilities::mandatory`], and [`ModuleMemoryProvider::verify`]
//! cross-checks it against the module's own answer on first use and logs loudly
//! on disagreement.
//!
//! Guessing high would be the dangerous direction: the kernel filters its RPC
//! surface and agent-tool assembly from this set, so an overstated capability
//! registers methods that answer errors. Guessing exactly is safe; the
//! cross-check catches a future artifact that widens its scope.
//!
//! # Errors round-trip through the shared table
//!
//! `tinymemory_api::wire` maps a `MemoryError` to a `(name, message)` pair and
//! back, and **both ends use it**. Reimplementing the mapping here is what would
//! let a `PathEscape` arrive as an `Invalid`, silently reclassifying a sandbox
//! escape as a caller mistake.

use std::sync::Arc;

use async_trait::async_trait;
use tinymemory_api::capabilities::Capabilities;
use tinymemory_api::error::MemoryError;
use tinymemory_api::health::MemoryHealth;
use tinymemory_api::provider::mandatory::{MemoryCore, MemoryPortability, MemoryRecall};
use tinymemory_api::provider::types::{ExportPage, ExportRecord, ImportOutcome, SourceScope};
use tinymemory_api::provider::MemoryProvider;
use tinymemory_api::recall::OwnedRecallOpts;
use tinymemory_api::types::{MemoryCategory, MemoryEntry, MemoryTaint, NamespaceSummary};
use tinymemory_api::wire;

use super::{host, ops, registry};
use crate::openhuman::config::Config;

/// Registry id of the module these calls go to.
pub const MODULE_ID: &str = "tinymemory";

/// The `[modules]` policy this process was booted with.
///
/// # Why a process-global and not a constructor argument
///
/// `memory::binding::build` is where a module driver is constructed, and it
/// receives only a workspace dir and a `MemorySubsystemConfig`. What
/// [`ops::ensure_loaded`] needs is `modules.{enabled, allow_download,
/// install_dir}`, which lives on the full `Config` — and threading a whole
/// `Config` down through `MemoryBinding::for_workspace` would widen that
/// function's dependency and change a cache key that ~4000 pre-boot tests hit.
///
/// So the policy is published once during boot instead. This is the same shape
/// `tinymemory_core::embedding_host` and `api::product` already use, and for the
/// same stated reason: the construction sites sit too deep to thread through.
///
/// # Unset means disabled, deliberately
///
/// A pre-boot test, or a host that never called [`set_modules_policy`], gets
/// `None` — and [`policy`] then reports modules disabled rather than assuming
/// permissive defaults. Defaulting `enabled` to `true` here would silently
/// ignore an operator who turned modules off, and would let a unit test reach
/// for a download.
static MODULES_POLICY: std::sync::OnceLock<Arc<Config>> = std::sync::OnceLock::new();

/// Publish the config a module driver should load against.
///
/// Call once during boot, before any workspace is bound. Later calls are
/// ignored — a driver already resolved against the first value must not have the
/// policy change underneath it.
pub fn set_modules_policy(config: Arc<Config>) {
    let _ = MODULES_POLICY.set(config);
}

/// The published policy, if boot supplied one.
fn policy() -> Option<&'static Arc<Config>> {
    MODULES_POLICY.get()
}

/// A memory driver served by the loaded `tinymemory` module.
pub struct ModuleMemoryProvider {
    /// The id reported by [`MemoryProvider::driver_id`].
    driver_id: String,
    /// The config to load against, when the caller had one to give.
    ///
    /// `None` is the binding-site case: `build` has no `Config`, so the provider
    /// falls back to the policy published at boot. Tests pass one explicitly.
    config: Option<Arc<Config>>,
    /// Set once the module has answered `Capabilities`, so the cross-check runs
    /// once rather than per call.
    verified: std::sync::OnceLock<()>,
}

impl std::fmt::Debug for ModuleMemoryProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `Config` is not rendered: it carries credentials.
        f.debug_struct("ModuleMemoryProvider")
            .field("driver_id", &self.driver_id)
            .finish_non_exhaustive()
    }
}

impl ModuleMemoryProvider {
    /// Bind the module-backed driver.
    ///
    /// Synchronous and I/O-free by requirement — see the module docs. Nothing is
    /// loaded until the first call.
    #[must_use]
    pub fn new(config: Arc<Config>) -> Self {
        Self::with_optional_config(Some(config))
    }

    /// Bind against the policy published by [`set_modules_policy`].
    ///
    /// This is what `memory::binding::build` uses, because it has no `Config` to
    /// hand over. If boot published nothing, every call reports the module
    /// unavailable rather than guessing a permissive default.
    #[must_use]
    pub fn from_boot_policy() -> Self {
        Self::with_optional_config(None)
    }

    fn with_optional_config(config: Option<Arc<Config>>) -> Self {
        Self {
            driver_id: registry::find(MODULE_ID)
                .map_or_else(|| MODULE_ID.to_string(), |record| record.id.to_string()),
            config,
            verified: std::sync::OnceLock::new(),
        }
    }

    /// Ensure the module is serving, and hand back a proxy for its object.
    ///
    /// `operation` identifies the forwarded call (e.g. `"store"`, `"recall"`)
    /// for the diagnostic below. Never `namespace`, `key`, `content`, or any
    /// record value — those are user memory content, not correlation fields.
    async fn proxy(&self, operation: &str) -> Result<tinybus::Proxy, MemoryError> {
        log::debug!(
            "[modules:memory] driver_id={} operation={operation} resolving module proxy",
            self.driver_id,
        );
        let config = self.config.as_ref().or_else(|| policy()).ok_or_else(|| {
            MemoryError::Other(anyhow::anyhow!(
                "the module host policy was never published, so module '{MODULE_ID}' \
                 cannot be loaded; call modules::memory::set_modules_policy during boot"
            ))
        })?;
        ops::ensure_loaded(config, MODULE_ID)
            .await
            .map_err(|message| MemoryError::Other(anyhow::anyhow!(message)))?;

        let record = registry::find(MODULE_ID)
            .ok_or_else(|| MemoryError::Other(anyhow::anyhow!("unknown module '{MODULE_ID}'")))?;
        let runtime = host::runtime().await.map_err(|error| {
            MemoryError::Other(anyhow::anyhow!("the module bus is not running: {error}"))
        })?;
        let proxy = runtime
            .proxy(record.bus_name, record.object_path)
            .map_err(|error| MemoryError::Other(anyhow::anyhow!(error.to_string())))?;

        self.verify(&proxy).await;
        Ok(proxy)
    }

    /// Cross-check the module's advertised capabilities against what this build
    /// assumes, once per process.
    ///
    /// Logged rather than fatal. A module that advertises *more* than the
    /// mandatory three is not dangerous — the kernel simply will not use the
    /// extra families, because it filtered its surface from the static set — but
    /// it does mean the registry pin and the artifact have diverged, which is
    /// worth seeing. A module advertising *less* is the real problem, and says so.
    async fn verify(&self, proxy: &tinybus::Proxy) {
        if self.verified.get().is_some() {
            return;
        }
        match proxy.call::<Capabilities>("Capabilities", ()).await {
            Ok(actual) => {
                let assumed = Capabilities::mandatory();
                if actual != assumed {
                    log::warn!(
                        "[modules:memory] the module advertises {actual:?} but this build \
                         assumes {assumed:?}; the registry pin and the artifact have diverged"
                    );
                }
            }
            Err(error) => {
                log::warn!("[modules:memory] could not read module capabilities: {error}");
            }
        }
        let _ = self.verified.set(());
    }
}

/// Map a bus failure back onto a [`MemoryError`].
///
/// Uses the shared table so the host and the module cannot disagree about what a
/// name means. An unrecognised name becomes `Other`, never `Invalid`.
fn from_bus(error: &tinybus::Error) -> MemoryError {
    wire::from_wire(error.wire_name(), &error.to_string())
}

#[async_trait]
impl MemoryProvider for ModuleMemoryProvider {
    fn driver_id(&self) -> &str {
        &self.driver_id
    }

    /// The mandatory three. See the module docs on why this is static.
    fn capabilities(&self) -> Capabilities {
        Capabilities::mandatory()
    }

    async fn health(&self) -> MemoryHealth {
        // An unreachable module is a *health* answer, not an error: that is the
        // question this method exists to answer, and returning `Down` is how
        // status output shows an unsupported platform or a refused artifact.
        match self.proxy("health").await {
            Ok(proxy) => proxy
                .call::<MemoryHealth>("Health", ())
                .await
                .unwrap_or_else(|error| MemoryHealth::down(error.to_string())),
            Err(error) => MemoryHealth::down(error.to_string()),
        }
    }

    async fn shutdown(&self) -> Result<(), MemoryError> {
        // Deliberately does not load the module in order to shut it down: a
        // shutdown on a driver that was never used should be a no-op, not a
        // download. tinybus never unloads a library anyway, so this releases
        // backend resources only.
        if self.verified.get().is_none() {
            return Ok(());
        }
        let proxy = self.proxy("shutdown").await?;
        proxy
            .call::<()>("Shutdown", ())
            .await
            .map_err(|error| from_bus(&error))
    }
}

#[async_trait]
impl MemoryCore for ModuleMemoryProvider {
    async fn store(
        &self,
        namespace: &str,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
        taint: MemoryTaint,
    ) -> Result<(), MemoryError> {
        // No log line carries `namespace`, `key` or `content`: all three are user
        // memory content.
        self.proxy()
            .await?
            .call::<()>(
                "Store",
                (
                    namespace,
                    key,
                    content,
                    category,
                    session_id.map(str::to_string),
                    taint,
                ),
            )
            .await
            .map_err(|error| from_bus(&error))
    }

    async fn get(&self, namespace: &str, key: &str) -> Result<Option<MemoryEntry>, MemoryError> {
        self.proxy()
            .await?
            .call("Get", (namespace, key))
            .await
            .map_err(|error| from_bus(&error))
    }

    async fn forget(&self, namespace: &str, key: &str) -> Result<bool, MemoryError> {
        self.proxy()
            .await?
            .call("Forget", (namespace, key))
            .await
            .map_err(|error| from_bus(&error))
    }

    async fn list(
        &self,
        namespace: Option<&str>,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        self.proxy()
            .await?
            .call(
                "List",
                (
                    namespace.map(str::to_string),
                    category.cloned(),
                    session_id.map(str::to_string),
                ),
            )
            .await
            .map_err(|error| from_bus(&error))
    }

    async fn namespaces(&self) -> Result<Vec<NamespaceSummary>, MemoryError> {
        self.proxy()
            .await?
            .call("Namespaces", ())
            .await
            .map_err(|error| from_bus(&error))
    }
}

#[async_trait]
impl MemoryRecall for ModuleMemoryProvider {
    async fn recall(
        &self,
        query: &str,
        limit: usize,
        opts: &OwnedRecallOpts,
        scope: Option<&SourceScope>,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        // `scope` crosses as a value because the driver must apply it as a query
        // predicate internally; narrowing the result here instead would let the
        // module spend its `limit` on entries the caller may not see.
        self.proxy()
            .await?
            .call("Recall", (query, limit, opts, scope.cloned()))
            .await
            .map_err(|error| from_bus(&error))
    }
}

#[async_trait]
impl MemoryPortability for ModuleMemoryProvider {
    async fn export_page(
        &self,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<ExportPage, MemoryError> {
        self.proxy()
            .await?
            .call("ExportPage", (cursor.map(str::to_string), limit))
            .await
            .map_err(|error| from_bus(&error))
    }

    async fn import_records(
        &self,
        records: Vec<ExportRecord>,
    ) -> Result<ImportOutcome, MemoryError> {
        self.proxy()
            .await?
            .call("ImportRecords", (records,))
            .await
            .map_err(|error| from_bus(&error))
    }
}

#[cfg(test)]
#[path = "memory_tests.rs"]
mod tests;
