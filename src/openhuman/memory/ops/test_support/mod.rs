//! Shared test infrastructure for `memory::ops` submodule tests.
//!
//! All `ops` submodules that need one workspace call
//! [`shared_memory_test_workspace`] instead of creating their own
//! `OnceLock<PathBuf>`. Sharing one leaked workspace is what makes concurrent
//! tests agree on a path rather than racing to bind different ones.
//!
//! # It no longer boots an engine
//!
//! It used to also expose `ensure_shared_memory_client`, which called
//! `tinymemory_core::global::init` to hand `ops` tests a real store to write
//! rows into and read back. That is gone with the engine (openhuman#6161); what
//! survives is the part that was never engine work — one agreed-upon temp
//! directory.

use std::path::PathBuf;
use std::sync::OnceLock;

/// Return the process-global workspace used by memory tests without starting a
/// client.
///
/// Binding tests use this narrower helper because constructing a module-backed
/// provider is intentionally synchronous and lazy. The live client starts a
/// Tokio ingestion worker, so initializing it here would make a mere bind
/// depend on whichever test happened to install a reactor first.
pub(crate) fn shared_memory_test_workspace() -> PathBuf {
    static WORKSPACE: OnceLock<PathBuf> = OnceLock::new();
    WORKSPACE
        .get_or_init(|| {
            let tmp = tempfile::TempDir::new().expect("tempdir");
            let path = tmp.path().join("workspace");
            std::fs::create_dir_all(&path).expect("workspace dir");
            std::mem::forget(tmp);
            path
        })
        .clone()
}
