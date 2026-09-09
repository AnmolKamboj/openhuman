use super::*;
use crate::openhuman::security::{AutonomyLevel, SecurityPolicy};

fn test_security(workspace: std::path::PathBuf) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        workspace_dir: workspace.clone(),
        action_dir: workspace,
        ..SecurityPolicy::default()
    })
}

fn test_security_readonly(workspace: std::path::PathBuf) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::ReadOnly,
        workspace_dir: workspace.clone(),
        action_dir: workspace,
        ..SecurityPolicy::default()
    })
}

#[test]
fn edit_name() {
    let tool = EditFileTool::new(test_security(std::env::temp_dir()));
    assert_eq!(tool.name(), "edit");
}

#[tokio::test]
async fn edit_replaces_unique_match() {
    let dir = std::env::temp_dir().join("openhuman_test_edit_unique");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(dir.join("f.txt"), "alpha bravo")
        .await
        .unwrap();

    let tool = EditFileTool::new(test_security(dir.clone()));
    let result = tool
        .execute(json!({"path": "f.txt", "old_string": "bravo", "new_string": "charlie"}))
        .await
        .unwrap();
    assert!(!result.is_error, "{}", result.output());
    let updated = tokio::fs::read_to_string(dir.join("f.txt")).await.unwrap();
    assert_eq!(updated, "alpha charlie");

    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[tokio::test]
async fn edit_rejects_ambiguous_match() {
    let dir = std::env::temp_dir().join("openhuman_test_edit_ambig");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(dir.join("f.txt"), "x x x").await.unwrap();

    let tool = EditFileTool::new(test_security(dir.clone()));
    let result = tool
        .execute(json!({"path": "f.txt", "old_string": "x", "new_string": "y"}))
        .await
        .unwrap();
    assert!(result.is_error);
    assert!(result.output().contains("matches 3 times"));

    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[tokio::test]
async fn edit_replace_all() {
    let dir = std::env::temp_dir().join("openhuman_test_edit_all");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(dir.join("f.txt"), "x x x").await.unwrap();

    let tool = EditFileTool::new(test_security(dir.clone()));
    let result = tool
        .execute(
            json!({"path": "f.txt", "old_string": "x", "new_string": "y", "replace_all": true}),
        )
        .await
        .unwrap();
    assert!(!result.is_error);
    let updated = tokio::fs::read_to_string(dir.join("f.txt")).await.unwrap();
    assert_eq!(updated, "y y y");

    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[tokio::test]
async fn edit_no_match() {
    let dir = std::env::temp_dir().join("openhuman_test_edit_nomatch");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(dir.join("f.txt"), "alpha").await.unwrap();

    let tool = EditFileTool::new(test_security(dir.clone()));
    let result = tool
        .execute(json!({"path": "f.txt", "old_string": "zulu", "new_string": "x"}))
        .await
        .unwrap();
    assert!(result.is_error);
    assert!(result.output().contains("not found"));

    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[tokio::test]
async fn edit_blocks_readonly_mode() {
    let dir = std::env::temp_dir().join("openhuman_test_edit_ro");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(dir.join("f.txt"), "abc").await.unwrap();

    let tool = EditFileTool::new(test_security_readonly(dir.clone()));
    let result = tool
        .execute(json!({"path": "f.txt", "old_string": "abc", "new_string": "xyz"}))
        .await
        .unwrap();
    assert!(result.is_error);
    assert!(result.output().contains("read-only"));

    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[tokio::test]
async fn edit_rejects_empty_old_string() {
    let dir = std::env::temp_dir().join("openhuman_test_edit_empty_old");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(dir.join("f.txt"), "abc").await.unwrap();

    let tool = EditFileTool::new(test_security(dir.clone()));
    let result = tool
        .execute(json!({"path": "f.txt", "old_string": "", "new_string": "x"}))
        .await
        .unwrap();
    assert!(result.is_error);

    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[tokio::test]
async fn edit_rejects_identical_strings() {
    let dir = std::env::temp_dir().join("openhuman_test_edit_same");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(dir.join("f.txt"), "abc").await.unwrap();

    let tool = EditFileTool::new(test_security(dir.clone()));
    let result = tool
        .execute(json!({"path": "f.txt", "old_string": "abc", "new_string": "abc"}))
        .await
        .unwrap();
    assert!(result.is_error);
    assert!(result.output().contains("identical"));

    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[tokio::test]
#[cfg(unix)]
async fn edit_reports_an_os_write_failure_rather_than_a_silent_success() {
    use std::os::unix::fs::PermissionsExt;

    // SAFETY: `geteuid` takes no arguments, mutates no process state, and is
    // documented as always succeeding.
    let euid = unsafe { libc::geteuid() };
    if euid == 0 {
        // A mode-bit write denial is a DAC check; root carries
        // CAP_DAC_OVERRIDE and bypasses it, so the write below would
        // succeed and this test would assert nothing. Skip loudly rather
        // than pass on a precondition that never held.
        eprintln!(
            "edit_reports_an_os_write_failure_rather_than_a_silent_success: \
             skipped — running as root, which bypasses file-mode write checks"
        );
        return;
    }

    let dir = std::env::temp_dir().join("openhuman_test_edit_os_write_fail");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();
    let file = dir.join("f.txt");
    tokio::fs::write(&file, "abc").await.unwrap();
    // SecurityPolicy is `Supervised` (can_act), so the tool's own gate lets
    // this through; the failure comes from the OS `write` call itself.
    tokio::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o444))
        .await
        .unwrap();

    let tool = EditFileTool::new(test_security(dir.clone()));
    let result = tool
        .execute(json!({"path": "f.txt", "old_string": "abc", "new_string": "xyz"}))
        .await
        .unwrap();

    tokio::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644))
        .await
        .unwrap();
    assert!(
        result.is_error,
        "an OS-level write failure must surface as an error, not a fabricated success"
    );
    assert!(result.output().contains("Failed to write file"));
    assert_eq!(
        tokio::fs::read_to_string(&file).await.unwrap(),
        "abc",
        "the file must be left untouched when the write itself failed"
    );

    let _ = tokio::fs::remove_dir_all(&dir).await;
}
