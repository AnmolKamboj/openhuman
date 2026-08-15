use serde_json::json;

use crate::openhuman::config::rpc as config_rpc;
use crate::openhuman::skills::init_workflows_dir;
use crate::openhuman::subconscious::heartbeat::engine::HeartbeatEngine;
use std::path::Path;

const BOOTSTRAP_FILES: [(&str, &str); 2] = [
    ("SOUL.md", include_str!("../../agent/prompts/SOUL.md")),
    (
        "IDENTITY.md",
        include_str!("../../agent/prompts/IDENTITY.md"),
    ),
];

/// Bundled default contents for a bootstrap workspace file, or `None` when the
/// name is not one of the files shipped in [`BOOTSTRAP_FILES`].
///
/// This is the single source of truth for both "which workspace files may be
/// edited from the Persona surface" and "what to restore on reset" — the
/// Persona Pack RPCs (`src/openhuman/config/workspace/rpc.rs`) treat membership here
/// as the editable allowlist so a caller can never read or clobber an
/// arbitrary path under the workspace.
pub fn bundled_default_contents(filename: &str) -> Option<&'static str> {
    BOOTSTRAP_FILES
        .iter()
        .find(|(name, _)| *name == filename)
        .map(|(_, contents)| *contents)
}

fn ensure_workspace_file(
    workspace_dir: &Path,
    filename: &str,
    contents: &str,
    force: bool,
) -> Result<&'static str, String> {
    let path = workspace_dir.join(filename);
    if path.exists() && !force {
        return Ok("existing");
    }
    std::fs::write(&path, contents)
        .map_err(|e| format!("failed to write {}: {e}", path.display()))?;
    Ok(if force { "overwritten" } else { "created" })
}

/// If the workspace has no `PROFILE.md` / `MEMORY.md`, seed them from an
/// enabled folder memory source (Anmol's Obsidian vault) so Cursor/XML
/// turns still see identity even before the vector graph is warm.
fn seed_user_brain_files(
    workspace_dir: &Path,
    config: &crate::openhuman::config::Config,
    created: &mut Vec<String>,
    existing: &mut Vec<String>,
) -> Result<(), String> {
    use crate::openhuman::memory::sources::types::SourceKind;

    let memory_md = workspace_dir.join("MEMORY.md");
    if memory_md.is_file() {
        existing.push(memory_md.display().to_string());
    } else {
        let mut body = String::from(
            "# Long-term memory\n\n\
The user's brain is an Obsidian vault ingested as a folder memory source.\n\
Canonical identity note: `People/Anmol.md`.\n\n\
When the user asks what you know about them, their life, family, work, or any stored fact:\n\
1. Use PROFILE.md (already injected) for identity.\n\
2. Call `memory_recall` with a focused query before answering.\n\
3. If recall is thin, call `delegate_retrieve_memory` for a deeper tree walk.\n\
Never claim you have no memory without calling those tools.\n",
        );
        for source in &config.memory_sources {
            if source.enabled && source.kind == SourceKind::Folder {
                if let Some(path) = source.path.as_deref() {
                    body.push_str(&format!("\n- `{}` → `{path}`\n", source.label));
                }
            }
        }
        std::fs::write(&memory_md, body)
            .map_err(|e| format!("failed to write MEMORY.md: {e}"))?;
        created.push(memory_md.display().to_string());
    }

    let profile = workspace_dir.join("PROFILE.md");
    if profile.is_file() {
        existing.push(profile.display().to_string());
        return Ok(());
    }
    for source in &config.memory_sources {
        if !source.enabled || source.kind != SourceKind::Folder {
            continue;
        }
        let Some(root) = source.path.as_deref() else {
            continue;
        };
        for rel in ["People/Anmol.md", "People/Me.md", "PROFILE.md"] {
            let candidate = Path::new(root).join(rel);
            if candidate.is_file() {
                std::fs::copy(&candidate, &profile).map_err(|e| {
                    format!(
                        "failed to seed PROFILE.md from {}: {e}",
                        candidate.display()
                    )
                })?;
                created.push(profile.display().to_string());
                return Ok(());
            }
        }
    }
    Ok(())
}

/// Records `path` in the created / existing list according to what actually
/// happened, given whether it existed before the initialising call.
///
/// A path that still does not exist afterwards is recorded in **neither** list.
/// That case is reachable whenever the code responsible for creating it is
/// compiled out — the `skills` gate's no-op `init_workflows_dir` being the
/// live example — and reporting it as "created" would make `workspace_init`
/// claim work it never did.
fn classify_workspace_entry(
    path: &Path,
    existed_before: bool,
    created: &mut Vec<String>,
    existing: &mut Vec<String>,
) {
    if !path.exists() {
        return;
    }
    if existed_before {
        existing.push(path.display().to_string());
    } else {
        created.push(path.display().to_string());
    }
}

/// Create default dirs, copy bundled prompts, skills README, and heartbeat file.
pub async fn init_workspace(force: bool) -> Result<serde_json::Value, String> {
    let config = config_rpc::load_config_with_timeout().await?;
    let workspace_dir = config.workspace_dir.clone();

    let mut created_dirs = Vec::new();
    let mut existing_dirs = Vec::new();
    for rel in ["memory", "sessions", "state", "cron"] {
        let dir = workspace_dir.join(rel);
        if dir.exists() {
            existing_dirs.push(dir.display().to_string());
        } else {
            std::fs::create_dir_all(&dir)
                .map_err(|e| format!("failed to create directory {}: {e}", dir.display()))?;
            created_dirs.push(dir.display().to_string());
        }
    }

    let mut created_files = Vec::new();
    let mut overwritten_files = Vec::new();
    let mut existing_files = Vec::new();
    for (filename, contents) in BOOTSTRAP_FILES {
        match ensure_workspace_file(&workspace_dir, filename, contents, force)? {
            "created" => created_files.push(workspace_dir.join(filename).display().to_string()),
            "overwritten" => {
                overwritten_files.push(workspace_dir.join(filename).display().to_string())
            }
            _ => existing_files.push(workspace_dir.join(filename).display().to_string()),
        }
    }
    seed_user_brain_files(&workspace_dir, &config, &mut created_files, &mut existing_files)?;

    let skills_readme = workspace_dir.join("skills").join("README.md");
    let had_skills_readme = skills_readme.exists();
    let heartbeat = workspace_dir.join("HEARTBEAT.md");
    let had_heartbeat = heartbeat.exists();
    init_workflows_dir(&workspace_dir)
        .map_err(|e| format!("failed to initialize skills dir: {e}"))?;
    HeartbeatEngine::ensure_heartbeat_file(&workspace_dir)
        .await
        .map_err(|e| format!("failed to initialize HEARTBEAT.md: {e}"))?;

    // Report what the call actually did, not what it was expected to do.
    //
    // These two were previously classified from the pre-check alone, which
    // reports a file as "created" whenever it did not exist beforehand — even
    // if nothing subsequently created it. That is wrong in any build where
    // `skills` is compiled out: `init_workflows_dir` resolves to the stub,
    // which is a deliberate no-op, so `skills/README.md` is never written and
    // every call reports it as freshly created, forever. Re-checking existence
    // afterwards makes all three lists honest: absent stays absent.
    classify_workspace_entry(
        &skills_readme,
        had_skills_readme,
        &mut created_files,
        &mut existing_files,
    );
    classify_workspace_entry(
        &heartbeat,
        had_heartbeat,
        &mut created_files,
        &mut existing_files,
    );

    Ok(json!({
        "result": {
            "workspace_dir": workspace_dir.display().to_string(),
            "config_path": config.config_path.display().to_string(),
            "directories": {
                "created": created_dirs,
                "existing": existing_dirs
            },
            "files": {
                "created": created_files,
                "overwritten": overwritten_files,
                "existing": existing_files
            }
        },
        "logs": [
            "workspace initialization completed"
        ]
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openhuman::config::TEST_ENV_LOCK as ENV_LOCK;
    use tempfile::tempdir;

    /// RAII guard for `OPENHUMAN_WORKSPACE`. Sets the env var on
    /// construction and clears it on drop so a panicking test doesn't
    /// leak the override into sibling tests. Must be constructed while
    /// holding `ENV_LOCK` — mutating process env vars concurrently is
    /// unsafe and the lock serialises every test in this module.
    struct WorkspaceEnvGuard;

    impl WorkspaceEnvGuard {
        fn set(path: &std::path::Path) -> Self {
            // SAFETY: Caller holds `ENV_LOCK`, so no other thread in
            // this process is reading or mutating this env var.
            unsafe {
                std::env::set_var("OPENHUMAN_WORKSPACE", path);
            }
            Self
        }
    }

    impl Drop for WorkspaceEnvGuard {
        fn drop(&mut self) {
            // SAFETY: Same contract as `set()` — `ENV_LOCK` is held for
            // the whole test, so no concurrent env access is possible.
            unsafe {
                std::env::remove_var("OPENHUMAN_WORKSPACE");
            }
        }
    }

    // ── ensure_workspace_file ──────────────────────────────────────

    #[test]
    fn ensure_workspace_file_creates_missing_file() {
        let tmp = tempdir().unwrap();
        let status =
            ensure_workspace_file(tmp.path(), "A.md", "hello", false).expect("should create");
        assert_eq!(status, "created");
        assert_eq!(
            std::fs::read_to_string(tmp.path().join("A.md")).unwrap(),
            "hello"
        );
    }

    #[test]
    fn ensure_workspace_file_leaves_existing_file_untouched_without_force() {
        let tmp = tempdir().unwrap();
        std::fs::write(tmp.path().join("B.md"), "original").unwrap();
        let status = ensure_workspace_file(tmp.path(), "B.md", "new contents", false).expect("ok");
        assert_eq!(status, "existing");
        assert_eq!(
            std::fs::read_to_string(tmp.path().join("B.md")).unwrap(),
            "original",
            "file must not be overwritten when force=false"
        );
    }

    #[test]
    fn ensure_workspace_file_overwrites_when_forced() {
        let tmp = tempdir().unwrap();
        std::fs::write(tmp.path().join("C.md"), "original").unwrap();
        let status = ensure_workspace_file(tmp.path(), "C.md", "new contents", true).expect("ok");
        assert_eq!(status, "overwritten");
        assert_eq!(
            std::fs::read_to_string(tmp.path().join("C.md")).unwrap(),
            "new contents"
        );
    }

    #[test]
    fn ensure_workspace_file_errors_when_directory_missing() {
        let tmp = tempdir().unwrap();
        let missing = tmp.path().join("does/not/exist");
        let err = ensure_workspace_file(&missing, "x.md", "y", false).unwrap_err();
        assert!(
            err.contains("failed to write"),
            "expected write-failure error, got: {err}"
        );
    }

    #[test]
    fn seed_user_brain_files_copies_identity_note_once() {
        use crate::openhuman::memory::sources::types::{MemorySourceEntry, SourceKind};

        let tmp = tempdir().unwrap();
        let vault = tmp.path().join("vault");
        std::fs::create_dir_all(vault.join("People")).unwrap();
        std::fs::write(vault.join("People").join("Anmol.md"), "# Anmol\nfrom vault\n").unwrap();
        let workspace = tmp.path().join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();

        let mut config = crate::openhuman::config::Config::default();
        config.memory_sources = vec![MemorySourceEntry {
            id: "src_test".into(),
            kind: SourceKind::Folder,
            label: "smriti".into(),
            enabled: true,
            toolkit: None,
            connection_id: None,
            path: Some(vault.display().to_string()),
            glob: Some("**/*.md".into()),
            url: None,
            branch: None,
            paths: Vec::new(),
            max_commits: None,
            max_issues: None,
            max_prs: None,
            query: None,
            since_days: None,
            max_items: None,
            selector: None,
            max_tokens_per_sync: None,
            max_cost_per_sync_usd: None,
            sync_depth_days: None,
        }];

        let mut created = Vec::new();
        let mut existing = Vec::new();
        seed_user_brain_files(&workspace, &config, &mut created, &mut existing).unwrap();
        assert!(
            std::fs::read_to_string(workspace.join("PROFILE.md"))
                .unwrap()
                .contains("from vault")
        );
        assert!(
            std::fs::read_to_string(workspace.join("MEMORY.md"))
                .unwrap()
                .contains("memory_recall")
        );

        std::fs::write(workspace.join("PROFILE.md"), "user edited").unwrap();
        created.clear();
        existing.clear();
        seed_user_brain_files(&workspace, &config, &mut created, &mut existing).unwrap();
        assert_eq!(
            std::fs::read_to_string(workspace.join("PROFILE.md")).unwrap(),
            "user edited"
        );
        assert!(created.is_empty());
    }

    #[test]
    fn bootstrap_files_contain_soul_and_identity() {
        // Lock in the contract so `init_workspace` doesn't silently stop
        // shipping a required prompt. These are the canonical prompt
        // files the agent harness expects in every fresh workspace.
        let names: Vec<&str> = BOOTSTRAP_FILES.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"SOUL.md"));
        assert!(names.contains(&"IDENTITY.md"));
        assert_eq!(BOOTSTRAP_FILES.len(), 2);
        // Bundled contents must be non-empty — a packaging regression
        // that empties one would otherwise silently ship a broken agent.
        for (_, contents) in BOOTSTRAP_FILES {
            assert!(!contents.trim().is_empty());
        }
    }

    // ── init_workspace ────────────────────────────────────────────

    #[tokio::test]
    async fn init_workspace_creates_dirs_and_files_in_fresh_workspace() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let tmp = tempdir().unwrap();
        let _env = WorkspaceEnvGuard::set(tmp.path());

        let value = init_workspace(false)
            .await
            .expect("init_workspace on empty temp should succeed");

        let workspace_dir = value["result"]["workspace_dir"]
            .as_str()
            .expect("workspace_dir string");
        let workspace_dir = std::path::PathBuf::from(workspace_dir);
        for rel in ["memory", "sessions", "state", "cron"] {
            assert!(
                workspace_dir.join(rel).is_dir(),
                "expected {rel} directory under {}",
                workspace_dir.display()
            );
        }
        assert!(workspace_dir.join("SOUL.md").is_file());
        assert!(workspace_dir.join("IDENTITY.md").is_file());
        assert!(workspace_dir.join("HEARTBEAT.md").is_file());

        let created: Vec<&str> = value["result"]["files"]["created"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(created.iter().any(|s| s.ends_with("SOUL.md")));
        assert!(created.iter().any(|s| s.ends_with("IDENTITY.md")));

        let logs = value["logs"].as_array().expect("logs array");
        assert!(logs.iter().any(|l| l
            .as_str()
            .unwrap_or("")
            .contains("workspace initialization completed")));
    }

    #[tokio::test]
    async fn init_workspace_reports_existing_entries_on_second_call_without_force() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let tmp = tempdir().unwrap();
        let _env = WorkspaceEnvGuard::set(tmp.path());

        // First call populates the workspace.
        init_workspace(false).await.expect("first init ok");
        // Second call without force should report everything as existing
        // and nothing as created / overwritten.
        let value = init_workspace(false).await.expect("second init ok");

        let created = value["result"]["files"]["created"].as_array().unwrap();
        let overwritten = value["result"]["files"]["overwritten"].as_array().unwrap();
        let existing = value["result"]["files"]["existing"].as_array().unwrap();
        assert!(created.is_empty(), "no files should be re-created");
        assert!(overwritten.is_empty(), "no files should be overwritten");
        assert!(
            existing
                .iter()
                .any(|v| v.as_str().unwrap_or("").ends_with("SOUL.md")),
            "SOUL.md should appear in the existing list"
        );

        let created_dirs = value["result"]["directories"]["created"]
            .as_array()
            .unwrap();
        let existing_dirs = value["result"]["directories"]["existing"]
            .as_array()
            .unwrap();
        assert!(created_dirs.is_empty());
        assert!(!existing_dirs.is_empty());
    }

    #[tokio::test]
    async fn init_workspace_with_force_overwrites_existing_bootstrap_files() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let tmp = tempdir().unwrap();
        let _env = WorkspaceEnvGuard::set(tmp.path());

        let first = init_workspace(false).await.expect("initial init");
        // The config loader may place the workspace at a subpath of the
        // env override (e.g. `{tmp}/workspace`), so discover the real
        // location from the first result rather than assuming it is
        // `tmp.path()` itself.
        let workspace_dir = std::path::PathBuf::from(
            first["result"]["workspace_dir"]
                .as_str()
                .expect("workspace_dir string"),
        );
        let soul = workspace_dir.join("SOUL.md");
        std::fs::write(&soul, "corrupted").unwrap();

        let value = init_workspace(true).await.expect("forced init");

        let overwritten: Vec<&str> = value["result"]["files"]["overwritten"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(overwritten.iter().any(|s| s.ends_with("SOUL.md")));
        // And the on-disk contents must no longer be "corrupted".
        let restored = std::fs::read_to_string(&soul).unwrap();
        assert_ne!(restored, "corrupted");
        assert!(!restored.trim().is_empty());
    }
}
