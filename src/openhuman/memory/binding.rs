//! Per-workspace memory-driver binding — the memory subsystem's half of
//! `docs/specs/kernel.md` §3.1 (one driver per subsystem per process, per
//! workspace here), §3.4 (fail-closed trust), and §3.7 (a fallback is never
//! silent).
//!
//! ## Reached through [`CoreContext`], never through a global slot
//!
//! The binding is resolved by
//! [`CoreContext::memory_binding`](crate::core::runtime::CoreContext::memory_binding),
//! which keys on the context's workspace dir. The cache below is deliberately
//! shaped like
//! [`memory::people::store::for_workspace`](crate::openhuman::memory::people::store::for_workspace)
//! — a **workspace-keyed map** — and deliberately *not* like
//! [`memory::global`](crate::openhuman::memory::global), which is a single slot
//! holding "the one active-user workspace".
//!
//! That shape choice carries a real correctness property for free.
//! `memory::global::init` needs an explicit clear-on-failed-rebind guard so a
//! failed switch to workspace B cannot leave callers writing into workspace A.
//! With a workspace-keyed map there is no shared slot to go stale: a context
//! bound to B resolves the entry for B or falls back, and can never be handed
//! A's driver. Pinned by
//! `failed_bind_never_returns_previous_workspace_binding` in
//! `src/core/runtime/context.rs`.
//!
//! ## Two vocabularies meet here, on purpose
//!
//! [`crate::openhuman::memory::api`] is the host-owned memory contract: `MemoryProvider`,
//! `Capabilities`, `MemoryHealth`. [`crate::core::subsystem`] is the kernel's
//! *generic* driver vocabulary shared with the subsystems that come after
//! memory: `DriverClass`, `DriverCapabilities`, `DriverHealth`, `BoundDriver`.
//! This module is the adapter between them — the only place in the tree where
//! the conversion lives. `DriverClass` is reused from the kernel rather than
//! redefined here precisely because it is a *host* fact about how a driver was
//! bound, identical for every subsystem.
//!
//! The built-in driver is the compiled TinyMemory TinyBus module. The host no
//! longer exposes an embedded engine class for memory.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};

use crate::openhuman::memory::api::capabilities::Capabilities;
use crate::openhuman::memory::api::health::MemoryHealth;
use crate::openhuman::memory::api::null::{NullMemoryProvider, NULL_DRIVER_ID};
use crate::openhuman::memory::api::provider::MemoryProvider;
use crate::openhuman::memory::api::CONTRACT_VERSION;
use crate::openhuman::memory::guard::{GuardPolicy, MemoryGuard};

use crate::core::subsystem::{
    BoundDriver, DriverCapabilities, DriverClass, DriverHealth, SubsystemSlot,
};
use crate::openhuman::config::schema::MemorySubsystemConfig;

/// Registry id of the built-in TinyMemory module.
pub(crate) const MODULE_ID: &str = "tinymemory";

/// Why a bind fell back to the placeholder driver.
///
/// `reason` is operator-facing: it is logged, published on the event bus, and
/// rendered in status. It must therefore never interpolate `credential_ref` or
/// `endpoint` from [`crate::openhuman::config::schema::MemoryDriverConfig`],
/// which carries a manual redacting `Debug` for exactly that reason. Pinned by
/// `fallback_reason_never_contains_credential_ref_or_endpoint`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FallbackReason {
    /// The driver id that was asked for in `[subsystems.memory] driver`.
    pub configured_driver: String,
    /// Why it was refused.
    pub reason: String,
}

/// One bound memory driver, for one workspace.
pub struct MemoryBinding {
    provider: Arc<dyn MemoryProvider>,
    guard: Arc<MemoryGuard>,
    driver_id: String,
    class: DriverClass,
    /// Asked **once**, at bind time, and cached here. The contract's
    /// `MemoryProvider::capabilities` doc is normative on this ("asked once at
    /// bind time and cached"): re-asking would let a driver's advertised
    /// surface drift underneath an already-filtered RPC/tool registration.
    capabilities: Capabilities,
    fallback: Option<FallbackReason>,
}

impl MemoryBinding {
    /// The bound driver.
    pub fn provider(&self) -> &Arc<dyn MemoryProvider> {
        &self.provider
    }

    pub(crate) fn unguarded_provider(&self) -> &Arc<dyn MemoryProvider> {
        &self.provider
    }

    pub fn guard(&self) -> Arc<MemoryGuard> {
        Arc::clone(&self.guard)
    }

    pub fn disables_memory(&self) -> bool {
        self.class == DriverClass::Null && self.fallback.is_none()
    }

    /// The id of the driver that actually bound — `"null"` after a fallback,
    /// not the id that was asked for (that is in [`Self::fallback`]).
    pub fn driver_id(&self) -> &str {
        &self.driver_id
    }

    /// How the bound driver was reached. A host fact, never self-reported.
    pub fn class(&self) -> DriverClass {
        self.class
    }

    /// The cached capability set. Cheap: `Capabilities` is a `Copy` bitset.
    pub fn capabilities(&self) -> Capabilities {
        self.capabilities
    }

    /// `Some` when this binding is a fallback; `None` when the configured
    /// driver bound as asked.
    pub fn fallback(&self) -> Option<&FallbackReason> {
        self.fallback.as_ref()
    }

    /// This binding in the kernel's generic vocabulary, for the subsystem
    /// registry and `subsystems_status` (kernel.md §6 item 6). This is the
    /// memory adapter `core::subsystem`'s module docs said would land later.
    pub fn to_bound_driver(&self) -> BoundDriver {
        BoundDriver {
            slot: SubsystemSlot::Memory,
            id: self.driver_id.clone(),
            class: self.class,
            capabilities: to_driver_capabilities(self.capabilities),
            health: DriverHealth::Ready,
            contract_version: CONTRACT_VERSION,
            fell_back_from: self.fallback.as_ref().map(|f| f.configured_driver.clone()),
        }
    }
}

/// Convert the memory contract's typed capability set into the kernel's opaque
/// one. The kernel deliberately does not know memory's family vocabulary.
pub fn to_driver_capabilities(capabilities: Capabilities) -> DriverCapabilities {
    capabilities.iter().map(|c| c.as_str()).collect()
}

/// Convert the memory contract's health into the kernel's. A total three-arm
/// match, which is why both enums were shaped one-for-one.
pub fn to_driver_health(health: MemoryHealth) -> DriverHealth {
    match health {
        MemoryHealth::Ready => DriverHealth::Ready,
        MemoryHealth::Degraded { reason } => DriverHealth::Degraded { reason },
        MemoryHealth::Down { reason } => DriverHealth::Down { reason },
    }
}

/// The capability set assumed when nothing is bound.
///
/// **Deliberately the full set.** This mirrors
/// [`crate::core::all`]'s `group_allowed`, which returns `true` when there is
/// no ambient context: roughly 4000 unit tests run pre-boot with no bound
/// driver, and a deny-by-default here would fail all of them at once. Denying a
/// capability is only ever correct *after* a driver has actually answered
/// `capabilities()`.
pub fn unbound_default_capabilities() -> Capabilities {
    Capabilities::all()
}

/// Decide, from config alone, whether the configured driver may bind.
///
/// Pure — no I/O, no globals — so the fail-closed trust rule is unit-testable
/// without booting anything.
///
/// # Errors
///
/// Returns the [`FallbackReason`] to record and publish when the configured
/// driver is refused. Callers fall back rather than failing: kernel.md §3.7
/// requires the subsystem stay bound, loudly.
pub fn admit(cfg: &MemorySubsystemConfig) -> Result<(String, DriverClass), FallbackReason> {
    let configured_id = cfg.driver.trim();
    if configured_id.is_empty() {
        return Err(FallbackReason {
            configured_driver: String::new(),
            reason: "[subsystems.memory] driver is empty".to_string(),
        });
    }

    let refuse = |reason: &str| FallbackReason {
        configured_driver: configured_id.to_string(),
        reason: reason.to_string(),
    };

    // Temporary persisted-config alias. The schema still comes from the
    // legacy contract until its remaining engine callers are moved onto the
    // host-owned copy; both values bind the compiled module and report its
    // actual id. Remove this alias with that final schema cutover.
    const LEGACY_MODULE_ID: &str = "tinycortex";
    let id = if configured_id == LEGACY_MODULE_ID {
        MODULE_ID
    } else {
        configured_id
    };

    // The two built-ins need no `[subsystems.memory.drivers.<id>]` entry.
    let Some(entry) = cfg
        .drivers
        .get(configured_id)
        .or_else(|| cfg.drivers.get(id))
    else {
        return match id {
            NULL_DRIVER_ID => Ok((id.to_string(), DriverClass::Null)),
            MODULE_ID => Ok((id.to_string(), DriverClass::Module)),
            _ => Err(refuse(&format!(
                "driver '{id}' is not built in; add [subsystems.memory.drivers.{id}] with an explicit class line"
            ))),
        };
    };

    let class = match entry.class.as_deref() {
        None if id == NULL_DRIVER_ID => DriverClass::Null,
        None if id == MODULE_ID => DriverClass::Module,
        None => {
            return Err(refuse(&format!(
                "driver '{id}' is not built in and requires an explicit class line"
            )))
        }
        Some(raw) => DriverClass::parse(raw).map_err(|e| refuse(&e))?,
    };

    if class == DriverClass::Embedded {
        return Err(refuse(
            "embedded memory drivers are no longer supported; use the 'tinymemory' module driver",
        ));
    }

    let built_in_class = match id {
        NULL_DRIVER_ID => Some(DriverClass::Null),
        MODULE_ID => Some(DriverClass::Module),
        _ => None,
    };
    if let Some(expected) = built_in_class {
        if class != expected {
            return Err(refuse(&format!(
                "built in driver '{configured_id}' has class '{expected}' and cannot be re-classed as '{class}'"
            )));
        }
    }

    if class == DriverClass::Module && id != MODULE_ID {
        return Err(refuse(&format!(
            "module driver '{id}' is not registered; the built-in memory module id is '{MODULE_ID}'"
        )));
    }

    if class == DriverClass::External {
        // kernel.md §3.4: fail-closed. Trust must be explicitly raised before
        // an out-of-process driver is allowed to answer for memory.
        if entry.trust_state != "trusted" {
            return Err(refuse(
                "external driver is untrusted: set trust_state = \"trusted\" \
                 under [subsystems.memory.drivers] to allow this binding",
            ));
        }
        // Distinct reason string from the trust refusal above, so the trust
        // test cannot pass for the wrong reason.
        return Err(refuse(
            "external driver transport is not implemented yet (the http adapter lands in M4)",
        ));
    }

    Ok((id.to_string(), class))
}

/// Build the binding for a workspace. Infallible by design: an inadmissible
/// driver falls back to the placeholder rather than leaving the slot empty
/// (kernel.md §3.7 — "logged loudly, surfaced in status, never silent").
fn build(workspace_dir: &Path, cfg: &MemorySubsystemConfig) -> MemoryBinding {
    match admit(cfg) {
        Ok((driver_id, class)) => {
            let (provider, reported_class): (Arc<dyn MemoryProvider>, DriverClass) =
                if class == DriverClass::Null {
                    (Arc::new(NullMemoryProvider::new()), DriverClass::Null)
                } else {
                    module_provider(workspace_dir)
                };
            let binding = bind_provider(provider, driver_id, reported_class, None);
            log::info!(
                "[memory:binding] workspace={} bound driver='{}' class={} capabilities=[{}]",
                workspace_dir.display(),
                binding.driver_id(),
                binding.class(),
                binding
                    .capabilities()
                    .iter()
                    .map(|c| c.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            );
            binding
        }
        Err(fallback) => {
            log::warn!(
                "[memory:binding] workspace={} driver '{}' refused to bind ({}); \
                 falling back to '{NULL_DRIVER_ID}' — memory writes are DISCARDED this run",
                workspace_dir.display(),
                fallback.configured_driver,
                fallback.reason
            );
            // Sync, and a no-op when the bus is not yet initialized, so this is
            // safe to call pre-boot with no `#[cfg(test)]` guard.
            crate::core::bus::BUS.publish(
                crate::core::events::DomainEvent::MemoryDriverBindFailed {
                    configured_driver: fallback.configured_driver.clone(),
                    bound_driver: NULL_DRIVER_ID.to_string(),
                    reason: fallback.reason.clone(),
                },
            );
            bind_provider(
                Arc::new(NullMemoryProvider::new()),
                NULL_DRIVER_ID.to_string(),
                DriverClass::Null,
                Some(fallback),
            )
        }
    }
}

#[cfg(all(feature = "modules", not(test)))]
fn module_provider(_workspace_dir: &Path) -> (Arc<dyn MemoryProvider>, DriverClass) {
    (
        Arc::new(crate::openhuman::modules::memory::ModuleMemoryProvider::from_boot_policy()),
        DriverClass::Module,
    )
}

#[cfg(all(feature = "modules", test))]
fn module_provider(_workspace_dir: &Path) -> (Arc<dyn MemoryProvider>, DriverClass) {
    // Unit tests do not run the full boot sequence that publishes the module
    // policy. A native module is loaded once per process and therefore captures
    // the first workspace it receives. Pin every test binding to the same
    // workspace as the process-global test client so concurrent tests cannot
    // win module initialization with an unrelated tempdir and split guarded
    // writes from legacy read-back calls.
    let workspace_dir = crate::openhuman::memory::ops::shared_memory_test_workspace();
    let mut config = crate::openhuman::config::Config::default();
    config.workspace_dir = workspace_dir.clone();
    config.modules.install_dir = Some(workspace_dir.join("modules").to_string_lossy().into_owned());
    if let Some(path) = std::env::var_os("TINYMEMORY_TEST_MODULE") {
        config
            .modules
            .overrides
            .push(crate::openhuman::config::schema::ModuleOverride {
                id: MODULE_ID.to_string(),
                path: path.to_string_lossy().into_owned(),
            });
    }
    (
        Arc::new(crate::openhuman::modules::memory::ModuleMemoryProvider::new(Arc::new(config))),
        DriverClass::Module,
    )
}

#[cfg(not(feature = "modules"))]
fn module_provider(_workspace_dir: &Path) -> (Arc<dyn MemoryProvider>, DriverClass) {
    log::warn!(
        "[memory:binding] the 'modules' feature is disabled; binding the null memory provider"
    );
    (Arc::new(NullMemoryProvider::new()), DriverClass::Null)
}

/// The single place `capabilities()` is asked. Every construction path — real
/// bind, fallback, and the test seam — goes through here, so the "asked once
/// per bind" property holds by construction rather than by convention.
fn bind_provider(
    provider: Arc<dyn MemoryProvider>,
    driver_id: String,
    class: DriverClass,
    fallback: Option<FallbackReason>,
) -> MemoryBinding {
    let capabilities = provider.capabilities();
    let guard = Arc::new(MemoryGuard::new(
        Arc::clone(&provider),
        Arc::new(GuardPolicy::new(
            driver_id.clone(),
            class,
            crate::openhuman::config::schema::MemoryHooksConfig::default(),
            "trusted",
        )),
    ));
    MemoryBinding {
        provider,
        guard,
        driver_id,
        class,
        capabilities,
        fallback,
    }
}

/// Test-only injection seam: bind an arbitrary provider through the same
/// ask-once-and-cache path [`build`] uses. Exists because [`build`] hard-codes
/// the placeholder, so the "capabilities asked exactly once" property would
/// otherwise be untestable.
#[cfg(test)]
pub(crate) fn bind_provider_for_test(
    provider: Arc<dyn MemoryProvider>,
    class: DriverClass,
) -> MemoryBinding {
    let driver_id = provider.driver_id().to_string();
    bind_provider(provider, driver_id, class, None)
}

/// Per-workspace binding cache. Same shape as
/// `memory::people::store::STORES` — see the module docs for why this is a map
/// and not a slot.
type BindingCacheKey = (PathBuf, MemorySubsystemConfig);
static BINDINGS: OnceLock<RwLock<HashMap<BindingCacheKey, Arc<MemoryBinding>>>> = OnceLock::new();

/// The bound memory driver for `workspace_dir`, constructing it on first use.
///
/// The same workspace always resolves to the same cached `Arc` (so
/// `capabilities()` is asked once); different workspaces get isolated bindings.
///
/// # Errors
///
/// Only lock poisoning. A driver that cannot bind is *not* an error here — it
/// falls back, per kernel.md §3.7.
pub fn for_workspace(
    workspace_dir: &Path,
    cfg: &MemorySubsystemConfig,
) -> Result<Arc<MemoryBinding>, String> {
    let cache = BINDINGS.get_or_init(Default::default);
    let key = (workspace_dir.to_path_buf(), cfg.clone());
    if let Some(binding) = cache
        .read()
        .map_err(|e| format!("[memory:binding] cache read lock poisoned: {e}"))?
        .get(&key)
    {
        return Ok(Arc::clone(binding));
    }

    let binding = Arc::new(build(workspace_dir, cfg));

    let mut guard = cache
        .write()
        .map_err(|e| format!("[memory:binding] cache write lock poisoned: {e}"))?;
    // Re-check under the write lock: a racing caller may have bound the same
    // workspace while we were building. Reuse theirs so one workspace never has
    // two live drivers (kernel.md §3.1) and `capabilities()` stays asked once.
    let entry = guard.entry(key).or_insert_with(|| Arc::clone(&binding));
    Ok(Arc::clone(entry))
}

#[cfg(test)]
#[path = "binding_tests.rs"]
mod tests;
