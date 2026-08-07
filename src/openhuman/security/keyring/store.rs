//! Backend selection and global-state management for the keyring module.
//!
//! Owns the two `OnceLock` singletons:
//! - [`WORKSPACE_DIR`] — the workspace directory provided at startup.
//! - [`BACKEND`] — the selected backend, initialized on first use.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::openhuman::security::keyring::backend::{self, KeyringBackend};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BackendKind {
    Os,
    File,
    EncryptedFile,
}

// ── Global state ─────────────────────────────────────────────────────────────

/// The workspace directory provided by the caller at startup.
///
/// Used by [`FileBackend`] to locate `dev-keychain.json`.  If not set, falls
/// back to the same env-var derivation as the config subsystem.
pub(super) static WORKSPACE_DIR: OnceLock<PathBuf> = OnceLock::new();

/// The selected backend, initialized on first use.
pub(super) static BACKEND: OnceLock<Box<dyn KeyringBackend>> = OnceLock::new();

// ── Initialization ────────────────────────────────────────────────────────────

/// Register the workspace directory for the `file` backend.
///
/// Call this once at application startup (before any keyring operation) so the
/// `FileBackend` knows where to write `dev-keychain.json`.  If not called, the
/// backend derives a default path from env vars.
pub fn init_workspace(workspace_dir: &Path) {
    if WORKSPACE_DIR.set(workspace_dir.to_path_buf()).is_err() {
        // Already initialized — harmless, but log at debug to aid diagnostics.
        log::debug!("[keyring] init_workspace called after initialization; ignored");
    }
}

/// Returns the selected backend, initializing it on first call.
pub(super) fn backend() -> &'static dyn KeyringBackend {
    BACKEND.get_or_init(build_backend).as_ref()
}

pub(super) fn build_backend() -> Box<dyn KeyringBackend> {
    // Priority 1: explicit env var override.
    if let Ok(env_val) = std::env::var("OPENHUMAN_KEYRING_BACKEND") {
        match backend_kind_from_env_value(&env_val) {
            Some(BackendKind::Os) => {
                log::info!("[keyring] backend=os (OPENHUMAN_KEYRING_BACKEND override)");
                return Box::new(backend::OsBackend);
            }
            Some(BackendKind::File) => {
                let path = workspace_dir_for_file_backend();
                log::info!(
                    "[keyring] backend=file dir={} file={}/dev-keychain.json (OPENHUMAN_KEYRING_BACKEND override)",
                    path.display(),
                    path.display()
                );
                return Box::new(backend::FileBackend::new(&path));
            }
            Some(BackendKind::EncryptedFile) => {
                let path = workspace_dir_for_file_backend();
                log::info!(
                    "[keyring] backend=encrypted_file path={} (OPENHUMAN_KEYRING_BACKEND override)",
                    path.display()
                );
                return Box::new(super::encrypted_file_backend::EncryptedFileBackend::new(
                    &path,
                ));
            }
            None => {
                log::warn!(
                    "[keyring] unknown OPENHUMAN_KEYRING_BACKEND={:?}; falling through to defaults",
                    env_val.trim()
                );
            }
        }
    }

    // Priority 2: unit tests → file backend for deterministic isolation.
    if cfg!(test) {
        let path = workspace_dir_for_file_backend();
        log::info!("[keyring] backend=file path={} (cfg(test))", path.display());
        return Box::new(backend::FileBackend::new(&path));
    }

    // Priority 3: staging/production → encrypted file backend (master key in OS keychain).
    // Dev builds → plain file backend (no keychain interaction, avoids codesign prompts).
    let path = workspace_dir_for_file_backend();
    if is_staging_or_production() {
        log::info!("[keyring] backend=encrypted_file path={}", path.display());
        Box::new(super::encrypted_file_backend::EncryptedFileBackend::new(
            &path,
        ))
    } else {
        log::info!(
            "[keyring] backend=file dir={} file={}/dev-keychain.json (dev environment)",
            path.display(),
            path.display()
        );
        Box::new(backend::FileBackend::new(&path))
    }
}

fn is_staging_or_production() -> bool {
    is_staging_or_production_value(std::env::var("OPENHUMAN_APP_ENV").as_deref().ok())
}

pub(super) fn effective_backend_kind() -> BackendKind {
    effective_backend_kind_for(
        std::env::var("OPENHUMAN_APP_ENV").as_deref().ok(),
        std::env::var("OPENHUMAN_KEYRING_BACKEND").as_deref().ok(),
        cfg!(test),
    )
}

fn effective_backend_kind_for(
    app_env: Option<&str>,
    backend_override: Option<&str>,
    cfg_test: bool,
) -> BackendKind {
    if let Some(kind) = backend_override.and_then(backend_kind_from_env_value) {
        return kind;
    }
    if cfg_test {
        return BackendKind::File;
    }
    if is_staging_or_production_value(app_env) {
        BackendKind::EncryptedFile
    } else {
        BackendKind::File
    }
}

fn backend_kind_from_env_value(value: &str) -> Option<BackendKind> {
    match value.trim().to_ascii_lowercase().as_str() {
        "os" => Some(BackendKind::Os),
        "file" => Some(BackendKind::File),
        "encrypted_file" => Some(BackendKind::EncryptedFile),
        _ => None,
    }
}

fn is_staging_or_production_value(app_env: Option<&str>) -> bool {
    matches!(app_env.map(str::trim), Some("staging") | Some("production"))
}

/// Derive the directory for keyring files (`secrets.enc`, `dev-keychain.json`).
///
/// Uses the registered value from [`init_workspace`] if set; otherwise falls
/// back to the same env-var / home-dir logic as the config subsystem.
/// Always resolves to a stable absolute path — never CWD.
///
/// Under `cfg(test)` the environment is deliberately *not* consulted — see the
/// test build of [`fallback_workspace_dir`].
pub fn workspace_dir_for_file_backend() -> PathBuf {
    WORKSPACE_DIR
        .get()
        .cloned()
        .unwrap_or_else(fallback_workspace_dir)
}

/// The directory to use when no caller registered one via [`init_workspace`].
#[cfg(not(test))]
fn fallback_workspace_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("OPENHUMAN_WORKSPACE") {
        if !custom.trim().is_empty() {
            return PathBuf::from(custom);
        }
    }

    let home = dirs::home_dir().unwrap_or_else(|| {
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()))
    });
    match std::env::var("OPENHUMAN_APP_ENV").as_deref() {
        Ok("staging") => home.join(".openhuman-staging"),
        _ => home.join(".openhuman"),
    }
}

/// A scratch directory owned by this test binary, used instead of any real
/// workspace.
///
/// Test code must never be able to reach a developer's live secret store, and
/// before this it could: the default honoured `OPENHUMAN_WORKSPACE`, which the
/// Medulla TUI exports into every process it spawns. A `cargo test` run started
/// from such a shell wrote its per-test secrets straight into the developer's
/// `dev-keychain.json` — thousands of tempdir-scoped junk entries, sharing one
/// read-modify-write file with the live app session sitting beside them.
///
/// A test that wants a specific location still gets one by calling
/// [`init_workspace`] before its first keyring call; only the *fallback* is
/// redirected. The directory is per-process and never cleaned up, because the
/// backend outlives any single test (`BACKEND` is a `OnceLock`) — the OS clears
/// it with the rest of the temp dir.
#[cfg(test)]
fn fallback_workspace_dir() -> PathBuf {
    static TEST_DIR: OnceLock<PathBuf> = OnceLock::new();
    TEST_DIR
        .get_or_init(|| {
            let dir =
                std::env::temp_dir().join(format!("openhuman-test-keyring-{}", std::process::id()));
            if let Err(e) = std::fs::create_dir_all(&dir) {
                log::warn!(
                    "[keyring] could not create the test keyring dir {}: {e}",
                    dir.display()
                );
            }
            dir
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use super::{effective_backend_kind_for, BackendKind};

    #[test]
    fn explicit_file_backend_wins_over_staging_environment() {
        assert_eq!(
            effective_backend_kind_for(Some("staging"), Some("file"), false),
            BackendKind::File
        );
    }

    #[test]
    fn explicit_encrypted_file_backend_wins_in_dev_environment() {
        assert_eq!(
            effective_backend_kind_for(Some("development"), Some("encrypted_file"), false),
            BackendKind::EncryptedFile
        );
    }

    #[test]
    fn staging_defaults_to_encrypted_file_without_override() {
        assert_eq!(
            effective_backend_kind_for(Some(" staging "), None, false),
            BackendKind::EncryptedFile
        );
    }

    #[test]
    fn unknown_backend_override_falls_back_to_environment_default() {
        assert_eq!(
            effective_backend_kind_for(Some("production"), Some("bogus"), false),
            BackendKind::EncryptedFile
        );
    }
}
