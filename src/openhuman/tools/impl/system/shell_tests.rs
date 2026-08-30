use super::*;
use crate::openhuman::agent::host_runtime::{NativeRuntime, RuntimeAdapter};
use crate::openhuman::security::{AutonomyLevel, CommandClass, SecurityPolicy};

fn test_security(autonomy: AutonomyLevel) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy {
        autonomy,
        workspace_dir: std::env::temp_dir(),
        action_dir: std::env::temp_dir(),
        ..SecurityPolicy::default()
    })
}

fn test_runtime() -> Arc<dyn RuntimeAdapter> {
    Arc::new(NativeRuntime::new())
}

fn test_audit() -> Arc<AuditLogger> {
    AuditLogger::disabled()
}

fn audit_with_tempdir() -> (Arc<AuditLogger>, tempfile::TempDir) {
    use crate::openhuman::config::AuditConfig;
    let tmp = tempfile::tempdir().expect("create tempdir");
    let logger = AuditLogger::new(
        AuditConfig {
            enabled: true,
            log_path: "audit.log".into(),
            max_size_mb: 10,
        },
        tmp.path().to_path_buf(),
    )
    .expect("create audit logger");
    (Arc::new(logger), tmp)
}

#[cfg(not(windows))]
#[tokio::test]
async fn shell_emits_audit_line_on_success() {
    use crate::openhuman::security::AuditEvent;
    let (audit, tmp) = audit_with_tempdir();
    let tool = ShellTool::new(
        test_security(AutonomyLevel::Supervised),
        test_runtime(),
        audit,
    );
    let _ = tool
        .execute(json!({"command": "echo hello"}))
        .await
        .unwrap();
    let log =
        std::fs::read_to_string(tmp.path().join("audit.log")).expect("audit log file should exist");
    assert!(!log.is_empty(), "audit log should not be empty");
    let parsed: AuditEvent = serde_json::from_str(log.trim()).expect("audit event JSON parses");
    let action = parsed.action.expect("action present");
    assert_eq!(action.command, Some("echo hello".to_string()));
    assert!(action.allowed, "allowed command should set allowed=true");
    let result = parsed.result.expect("result present");
    assert!(result.success, "echo hello should succeed");
    let actor = parsed.actor.expect("actor present");
    assert_eq!(actor.channel, "tool:shell");
}

#[tokio::test]
async fn shell_emits_audit_line_on_denial() {
    use crate::openhuman::security::AuditEvent;
    let (audit, tmp) = audit_with_tempdir();
    let tool = ShellTool::new(
        test_security(AutonomyLevel::ReadOnly),
        test_runtime(),
        audit,
    );
    // A write command in read-only mode is denied before execution.
    let _ = tool
        .execute(json!({"command": "touch denied_file"}))
        .await
        .unwrap();
    let log =
        std::fs::read_to_string(tmp.path().join("audit.log")).expect("audit log file should exist");
    let parsed: AuditEvent = serde_json::from_str(log.trim()).expect("audit event JSON parses");
    let action = parsed.action.expect("action present");
    assert!(
        !action.allowed,
        "denied command should set allowed=false on the audit event"
    );
}

#[test]
fn shell_tool_name() {
    let tool = ShellTool::new(
        test_security(AutonomyLevel::Supervised),
        test_runtime(),
        test_audit(),
    );
    assert_eq!(tool.name(), "shell");
}

#[test]
fn shell_tool_description() {
    let tool = ShellTool::new(
        test_security(AutonomyLevel::Supervised),
        test_runtime(),
        test_audit(),
    );
    assert!(!tool.description().is_empty());
}

#[test]
fn shell_tool_schema_has_command() {
    let tool = ShellTool::new(
        test_security(AutonomyLevel::Supervised),
        test_runtime(),
        test_audit(),
    );
    let schema = tool.parameters_schema();
    assert!(schema["properties"]["command"].is_object());
    assert!(schema["required"]
        .as_array()
        .unwrap()
        .contains(&json!("command")));
    // The self-asserted `approved` param was removed — approval now happens
    // at the harness ApprovalGate, not via a model-set flag.
    assert!(schema["properties"]["approved"].is_null());
}

#[cfg(not(windows))]
#[tokio::test]
async fn shell_executes_allowed_command() {
    let tool = ShellTool::new(
        test_security(AutonomyLevel::Supervised),
        test_runtime(),
        test_audit(),
    );
    let result = tool
        .execute(json!({"command": "echo hello"}))
        .await
        .unwrap();
    assert!(!result.is_error, "{}", result.output());
    assert!(result.output().trim().contains("hello"));
    assert!(!result.is_error);
}

#[tokio::test]
async fn shell_destructive_command_is_gated_not_run_inline() {
    // `rm -rf /` is Destructive → it must route through the human approval
    // gate (external_effect), never auto-run. Assert the classification
    // here rather than executing it.
    let security = test_security(AutonomyLevel::Supervised);
    let tool = ShellTool::new(security.clone(), test_runtime(), test_audit());
    assert_eq!(
        security.classify_command("rm -rf /"),
        CommandClass::Destructive
    );
    assert!(tool.external_effect_with_args(&json!({"command": "rm -rf /"})));
}

/// End-to-end regression guard for #3238.
///
/// PR #3074 split `Config.action_dir` (the agent's read/write root)
/// from `Config.workspace_dir` (internal product state). `ShellTool`
/// is contractually obligated to spawn its child process with
/// `current_dir = security.action_dir` so `pwd` inside the shell
/// reports the action sandbox path, never `workspace_dir` and never
/// the cargo-test caller's CWD.
///
/// This test constructs a `SecurityPolicy` whose `action_dir` is a
/// fresh tempdir (distinct from `workspace_dir` and from `cwd`),
/// runs `pwd`, and asserts the captured stdout canonicalises to the
/// same path as `action_dir`. If `ShellTool::run_with_security`
/// stops passing `&security.action_dir` to `build_shell_command`
/// (or `build_shell_command` stops calling `current_dir`), this
/// test fails before the regression ships.
#[cfg(not(windows))]
#[tokio::test]
async fn shell_pwd_returns_action_dir_not_workspace_dir() {
    let action_tmp = tempfile::tempdir().expect("create action tempdir");
    let workspace_tmp = tempfile::tempdir().expect("create workspace tempdir");
    let security = Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        workspace_dir: workspace_tmp.path().to_path_buf(),
        action_dir: action_tmp.path().to_path_buf(),
        ..SecurityPolicy::default()
    });
    let tool = ShellTool::new(security.clone(), test_runtime(), test_audit());

    let result = tool
        .execute(json!({"command": "pwd"}))
        .await
        .expect("pwd should execute without harness error");
    assert!(
        !result.is_error,
        "pwd unexpectedly errored: {}",
        result.output()
    );

    // Canonicalise both sides — on macOS `/tmp` is a symlink to
    // `/private/tmp`, so the raw strings won't match even when the
    // paths are the same.
    let reported = std::path::PathBuf::from(result.output().trim());
    let actual = reported.canonicalize().unwrap_or_else(|_| reported.clone());
    let expected = security
        .action_dir
        .canonicalize()
        .unwrap_or_else(|_| security.action_dir.clone());
    let workspace_canon = security
        .workspace_dir
        .canonicalize()
        .unwrap_or_else(|_| security.workspace_dir.clone());

    assert_eq!(
        actual,
        expected,
        "pwd must report `action_dir`. got `{}`, expected `{}`. \
         If this fails, `ShellTool::run_with_security` likely stopped \
         passing `&security.action_dir` to `runtime.build_shell_command`, \
         or `build_shell_command` stopped calling `current_dir(...)`. See #3238.",
        actual.display(),
        expected.display(),
    );
    assert_ne!(
        actual, workspace_canon,
        "pwd reported `workspace_dir` instead of `action_dir` — the \
         action/internal split is broken. See #3074, #3238."
    );
}

/// Source-level regression guard for #3238.
///
/// Locks in the contract that the three shell-family acting tools
/// (`shell`, `node_exec`, `npm_exec`) resolve their CWD against
/// `security.action_dir`, never `security.workspace_dir`. The
/// behavioural assertion above covers `shell`; this guard catches
/// regressions in `node_exec` / `npm_exec` without requiring a real
/// Node.js install in CI (their `execute()` path runs
/// `NodeBootstrap::resolve()` first, which is brittle to mock).
///
/// If a future refactor accidentally switches any of these tools
/// back to `workspace_dir`, this assertion fires at compile-time
/// string-match level.
#[test]
fn shell_family_tools_route_cwd_through_action_dir() {
    const SHELL_SRC: &str = include_str!("shell.rs");
    const NODE_EXEC_SRC: &str = include_str!("node_exec.rs");
    const NPM_EXEC_SRC: &str = include_str!("npm_exec.rs");

    // Compose forbidden patterns at runtime so this test's own source
    // doesn't trigger the contains() check on itself.
    let bad_field = format!("self.security.{}_dir", "workspace");
    let bad_call_1 = format!("build_shell_command(&command, &{bad_field})");
    let bad_call_2 = format!("build_shell_command(command, &{bad_field})");

    for (name, src) in [
        ("shell.rs", SHELL_SRC),
        ("node_exec.rs", NODE_EXEC_SRC),
        ("npm_exec.rs", NPM_EXEC_SRC),
    ] {
        // The tool CWD must resolve against `action_dir`, sourced from the
        // tool's own `self.security`. Two accepted spellings:
        //   * direct: `self.security.action_dir` (shell.rs / node_exec.rs)
        //   * workspace-context-aware: `security_for_tool_context(&self.security, …)`
        //     → `resolve_cwd(&path_policy.action_dir, …)` (npm_exec.rs, #4249)
        // Both keep CWD rooted at `action_dir` and tied to `self.security`;
        // neither may reach for `workspace_dir`.
        let direct = src.contains("self.security.action_dir");
        let context_aware = src.contains("security_for_tool_context(&self.security")
            && src.contains("path_policy.action_dir");
        assert!(
            direct || context_aware,
            "{name} must route tool CWD through `action_dir` sourced from \
             `self.security` (see #3074, #3238, #4249)"
        );
        assert!(
            !src.contains(&bad_call_1) && !src.contains(&bad_call_2),
            "{name} must not pass `workspace_dir` to `build_shell_command` — \
             acting tools spawn into `action_dir`. See #3074, #3238."
        );
    }
}

/// Build a `ToolExecutionContext` carrying a `WorkspaceDescriptor` rooted
/// at `root`, mirroring what the tinyagents harness threads into every tool
/// call of a worktree-isolated worker (`RunContext::with_workspace` →
/// `ToolExecutionContext::from_run_context`).
fn tool_context_with_workspace(
    root: &std::path::Path,
) -> tinyagents::harness::tool::ToolExecutionContext {
    use tinyagents::harness::context::{RunConfig, RunContext};
    use tinyagents::harness::tool::ToolExecutionContext;
    use tinyagents::harness::workspace::WorkspaceDescriptor;
    let ws = WorkspaceDescriptor::new(root.to_path_buf()).with_policy_id("test-worktree");
    let ctx: RunContext = RunContext::new(RunConfig::new("test-run"), ()).with_workspace(ws);
    ToolExecutionContext::from_run_context(&ctx)
}

/// Parity guard for the worktree-isolation action-dir override (#3376,
/// #4249 08.5). A worktree-isolated worker's shell command MUST spawn inside
/// the isolated worktree, sourced from the carried `WorkspaceDescriptor`, not
/// the shared `security.action_dir`. This encodes the exact behaviour the
/// deleted `worktree_context.rs` task-local used to provide.
#[cfg(not(windows))]
#[tokio::test]
async fn shell_uses_workspace_descriptor_root_as_cwd() {
    let action_tmp = tempfile::tempdir().expect("create action tempdir");
    let worktree_tmp = tempfile::tempdir().expect("create worktree tempdir");
    let security = Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Full,
        workspace_dir: std::env::temp_dir(),
        action_dir: action_tmp.path().to_path_buf(),
        ..SecurityPolicy::default()
    });
    let tool = ShellTool::new(security.clone(), test_runtime(), test_audit());

    // WITH a descriptor → pwd reports the worktree root, never action_dir.
    let ctx = tool_context_with_workspace(worktree_tmp.path());
    let result = tool
        .execute_with_context(
            json!({"command": "pwd"}),
            crate::openhuman::tools::traits::ToolCallOptions {
                prefer_markdown: false,
            },
            Some(&ctx),
        )
        .await
        .expect("pwd executes");
    assert!(!result.is_error, "{}", result.output());
    let reported = std::path::PathBuf::from(result.output().trim());
    let reported = reported.canonicalize().unwrap_or(reported);
    let expected_wt = worktree_tmp
        .path()
        .canonicalize()
        .unwrap_or_else(|_| worktree_tmp.path().to_path_buf());
    let action_canon = action_tmp
        .path()
        .canonicalize()
        .unwrap_or_else(|_| action_tmp.path().to_path_buf());
    assert_eq!(
        reported, expected_wt,
        "shell with a WorkspaceDescriptor must spawn in the worktree root"
    );
    assert_ne!(
        reported, action_canon,
        "shell must NOT fall back to security.action_dir when a descriptor is present"
    );

    // WITHOUT a descriptor → pwd reports action_dir (non-isolated parity).
    let result = tool
        .execute(json!({"command": "pwd"}))
        .await
        .expect("pwd executes");
    assert!(!result.is_error, "{}", result.output());
    let reported = std::path::PathBuf::from(result.output().trim());
    let reported = reported.canonicalize().unwrap_or(reported);
    assert_eq!(
        reported, action_canon,
        "shell with no descriptor must fall back to security.action_dir"
    );
}

#[tokio::test]
async fn shell_readonly_allows_reads_blocks_writes() {
    let security = test_security(AutonomyLevel::ReadOnly);
    // Read commands are permitted in read-only mode…
    assert_eq!(
        security.gate_decision(security.classify_command("ls")),
        GateDecision::Allow
    );
    // …but a write command is blocked before execution.
    let tool = ShellTool::new(security, test_runtime(), test_audit());
    let blocked = tool
        .execute(json!({"command": "touch ro_test_file"}))
        .await
        .unwrap();
    assert!(blocked.is_error);
    assert!(blocked.output().contains("read-only"));
}

#[tokio::test]
async fn shell_missing_command_param() {
    let tool = ShellTool::new(
        test_security(AutonomyLevel::Supervised),
        test_runtime(),
        test_audit(),
    );
    let result = tool.execute(json!({})).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("command"));
}

#[tokio::test]
async fn shell_wrong_type_param() {
    let tool = ShellTool::new(
        test_security(AutonomyLevel::Supervised),
        test_runtime(),
        test_audit(),
    );
    let result = tool.execute(json!({"command": 123})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn shell_captures_exit_code() {
    let tool = ShellTool::new(
        test_security(AutonomyLevel::Supervised),
        test_runtime(),
        test_audit(),
    );
    let result = tool
        .execute(json!({"command": "ls /nonexistent_dir_xyz"}))
        .await
        .unwrap();
    assert!(result.is_error);
}

/// Regression for the code_executor no-progress loop (#4095): a FAILED
/// command must surface its exit code AND both streams — never drop stdout
/// when stderr is present — so the agent can read *why* it failed instead of
/// re-running it blindly.
#[cfg(not(windows))]
#[tokio::test]
async fn shell_failure_surfaces_exit_code_and_both_streams() {
    let tool = ShellTool::new(
        test_security(AutonomyLevel::Full),
        test_runtime(),
        test_audit(),
    );
    let result = tool
        .execute(json!({
            "command": "echo stdout-marker; echo stderr-marker 1>&2; exit 7"
        }))
        .await
        .unwrap();
    assert!(result.is_error, "non-zero exit must be an error result");
    let out = result.output();
    assert!(out.contains("exit code 7"), "exit code not surfaced: {out}");
    assert!(
        out.contains("stdout-marker"),
        "stdout dropped on failure: {out}"
    );
    assert!(
        out.contains("stderr-marker"),
        "stderr dropped on failure: {out}"
    );
}

/// A missing executable exits 127; the surfaced result must carry the code
/// and the actionable "command not found / missing dependency" hint so the
/// agent recognises the dependency wall and adapts instead of looping.
#[cfg(not(windows))]
#[tokio::test]
async fn shell_missing_command_surfaces_127_with_dependency_hint() {
    let tool = ShellTool::new(
        test_security(AutonomyLevel::Full),
        test_runtime(),
        test_audit(),
    );
    let result = tool
        .execute(json!({"command": "this_binary_does_not_exist_xyz --version"}))
        .await
        .unwrap();
    assert!(result.is_error);
    let out = result.output().to_lowercase();
    assert!(out.contains("127"), "exit code 127 not surfaced: {out}");
    assert!(
        out.contains("command not found"),
        "missing-dependency hint absent: {out}"
    );
}

fn test_security_with_env_cmd() -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        workspace_dir: std::env::temp_dir(),
        action_dir: std::env::temp_dir(),
        allowed_commands: vec!["echo".into(), "mkdir".into()],
        ..SecurityPolicy::default()
    })
}

/// RAII guard that restores an environment variable to its original state on drop,
/// ensuring cleanup even if the test panics.
struct EnvGuard {
    key: &'static str,
    original: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let original = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, original }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(val) => std::env::set_var(self.key, val),
            None => std::env::remove_var(self.key),
        }
    }
}

#[cfg(not(windows))]
#[tokio::test(flavor = "current_thread")]
async fn shell_does_not_leak_api_key() {
    let _g1 = EnvGuard::set("API_KEY", "sk-test-secret-12345");

    let tool = ShellTool::new(test_security_with_env_cmd(), test_runtime(), test_audit());
    let result = tool
        .execute(json!({"command": "echo $API_KEY"}))
        .await
        .unwrap();
    assert!(!result.is_error, "{}", result.output());
    assert!(
        !result.output().contains("sk-test-secret-12345"),
        "API_KEY leaked to shell command output"
    );
}

#[cfg(not(windows))]
#[tokio::test]
async fn shell_preserves_path_and_home() {
    let tool = ShellTool::new(test_security_with_env_cmd(), test_runtime(), test_audit());

    let result = tool
        .execute(json!({"command": "echo $HOME"}))
        .await
        .unwrap();
    assert!(!result.is_error, "{}", result.output());
    assert!(
        !result.output().trim().is_empty(),
        "HOME should be available in shell"
    );

    let result = tool
        .execute(json!({"command": "echo $PATH"}))
        .await
        .unwrap();
    assert!(!result.is_error, "{}", result.output());
    assert!(
        !result.output().trim().is_empty(),
        "PATH should be available in shell"
    );
}

#[tokio::test]
async fn shell_writes_are_gated_in_supervised_run_in_full() {
    // A write command routes through the approval gate in ask-before-edit
    // (no self-asserted `approved` flag any more)…
    let supervised = test_security(AutonomyLevel::Supervised);
    let tool = ShellTool::new(supervised.clone(), test_runtime(), test_audit());
    assert_eq!(supervised.classify_command("touch f"), CommandClass::Write);
    assert!(tool.external_effect_with_args(&json!({"command": "touch f"})));

    // …and runs without prompting in Full.
    let full = test_security(AutonomyLevel::Full);
    let full_tool = ShellTool::new(full, test_runtime(), test_audit());
    assert!(!full_tool.external_effect_with_args(&json!({"command": "touch f"})));
}

#[tokio::test]
async fn shell_llm_category_escalates_but_never_lowers() {
    // In Full a Write runs silently…
    let full = test_security(AutonomyLevel::Full);
    let tool = ShellTool::new(full, test_runtime(), test_audit());
    assert!(!tool.external_effect_with_args(&json!({"command": "touch f"})));
    // …but a self-declared `destructive` escalates it to a prompt.
    assert!(
        tool.external_effect_with_args(&json!({"command": "touch f", "category": "destructive"}))
    );
    // The hint can never LOWER: declaring a destructive command "read"
    // still prompts (in any acting tier).
    let supervised = test_security(AutonomyLevel::Supervised);
    let stool = ShellTool::new(supervised, test_runtime(), test_audit());
    assert!(stool.external_effect_with_args(&json!({"command": "sudo reboot", "category": "read"})));
}

// ── §5.2 Shell timeout enforcement tests ─────────────────

#[test]
fn shell_is_unbounded_by_default() {
    let tool = ShellTool::new(
        test_security(AutonomyLevel::Supervised),
        test_runtime(),
        test_audit(),
    );

    // No `timeout_secs` ⇒ no deadline. A long script must not be hard-killed.
    assert_eq!(
        tool.timeout_policy(&json!({"command": "make"})),
        ToolTimeout::Unbounded
    );
    assert_eq!(tool.explicit_timeout(None), None);
    // An explicit 0 disables the timeout too.
    assert_eq!(
        tool.timeout_policy(&json!({"command": "make", "timeout_secs": 0})),
        ToolTimeout::Unbounded
    );
    assert_eq!(tool.explicit_timeout(Some(0)), None);
}

#[test]
fn shell_timeout_honors_explicit_per_call_value() {
    let tool = ShellTool::new(
        test_security(AutonomyLevel::Supervised),
        test_runtime(),
        test_audit(),
    );

    // An explicit in-range request is enforced verbatim.
    assert_eq!(
        tool.timeout_policy(&json!({"command": "make", "timeout_secs": 1800})),
        ToolTimeout::Secs(1800)
    );
    assert_eq!(
        tool.explicit_timeout(Some(1800)),
        Some(Duration::from_secs(1800))
    );

    // Above the cap clamps down to MAX_TIMEOUT_SECS (3600).
    assert_eq!(
        tool.explicit_timeout(Some(9_999)),
        Some(Duration::from_secs(
            crate::openhuman::tools::timeout::MAX_TIMEOUT_SECS
        ))
    );
}

#[cfg(not(windows))]
#[tokio::test]
async fn shell_per_call_timeout_kills_slow_command() {
    let tool = ShellTool::new(
        test_security(AutonomyLevel::Supervised),
        test_runtime(),
        test_audit(),
    );

    // `sleep 30` would survive any sane default, but a per-call 1s budget
    // must kill it and report the per-call value in the error message.
    let result = tool
        .execute(json!({"command": "sleep 30", "timeout_secs": 1}))
        .await
        .unwrap();

    assert!(result.is_error, "slow command should time out");
    let text = result.text();
    assert!(
        text.contains("timed out after 1s"),
        "timeout message should reflect the per-call budget, got: {text}"
    );
}

#[test]
fn shell_schema_advertises_timeout_secs() {
    let tool = ShellTool::new(
        test_security(AutonomyLevel::Supervised),
        test_runtime(),
        test_audit(),
    );
    let schema = tool.parameters_schema();
    let timeout = &schema["properties"]["timeout_secs"];
    assert_eq!(timeout["type"], "integer");
    assert_eq!(timeout["minimum"], 1);
    assert_eq!(timeout["maximum"], 3600);
}

#[test]
fn shell_output_limit_is_1mb() {
    assert_eq!(
        MAX_OUTPUT_BYTES, 1_048_576,
        "max output must be 1 MB to prevent OOM"
    );
}

// ── §5.3 Non-UTF8 binary output tests ────────────────────

#[test]
fn shell_safe_env_vars_excludes_secrets() {
    for var in SAFE_ENV_VARS {
        let lower = var.to_lowercase();
        assert!(
            !lower.contains("key") && !lower.contains("secret") && !lower.contains("token"),
            "SAFE_ENV_VARS must not include sensitive variable: {var}"
        );
    }
}

#[test]
fn shell_safe_env_vars_includes_essentials() {
    assert!(
        SAFE_ENV_VARS.contains(&"PATH"),
        "PATH must be in safe env vars"
    );
    assert!(
        SAFE_ENV_VARS.contains(&"HOME"),
        "HOME must be in safe env vars"
    );
    assert!(
        SAFE_ENV_VARS.contains(&"TERM"),
        "TERM must be in safe env vars"
    );
}

#[test]
fn shell_safe_env_vars_include_windows_process_essentials() {
    for var in ["SystemRoot", "COMSPEC", "PATHEXT", "TEMP", "USERPROFILE"] {
        assert!(
            SAFE_ENV_VARS.contains(&var),
            "{var} must be forwarded for Windows child processes"
        );
    }
}

#[test]
fn shell_detects_python_runtime_commands() {
    for command in [
        "python3 -m pyfiglet hello",
        "python -m pip install pyfiglet",
        "pip install pyfiglet",
        "pip3.13 show pyfiglet",
        "/opt/openhuman/python/bin/python3 script.py",
        "echo hi && python3 -V",
    ] {
        assert!(
            shell_command_needs_python_runtime(command),
            "expected python runtime detection for {command}"
        );
    }

    for command in [
        "echo python3",
        "ls",
        "cat ./pipelines.txt",
        "node script.js",
    ] {
        assert!(
            !shell_command_needs_python_runtime(command),
            "did not expect python runtime detection for {command}"
        );
    }
}

#[test]
fn shell_runtime_path_prepends_managed_dirs_before_host_path() {
    let python = std::path::Path::new("/opt/openhuman/python/bin");
    let node = std::path::Path::new("/opt/openhuman/node/bin");
    let joined = prepend_path_dirs([python, node], "/usr/local/bin:/usr/bin");
    let sep = if cfg!(windows) { ";" } else { ":" };
    assert_eq!(
        joined,
        format!(
            "{}{}{}{}{}",
            python.display(),
            sep,
            node.display(),
            sep,
            "/usr/local/bin:/usr/bin"
        )
    );
}

/// Empirical answer to "does `shell` resolve/install managed Node on its
/// own?" — NO. The shell path consults the managed Node bootstrap only via
/// `try_cached()`, which never calls `resolve()` and therefore never
/// downloads/installs anything. So without a prior `node_exec` / `npm_exec`
/// (the tools that DO call `resolve()` and share this bootstrap instance),
/// `runtime_path_for_command` injects nothing for a node command. On a host
/// with no Node in the login PATH, the command then fails — the managed
/// runtime is never reached on the shell path. (Python, by contrast, IS
/// self-resolved in `runtime_path_for_command` — see the python branch.)
#[tokio::test]
async fn shell_does_not_resolve_or_install_node_on_its_own() {
    let node = Arc::new(NodeBootstrap::new(Arc::new(
        crate::openhuman::config::Config::default(),
    )));
    let tool = ShellTool::with_language_bootstraps(
        test_security(AutonomyLevel::Full),
        test_runtime(),
        test_audit(),
        Some(node),
        None,
    );

    // Unprimed (no prior node_exec/npm_exec resolve): shell injects NO
    // managed node bin onto PATH — it does not auto-resolve or install.
    let injected = tool
        .runtime_path_for_command("node --version")
        .await
        .expect("runtime path resolves");
    assert!(
        injected.is_none(),
        "shell injected a managed node bin without any prior node_exec/npm_exec \
         resolve — it must not auto-resolve/install on the shell path: {injected:?}"
    );
}

#[tokio::test]
async fn shell_blocks_rate_limited() {
    let security = Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        max_actions_per_hour: 0,
        workspace_dir: std::env::temp_dir(),
        action_dir: std::env::temp_dir(),
        ..SecurityPolicy::default()
    });
    let tool = ShellTool::new(security, test_runtime(), test_audit());
    let result = tool.execute(json!({"command": "echo test"})).await.unwrap();
    assert!(result.is_error);
    assert!(result.output().contains("Rate limit"));
}

#[cfg(not(windows))]
#[tokio::test]
async fn shell_sandboxed_mode_routes_through_sandbox_backend() {
    use crate::openhuman::agent::harness::definition::SandboxMode;
    use crate::openhuman::agent::harness::with_current_sandbox_mode;

    let tool = ShellTool::new(
        test_security(AutonomyLevel::Supervised),
        test_runtime(),
        test_audit(),
    );
    let result = with_current_sandbox_mode(SandboxMode::Sandboxed, async {
        tool.execute(json!({"command": "echo sandboxed-output"}))
            .await
            .unwrap()
    })
    .await;
    assert!(
        !result.is_error,
        "sandboxed echo should succeed: {}",
        result.output()
    );
    assert!(
        result.output().contains("sandboxed-output"),
        "expected 'sandboxed-output' in result, got: {:?}",
        result.output()
    );
}

/// Regression guard for #3235 (cwd_jail wiring for shell-family tools).
///
/// PR #3261 wired `ShellTool` to route through `sandbox::execute_in_sandbox`
/// (which uses `cwd_jail` for the local-OS-jail backend) when the
/// active agent's `SandboxMode::Sandboxed` is set. This PR extends the
/// same wiring to `NodeExecTool` and `NpmExecTool`. The behavioural
/// `shell_sandboxed_mode_routes_through_sandbox_backend` test above
/// proves the contract end-to-end for `shell` (no managed-Node
/// dependency); `node_exec` and `npm_exec` cannot run end-to-end in
/// unit tests without a resolved `NodeBootstrap`, so this source-grep
/// guard catches refactors that drop the sandbox check from either
/// tool's `execute()` body.
#[test]
fn shell_family_tools_route_to_sandbox_when_sandboxed_mode_active() {
    const SHELL_SRC: &str = include_str!("shell.rs");
    const NODE_EXEC_SRC: &str = include_str!("node_exec.rs");
    const NPM_EXEC_SRC: &str = include_str!("npm_exec.rs");

    for (name, src) in [
        ("shell.rs", SHELL_SRC),
        ("node_exec.rs", NODE_EXEC_SRC),
        ("npm_exec.rs", NPM_EXEC_SRC),
    ] {
        assert!(
            src.contains("current_sandbox_mode()"),
            "{name} must check `current_sandbox_mode()` to detect SandboxMode::Sandboxed \
             sessions and route through the sandbox backend (see #3235)"
        );
        assert!(
            src.contains("SandboxMode::Sandboxed"),
            "{name} must compare against `SandboxMode::Sandboxed` to opt in to the \
             sandbox routing path (see #3235)"
        );
        // Use the call-site pattern `.run_sandboxed(` so the assertion
        // doesn't trivially pass on the helper definition itself
        // (`fn run_sandboxed(...)`). If `execute()` / `run_with_security()`
        // stop delegating, this fires even though the helper still exists.
        assert!(
            src.contains(".run_sandboxed("),
            "{name} must delegate to a `run_sandboxed` helper when the sandbox mode is \
             active (see #3235). Whitespace before `.run_sandboxed` is tolerated; the \
             helper call must appear in the source — *not* just the helper definition."
        );
    }
}
