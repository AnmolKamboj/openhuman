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

/// Bind the in-process TinyCortex driver over a whole `Config`'s workspace.
///
/// The shorthand for a test whose handler reads through a family the null
/// driver does not serve — `Chunks`, `Documents`, `Retrieval` — and which
/// proves itself by writing rows and reading them back. `FixedDiagnostics`
/// cannot serve those: it answers `Maintenance` and delegates the rest to
/// null.
///
/// This is the driver the loadable module wraps, so a test binding it exercises
/// the same engine production reaches over the bus. It is not the bus itself,
/// and cannot be: a `dlopen`'ed module is a process singleton, and two tests
/// loading one in the same process hang rather than fail.
pub(crate) fn install_tinycortex_for_test(config: &crate::openhuman::config::Config) {
    crate::openhuman::memory::host_impls::install_for_tests();
    let client = Arc::new(
        tinymemory_core::store::MemoryClient::from_workspace_dir(config.workspace_dir.clone())
            .expect("open the workspace store"),
    );
    let engine_config = tinymemory_tinycortex::engine::EngineRuntimeConfig {
        workspace_dir: config.workspace_dir.clone(),
        config_path: config.workspace_dir.join("config.toml"),
        memory: config.memory.clone(),
        memory_tree: config.memory_tree.clone(),
        scheduler_gate: config.scheduler_gate.clone(),
        local_ai: config.local_ai.clone(),
        embeddings_provider: config.embeddings_provider.clone(),
        memory_provider: None,
        // Added by tinymemory#100, which moved the periodic sync loops into the
        // module. A test fixture wants the same "no cadence configured" default
        // the module answers for an older host that sends nothing.
        memory_sync_interval_secs: None,
        composio_mode: String::new(),
        composio_entity_id: String::new(),
        default_model: None,
        default_temperature: 0.2,
        output_language: None,
        memory_sources: serde_json::Value::Null,
    };
    let provider: Arc<dyn MemoryProvider> =
        Arc::new(tinymemory_tinycortex::engine::TinycortexProvider::new(
            "tinycortex".to_string(),
            engine_config,
            client,
        ));
    install_for_test(&config.workspace_dir, &config.subsystems.memory, provider);
}
