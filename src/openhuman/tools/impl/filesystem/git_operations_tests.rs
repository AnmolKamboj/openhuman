use super::*;
use crate::openhuman::security::SecurityPolicy;
use tempfile::TempDir;
use tinyagents_harness::tool::ToolExecutionContext;

fn test_tool(dir: &std::path::Path) -> GitOperationsTool {
    let security = Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        ..SecurityPolicy::default()
    });
    GitOperationsTool::new(security, dir.to_path_buf())
}

#[test]
fn sanitize_git_blocks_injection() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    // Should block dangerous arguments
    assert!(tool.sanitize_git_args("--exec=rm -rf /").is_err());
    assert!(tool.sanitize_git_args("$(echo pwned)").is_err());
    assert!(tool.sanitize_git_args("`malicious`").is_err());
    assert!(tool.sanitize_git_args("arg | cat").is_err());
    assert!(tool.sanitize_git_args("arg; rm file").is_err());
}

#[test]
fn sanitize_git_blocks_pager_editor_injection() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    assert!(tool.sanitize_git_args("--pager=less").is_err());
    assert!(tool.sanitize_git_args("--editor=vim").is_err());
}

#[test]
fn sanitize_git_blocks_config_injection() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    // Exact `-c` flag (config injection)
    assert!(tool.sanitize_git_args("-c core.sshCommand=evil").is_err());
    assert!(tool.sanitize_git_args("-c=core.pager=less").is_err());
}

#[test]
fn sanitize_git_blocks_no_verify() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    assert!(tool.sanitize_git_args("--no-verify").is_err());
}

#[test]
fn sanitize_git_blocks_redirect_in_args() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    assert!(tool.sanitize_git_args("file.txt > /tmp/out").is_err());
}

#[test]
fn sanitize_git_cached_not_blocked() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    // --cached must NOT be blocked by the `-c` check
    assert!(tool.sanitize_git_args("--cached").is_ok());
    // Other safe flags starting with -c prefix
    assert!(tool.sanitize_git_args("-cached").is_ok());
}

#[test]
fn sanitize_git_allows_safe() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    // Should allow safe arguments
    assert!(tool.sanitize_git_args("main").is_ok());
    assert!(tool.sanitize_git_args("feature/test-branch").is_ok());
    assert!(tool.sanitize_git_args("--cached").is_ok());
    assert!(tool.sanitize_git_args("src/main.rs").is_ok());
    assert!(tool.sanitize_git_args(".").is_ok());
}

/// Parity guard for the worktree-isolation action-dir override (#3376,
/// #4249 08.5). A worktree-isolated worker's git operation MUST resolve its CWD
/// from the carried `WorkspaceDescriptor` (the isolated worktree), never the
/// tool's configured `action_dir`. WITHOUT a descriptor it falls back to
/// `action_dir` — the non-isolated path, byte-identical to before. This encodes
/// the behaviour the deleted `worktree_context.rs` task-local used to provide.
#[test]
fn git_resolves_cwd_from_workspace_descriptor() {
    use tinyagents_harness::context::{RunConfig, RunContext};
    use tinyagents_harness::workspace::WorkspaceDescriptor;

    let action_tmp = TempDir::new().unwrap();
    let worktree_tmp = TempDir::new().unwrap();
    let tool = test_tool(action_tmp.path());

    // WITH a descriptor → the worktree root wins.
    let ws =
        WorkspaceDescriptor::new(worktree_tmp.path().to_path_buf()).with_policy_id("test-worktree");
    let ctx: RunContext = RunContext::new(RunConfig::new("test-run"), ()).with_workspace(ws);
    let tool_ctx = ToolExecutionContext::from_run_context(&ctx);
    assert_eq!(
        tool.effective_action_dir_for_context(Some(&tool_ctx)),
        worktree_tmp.path().to_path_buf(),
        "git with a WorkspaceDescriptor must resolve CWD to the worktree root"
    );

    // WITHOUT a descriptor → configured action_dir (non-isolated parity).
    assert_eq!(
        tool.effective_action_dir_for_context(None),
        action_tmp.path().to_path_buf(),
        "git with no descriptor must fall back to the configured action_dir"
    );
}

#[test]
fn requires_write_detection() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    assert!(tool.requires_write_access("commit"));
    assert!(tool.requires_write_access("add"));
    assert!(tool.requires_write_access("checkout"));

    assert!(!tool.requires_write_access("status"));
    assert!(!tool.requires_write_access("diff"));
    assert!(!tool.requires_write_access("log"));
}

#[test]
fn branch_is_not_write_gated() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    // Branch listing is read-only; it must not require write access
    assert!(!tool.requires_write_access("branch"));
    assert!(tool.is_read_only("branch"));
}

#[test]
fn is_read_only_detection() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    assert!(tool.is_read_only("status"));
    assert!(tool.is_read_only("diff"));
    assert!(tool.is_read_only("log"));
    assert!(tool.is_read_only("branch"));

    assert!(!tool.is_read_only("commit"));
    assert!(!tool.is_read_only("add"));
}

#[tokio::test]
async fn blocks_readonly_mode_for_write_ops() {
    let tmp = TempDir::new().unwrap();
    // Initialize a git repository
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let security = Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::ReadOnly,
        ..SecurityPolicy::default()
    });
    let tool = GitOperationsTool::new(security, tmp.path().to_path_buf());

    let result = tool
        .execute(json!({"operation": "commit", "message": "test"}))
        .await
        .unwrap();
    assert!(result.is_error);
    // can_act() returns false for ReadOnly, so we get the "higher autonomy level" message
    assert!(result.output().contains("higher autonomy"));
}

#[tokio::test]
async fn allows_branch_listing_in_readonly_mode() {
    let tmp = TempDir::new().unwrap();
    // Initialize a git repository so the command can succeed
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let security = Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::ReadOnly,
        ..SecurityPolicy::default()
    });
    let tool = GitOperationsTool::new(security, tmp.path().to_path_buf());

    let result = tool.execute(json!({"operation": "branch"})).await.unwrap();
    // Branch listing must not be blocked by read-only autonomy
    let error_msg = result.output();
    assert!(
        !error_msg.contains("read-only") && !error_msg.contains("higher autonomy"),
        "branch listing should not be blocked in read-only mode, got: {error_msg}"
    );
}

#[tokio::test]
async fn allows_readonly_ops_in_readonly_mode() {
    let tmp = TempDir::new().unwrap();
    let security = Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::ReadOnly,
        ..SecurityPolicy::default()
    });
    let tool = GitOperationsTool::new(security, tmp.path().to_path_buf());

    // This will fail because there's no git repo, but it shouldn't be blocked by autonomy
    let result = tool.execute(json!({"operation": "status"})).await.unwrap();
    // The error should be about git (not about autonomy/read-only mode)
    assert!(result.is_error, "Expected failure due to missing git repo");
    let error_msg = result.output();
    assert!(
        !error_msg.is_empty(),
        "Expected a git-related error message"
    );
    assert!(
        !error_msg.contains("read-only") && !error_msg.contains("autonomy"),
        "Error should be about git, not about autonomy restrictions: {error_msg}"
    );
}

#[tokio::test]
async fn rejects_missing_operation() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    let result = tool.execute(json!({})).await.unwrap();
    assert!(result.is_error);
    assert!(result.output().contains("Missing 'operation'"));
}

#[tokio::test]
async fn rejects_unknown_operation() {
    let tmp = TempDir::new().unwrap();
    // Initialize a git repository
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let tool = test_tool(tmp.path());

    let result = tool.execute(json!({"operation": "push"})).await.unwrap();
    assert!(result.is_error);
    assert!(result.output().contains("Unknown operation"));
}

#[test]
fn truncates_multibyte_commit_message_without_panicking() {
    let long = "🦀".repeat(2500);
    let truncated = GitOperationsTool::truncate_commit_message(&long);

    assert_eq!(truncated.chars().count(), 2000);
}

// ── truncate_commit_message: short messages pass through unchanged ─────────

#[test]
fn truncate_short_message_unchanged() {
    let msg = "Fix the bug";
    assert_eq!(GitOperationsTool::truncate_commit_message(msg), msg);
}

#[test]
fn truncate_exact_2000_chars_unchanged() {
    let msg = "a".repeat(2000);
    let result = GitOperationsTool::truncate_commit_message(&msg);
    assert_eq!(result.chars().count(), 2000);
    assert!(!result.ends_with("..."));
}

#[test]
fn truncate_2001_chars_adds_ellipsis() {
    let msg = "a".repeat(2001);
    let result = GitOperationsTool::truncate_commit_message(&msg);
    assert!(result.ends_with("..."));
    assert_eq!(result.chars().count(), 2000);
}

// ── sanitize_git_args: allow leading dash that is not -c ─────────────────

#[test]
fn sanitize_git_allows_other_flags() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());
    assert!(tool.sanitize_git_args("--follow").is_ok());
    assert!(tool.sanitize_git_args("-p").is_ok());
    assert!(tool.sanitize_git_args("-n5").is_ok());
}

// ── requires_write_access completeness ────────────────────────────────────

#[test]
fn requires_write_access_covers_all_write_ops() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());
    for op in ["commit", "add", "checkout", "stash", "reset", "revert"] {
        assert!(
            tool.requires_write_access(op),
            "'{op}' should require write access"
        );
    }
}

// ── schema validation ─────────────────────────────────────────────────────

#[test]
fn schema_has_required_operation() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());
    let schema = tool.parameters_schema();
    let required = schema["required"].as_array().unwrap();
    assert!(
        required.contains(&serde_json::json!("operation")),
        "schema required should include 'operation'"
    );
}

#[test]
fn schema_enumerates_operations() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());
    let schema = tool.parameters_schema();
    let ops = schema["properties"]["operation"]["enum"]
        .as_array()
        .unwrap();
    let op_names: Vec<&str> = ops.iter().map(|v| v.as_str().unwrap()).collect();
    for expected in [
        "status", "diff", "log", "branch", "commit", "add", "checkout", "stash",
    ] {
        assert!(
            op_names.contains(&expected),
            "schema should include '{expected}'"
        );
    }
}

// ── git_operations tool name / description ────────────────────────────────

#[test]
fn tool_name_and_description() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());
    assert_eq!(tool.name(), "git_operations");
    assert!(!tool.description().is_empty());
    assert!(tool.description().contains("Git"));
}

// ── not_in_git_repo returns error (covers the git-repo check) ─────────────

#[tokio::test]
async fn not_in_git_repo_returns_error() {
    let tmp = TempDir::new().unwrap();
    // Do NOT init a git repo
    let tool = test_tool(tmp.path());
    let result = tool.execute(json!({"operation": "status"})).await.unwrap();
    assert!(result.is_error);
    assert!(result.output().contains("Not in a git repository"));
}

/// Suppress the developer's own system/global git config on a raw
/// `std::process::Command`, so a machine-local `init.templateDir` or similar
/// cannot write extra keys into a test repository's `.git/config` and make
/// these tests depend on ambient environment. Mirrors [`hardened_git`]'s two
/// env vars; the production code under test applies its own suppression when
/// it later reads this same config, so this only affects setup.
fn hermetic(cmd: &mut std::process::Command) -> &mut std::process::Command {
    cmd.env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", NULL_CONFIG_PATH)
}

/// Initialise a git repo at `path` and fail the test if `git init`
/// itself didn't succeed (so we don't misread later assertion failures
/// as product bugs when the real problem is a missing/broken git).
fn init_git_repo(path: &std::path::Path) {
    let output = hermetic(
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(path),
    )
    .output()
    .expect("failed to spawn `git init`");
    assert!(
        output.status.success(),
        "`git init` failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Extract the error text from a Result<ToolResult> — whether the
/// failure came through `Err(anyhow::Error)` or `Ok(ToolResult::error)`.
fn error_text(result: &anyhow::Result<ToolResult>) -> String {
    match result {
        Ok(r) => {
            assert!(r.is_error, "expected a tool-error ToolResult");
            r.output().to_string()
        }
        Err(e) => e.to_string(),
    }
}

// ── stash: unknown action returns error ────────────────────────────────────

#[tokio::test]
async fn stash_unknown_action_returns_error() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());

    let tool = test_tool(tmp.path());
    let result = tool
        .execute(json!({"operation": "stash", "action": "squash"}))
        .await;
    let msg = error_text(&result);
    assert!(
        msg.contains("Unknown stash action"),
        "expected 'Unknown stash action' in error, got: {msg}"
    );
}

// ── checkout: dangerous characters ────────────────────────────────────────

#[tokio::test]
async fn checkout_rejects_dangerous_branch_names() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());

    let tool = test_tool(tmp.path());

    for dangerous in ["main@{1}", "HEAD^", "v1~2"] {
        let result = tool
            .execute(json!({"operation": "checkout", "branch": dangerous}))
            .await;
        let msg = error_text(&result);
        assert!(
            msg.contains("invalid characters") || msg.contains("Invalid branch"),
            "expected a dangerous-branch rejection for '{dangerous}', got: {msg}"
        );
    }
}

// ── commit: missing message ────────────────────────────────────────────────

#[tokio::test]
async fn commit_missing_message_returns_error() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());

    let tool = test_tool(tmp.path());
    let result = tool.execute(json!({"operation": "commit"})).await;
    let msg = error_text(&result);
    assert!(
        msg.contains("Missing 'message' parameter"),
        "expected missing-message error, got: {msg}"
    );
}

// ── add: missing paths ─────────────────────────────────────────────────────

#[tokio::test]
async fn add_missing_paths_returns_error() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());

    let tool = test_tool(tmp.path());
    let result = tool.execute(json!({"operation": "add"})).await;
    let msg = error_text(&result);
    assert!(
        msg.contains("Missing 'paths' parameter"),
        "expected missing-paths error, got: {msg}"
    );
}

// ── run_git_command_in: repository config hardening (issue #5494) ─────────
//
// `run_git_command_in` backs every operation this tool exposes, including
// `status`, which — like `read_workspace_state`'s `run_git` before #5493 —
// invokes `core.fsmonitor` from the repository's own `.git/config`. That file
// lives in `action_dir`, which `file_write` and `git_operations` itself
// (`add`, `commit`, `checkout`) can write to, so it is attacker-controlled
// input, not trusted configuration.

/// Write a `core.fsmonitor` hook into `dir`'s repository config that creates a
/// marker file when git runs it, and return the marker's path.
///
/// Runs the hook once up front and asserts the marker appears, then removes
/// it — so a later absent marker means the hardening refused the hook, not
/// that the hook itself was silently broken (e.g. by `{:?}`-escaping a path
/// the shell would quote differently than Rust's `Debug` does).
#[cfg(unix)]
fn plant_fsmonitor_hook(dir: &std::path::Path) -> std::path::PathBuf {
    let hook = dir.join("hook.sh");
    let marker = dir.join("COMMAND_RAN");
    std::fs::write(
        &hook,
        format!("#!/bin/sh\ntouch {:?}\nexit 1\n", marker.to_string_lossy()),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();

    std::process::Command::new(&hook).status().unwrap();
    assert!(marker.exists(), "the planted hook does not run at all");
    std::fs::remove_file(&marker).unwrap();

    // Written with `git config` rather than by appending to the file:
    // appending only lands in `[core]` while `[core]` happens to be the last
    // section, which is true of a fresh `git init` and is not a property
    // worth depending on.
    let ok = hermetic(
        std::process::Command::new("git")
            .args(["config", "core.fsmonitor"])
            .arg(&hook)
            .current_dir(dir),
    )
    .status()
    .unwrap()
    .success();
    assert!(ok, "failed to plant the hook in the repository config");
    marker
}

/// Set a repository config key with `git config`, asserting it took.
fn set_config(dir: &std::path::Path, key: &str, value: &str) {
    let ok = hermetic(
        std::process::Command::new("git")
            .args(["config", key, value])
            .current_dir(dir),
    )
    .status()
    .unwrap()
    .success();
    assert!(ok, "failed to set {key} in the test workspace");
}

/// Issue #5494. `git status` executes the command named by the workspace's
/// own repository config unless `run_git_command_in` refuses to run under it.
/// Revert the hardening and this test fails by finding the marker — verified,
/// not assumed.
#[cfg(unix)]
#[tokio::test]
async fn repository_config_naming_a_command_does_not_get_to_run_it() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    let marker = plant_fsmonitor_hook(tmp.path());

    let tool = test_tool(tmp.path());
    let result = tool.execute(json!({"operation": "status"})).await;
    let msg = error_text(&result);

    assert!(
        !marker.exists(),
        "`git status` executed the command named by the workspace's own \
         repository config — this tool is a code-execution primitive"
    );
    assert!(
        msg.contains("fsmonitor"),
        "the refusal should name the key that caused it, got: {msg}"
    );
}

/// The allowlist has to leave an ordinary repository working, or the fix is
/// just a different way of breaking the tool.
#[tokio::test]
async fn an_ordinary_repository_still_reports_status() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    std::fs::write(tmp.path().join("tracked.txt"), "hi").unwrap();

    let tool = test_tool(tmp.path());
    let result = tool.execute(json!({"operation": "status"})).await.unwrap();

    assert!(!result.is_error, "got: {}", result.output());
    assert!(
        result.output().contains("tracked.txt"),
        "a plain `git init` workspace must still report status, got: {}",
        result.output()
    );
}

/// A first-draft allowlist that refused any repository carrying an ordinary
/// setting like `core.autocrlf` would report nothing useful for a large class
/// of real workspaces.
#[tokio::test]
async fn an_inert_setting_an_ordinary_repository_carries_is_allowed() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    set_config(tmp.path(), "core.autocrlf", "input");
    set_config(tmp.path(), "gc.auto", "0");
    set_config(tmp.path(), "remote.origin.prune", "true");

    let tool = test_tool(tmp.path());
    let result = tool.execute(json!({"operation": "status"})).await.unwrap();

    assert!(
        !result.is_error && !result.output().contains("not on the allowlist"),
        "an ordinary repository must still report status, got: {}",
        result.output()
    );
}

/// The other half of the same question, and the answer is the opposite one.
/// `filter.lfs.clean` names a program, so an LFS working copy is refused —
/// fail-closed, and intended rather than an oversight.
#[tokio::test]
async fn an_lfs_clone_is_refused_because_its_filter_names_a_program() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    // What `git lfs install` writes. `required` is inert and allowed; the
    // three programs are not.
    set_config(tmp.path(), "filter.lfs.required", "true");
    set_config(tmp.path(), "filter.lfs.clean", "git-lfs clean -- %f");

    let tool = test_tool(tmp.path());
    let result = tool.execute(json!({"operation": "status"})).await;
    let msg = error_text(&result);

    assert!(
        msg.contains("filter.lfs.clean"),
        "the refusal must name the key that caused it, got: {msg}"
    );
}

/// `credential.helper` reads like a preference and is command-valued: a value
/// beginning `!` is run as a shell command. It must be refused however inert
/// it reads.
#[tokio::test]
async fn credential_helper_is_refused_despite_looking_like_a_preference() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    set_config(tmp.path(), "credential.helper", "!echo pwned");

    let tool = test_tool(tmp.path());
    let result = tool.execute(json!({"operation": "status"})).await;
    let msg = error_text(&result);

    assert!(
        msg.contains("credential.helper"),
        "a command-valued key must be refused however inert it reads, got: {msg}"
    );
}

/// The refusal must hold for a write operation too, not just `status` — the
/// same repository config runs under `commit`/`add`/`checkout`/`stash`.
#[tokio::test]
async fn refusal_also_covers_write_operations() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    set_config(tmp.path(), "credential.helper", "!echo pwned");

    let tool = test_tool(tmp.path());
    let result = tool
        .execute(json!({"operation": "commit", "message": "test"}))
        .await
        .unwrap();

    assert!(
        result.is_error && result.output().contains("credential.helper"),
        "write operations must be refused under untrusted repo config too, got: {}",
        result.output()
    );
}

/// `core.worktree` redirects the working-tree root every write operation
/// here (`checkout`, `add`, `commit`, `stash`) targets. Left on the
/// allowlist, a repository config could point that root outside
/// `action_dir` and turn a supposedly sandboxed write into one against an
/// arbitrary directory. Nothing this tool does needs the key — worktree
/// isolation goes through `WorkspaceDescriptor` instead.
#[tokio::test]
async fn core_worktree_is_refused_because_it_can_redirect_writes_outside_the_sandbox() {
    let tmp = TempDir::new().unwrap();
    let elsewhere = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    set_config(
        tmp.path(),
        "core.worktree",
        &elsewhere.path().to_string_lossy(),
    );

    let tool = test_tool(tmp.path());
    let result = tool.execute(json!({"operation": "status"})).await;
    let msg = error_text(&result);

    assert!(
        msg.contains("core.worktree"),
        "the refusal must name the key that caused it, got: {msg}"
    );
}

/// `extensions.worktreeConfig` is itself allowlisted as an ordinary setting,
/// but turning it on makes git additionally read `config.worktree` — a
/// second file `--local` alone does not see. A `core.hooksPath` set there is
/// invisible to a `--local`-only inspection and would still run on the next
/// `commit`. The inspection step must read the same merged view git does.
#[tokio::test]
async fn a_hookspath_hidden_in_worktree_scoped_config_is_still_refused() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    set_config(tmp.path(), "extensions.worktreeConfig", "true");
    // What `git config --worktree core.hooksPath <dir>` writes; `set_config`
    // only reaches `--local`, so this key is planted directly the same way
    // the production inspection step reads it — via a real `git config
    // --worktree` invocation — to prove the bypass is closed, not just that
    // `set_config` happens to skip it.
    let ok = hermetic(
        std::process::Command::new("git")
            .args(["config", "--worktree", "core.hooksPath"])
            .arg(tmp.path())
            .current_dir(tmp.path()),
    )
    .status()
    .unwrap()
    .success();
    assert!(ok, "failed to set core.hooksPath in worktree-scoped config");

    let tool = test_tool(tmp.path());
    let result = tool.execute(json!({"operation": "status"})).await;
    let msg = error_text(&result);

    assert!(
        msg.contains("core.hookspath"),
        "a hookspath hidden in worktree-scoped config must still be refused, got: {msg}"
    );
}

/// The allowlist inspection and the real command are two separate `git`
/// invocations, so a config change landing in the gap between them would be
/// invisible to the first and still reach the second. This test calls
/// `hardened_git` directly — skipping `first_disallowed_repo_config_key`
/// entirely, standing in for that gap — to prove the second invocation does
/// not depend on the first having caught anything: `core.hooksPath` is
/// neutralised at the point of execution regardless of what any inspection
/// saw or missed.
#[cfg(unix)]
#[tokio::test]
async fn hardened_git_neutralises_hookspath_even_if_the_allowlist_check_never_ran() {
    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    set_config(tmp.path(), "user.email", "test@example.com");
    set_config(tmp.path(), "user.name", "Test");

    let hooks_dir = tmp.path().join("evil-hooks");
    std::fs::create_dir(&hooks_dir).unwrap();
    let marker = tmp.path().join("HOOK_RAN");
    std::fs::write(
        hooks_dir.join("pre-commit"),
        format!("#!/bin/sh\ntouch {:?}\nexit 0\n", marker.to_string_lossy()),
    )
    .unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            hooks_dir.join("pre-commit"),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
    }
    set_config(tmp.path(), "core.hooksPath", &hooks_dir.to_string_lossy());

    std::fs::write(tmp.path().join("f.txt"), "hi").unwrap();
    let staged = hermetic(
        std::process::Command::new("git")
            .args(["add", "f.txt"])
            .current_dir(tmp.path()),
    )
    .status()
    .unwrap()
    .success();
    assert!(staged, "failed to stage the test file");

    let output = super::hardened_git(tmp.path())
        .args(["commit", "-m", "msg"])
        .output()
        .await
        .unwrap();

    assert!(
        output.status.success(),
        "commit should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !marker.exists(),
        "hardened_git ran the repository-configured pre-commit hook — the \
         override that is supposed to hold even without the allowlist check \
         did not"
    );
}

/// `commit.gpgsign` is on `ALLOWED_REPO_CONFIG` as an ordinary boolean, but
/// left un-neutralised it would let a repository force every commit through
/// this tool to be signed. `output.status.success()` alone does not prove
/// that: a repository that also configures a *working* `gpg.program` would
/// make a signed commit succeed too, so this plants a fake one that always
/// signs successfully and then inspects the commit object itself for a
/// `gpgsig` header — the only assertion that actually distinguishes "signing
/// was skipped" from "signing was attempted and happened to work".
#[cfg(unix)]
#[tokio::test]
async fn hardened_git_neutralises_forced_commit_signing() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    set_config(tmp.path(), "user.email", "test@example.com");
    set_config(tmp.path(), "user.name", "Test");
    set_config(tmp.path(), "commit.gpgsign", "true");

    // A `gpg.program` that always "signs" successfully, so a regression here
    // fails by finding a signature, not by the commit merely erroring out —
    // the same distinction CodeRabbit's review raised.
    let fake_gpg = tmp.path().join("fake-gpg.sh");
    std::fs::write(
        &fake_gpg,
        "#!/bin/sh\n\
         printf '%s\\n' '[GNUPG:] BEGIN_SIGNING H10' >&2\n\
         cat >/dev/null\n\
         printf -- '-----BEGIN PGP SIGNATURE-----\\n\\nZmFrZQ==\\n-----END PGP SIGNATURE-----\\n'\n\
         printf '%s\\n' '[GNUPG:] SIG_CREATED D 1 10 00 0 0123456789ABCDEF' >&2\n",
    )
    .unwrap();
    std::fs::set_permissions(&fake_gpg, std::fs::Permissions::from_mode(0o755)).unwrap();
    set_config(tmp.path(), "gpg.program", &fake_gpg.to_string_lossy());

    std::fs::write(tmp.path().join("f.txt"), "hi").unwrap();
    let staged = hermetic(
        std::process::Command::new("git")
            .args(["add", "f.txt"])
            .current_dir(tmp.path()),
    )
    .status()
    .unwrap()
    .success();
    assert!(staged, "failed to stage the test file");

    let output = super::hardened_git(tmp.path())
        .args(["commit", "-m", "msg"])
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "commit failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let commit_object = super::hardened_git(tmp.path())
        .args(["cat-file", "-p", "HEAD"])
        .output()
        .await
        .unwrap();
    assert!(commit_object.status.success());
    let commit_object = String::from_utf8_lossy(&commit_object.stdout);
    assert!(
        !commit_object.lines().any(|l| l.starts_with("gpgsig ")),
        "commit.gpgsign=true must not be honoured, but HEAD carries a \
         signature: {commit_object}"
    );
}

/// The config-inspection step must fail closed: if `git config --list
/// --local` cannot be read, that is not the same as "nothing to distrust",
/// and running the real command anyway would skip the check entirely.
#[cfg(unix)]
#[tokio::test]
async fn unreadable_repo_config_fails_closed_rather_than_running_anyway() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = TempDir::new().unwrap();
    init_git_repo(tmp.path());
    let config_path = tmp.path().join(".git").join("config");
    std::fs::set_permissions(&config_path, std::fs::Permissions::from_mode(0o000)).unwrap();

    // Root (and some CI containers run as root) ignores this permission bit
    // entirely, which would make the assertion below meaningless rather than
    // wrong. Detect that up front instead of failing on an unrelated cause.
    let permission_enforced = std::fs::File::open(&config_path).is_err();

    let result = if permission_enforced {
        let tool = test_tool(tmp.path());
        Some(tool.execute(json!({"operation": "status"})).await)
    } else {
        None
    };

    // Restore permissions before the TempDir is dropped, so cleanup doesn't
    // fail on an unreadable file.
    std::fs::set_permissions(&config_path, std::fs::Permissions::from_mode(0o644)).unwrap();

    let Some(result) = result else {
        eprintln!(
            "skipping: file permissions are not enforced against this process (running as root?)"
        );
        return;
    };
    let msg = error_text(&result);

    assert!(
        msg.contains("could not inspect its repository config"),
        "an unreadable repo config must refuse, not silently proceed, got: {msg}"
    );
}

#[test]
fn a_subsection_is_elided_so_one_entry_covers_every_remote() {
    assert_eq!(normalise_config_key("remote.origin.url"), "remote.url");
    assert_eq!(normalise_config_key("remote.a.b.c.url"), "remote.url");
    assert_eq!(normalise_config_key("core.fileMode"), "core.filemode");
    assert_eq!(normalise_config_key("core.fsmonitor"), "core.fsmonitor");
    // The subsection itself contains dots; the first and last components
    // remain the reliable ones.
    assert_eq!(
        normalise_config_key("includeIf.gitdir:~/x.y/.path"),
        "includeif.path"
    );
    // A key with no dot at all is returned unchanged rather than panicking.
    assert_eq!(normalise_config_key("bare"), "bare");
}
