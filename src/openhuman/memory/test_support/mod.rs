//! Test-only driver construction for handlers that read through a family the
//! null driver does not serve.
//!
//! Lives in a `test_support` directory because that is what the memory-guard
//! bypass scanner skips by path (`bypass_allowlist_tests::is_test_path`). The
//! alternative was an allowlist entry, and the allowlist's own rule is that it
//! may shrink but never grow — a test fixture is not the kind of bypass that
//! list exists to track.

use super::binding::install_for_test;
use crate::openhuman::memory::api::provider::MemoryProvider;
use std::sync::Arc;

/// Bind a fake driver that serves every optional family over a whole `Config`'s
/// workspace.
///
/// The shorthand for a test whose handler reads through a family the null
/// driver does not serve — `Chunks`, `Documents`, `Retrieval`.
/// `FixedDiagnostics` cannot serve those: it answers `Maintenance` and
/// delegates the rest to null.
///
/// # Why this is not an engine any more
///
/// It used to build a real `TinycortexProvider` over a temp workspace, and that
/// is what kept `tinycortex` and `tinymemory-core` — 133k lines — on this
/// crate's test critical path long after they left the product build
/// (openhuman#5560). The docstring justified it on the grounds that the
/// alternative was the bus, and a `dlopen`ed module is a process singleton that
/// hangs when a second test loads it.
///
/// That was a false choice: the third option is a driver that is neither the
/// engine nor the bus. `tinymemory-conformance` ships one, it is held to the
/// same contract as TinyCortex by `assert_provider`, and the engine is run
/// against those same assertions upstream — so what a test observes here is
/// contract behaviour rather than one engine's behaviour.
///
/// **What it deliberately will not do is filter, rank, or summarise.** A test
/// that needs those is asserting engine semantics, and upstream owns them; the
/// fake staying simple is what stops such a test from passing here against
/// nothing but the fake.
pub(crate) fn install_memory_driver_for_test(config: &crate::openhuman::config::Config) {
    let provider: Arc<dyn MemoryProvider> =
        Arc::new(tinymemory_conformance::RecordingProvider::new());
    install_for_test(&config.workspace_dir, &config.subsystems.memory, provider);
}
