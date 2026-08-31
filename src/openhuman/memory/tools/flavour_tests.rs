use super::*;
use tempfile::TempDir;

fn test_config() -> (TempDir, Arc<Config>) {
    let tmp = TempDir::new().unwrap();
    let mut cfg = Config::default();
    cfg.workspace_dir = tmp.path().to_path_buf();
    (tmp, Arc::new(cfg))
}

#[test]
fn name_and_schema() {
    let (_tmp, cfg) = test_config();
    let tool = MemoryFlavourTool::new(cfg);
    assert_eq!(tool.name(), "memory_flavour");
    assert_eq!(tool.parameters_schema()["required"], json!(["flavour"]));
    assert!(tool.parameters_schema()["properties"]["flavour"].is_object());
}

#[test]
fn permission_level_is_read_only() {
    let (_tmp, cfg) = test_config();
    let tool = MemoryFlavourTool::new(cfg);
    assert_eq!(tool.permission_level(), PermissionLevel::ReadOnly);
}

#[test]
fn permission_level_with_args_is_always_read_only() {
    let (_tmp, cfg) = test_config();
    let tool = MemoryFlavourTool::new(cfg);
    assert_eq!(
        tool.permission_level_with_args(&json!({})),
        PermissionLevel::ReadOnly
    );
    assert_eq!(
        tool.permission_level_with_args(&json!({"flavour": "communication"})),
        PermissionLevel::ReadOnly
    );
}

#[tokio::test]
async fn missing_flavour_is_error() {
    let (_tmp, cfg) = test_config();
    let tool = MemoryFlavourTool::new(cfg);
    let result = tool.execute(json!({})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn empty_flavour_is_error() {
    let (_tmp, cfg) = test_config();
    let tool = MemoryFlavourTool::new(cfg);
    let result = tool.execute(json!({"flavour": "   "})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn unknown_flavour_is_error() {
    let (_tmp, cfg) = test_config();
    let tool = MemoryFlavourTool::new(cfg);
    let result = tool.execute(json!({"flavour": "astrology"})).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unknown flavour"));
}

#[tokio::test]
async fn valid_flavour_with_no_tree_yet_returns_no_profile_message() {
    let (_tmp, cfg) = test_config();
    let tool = MemoryFlavourTool::new(cfg);
    let result = tool
        .execute(json!({"flavour": "coding_style"}))
        .await
        .unwrap();
    assert!(!result.is_error);
    assert!(result.output().contains("No profile built yet"));
}

#[tokio::test]
async fn aliases_are_accepted() {
    for alias in ["comms", "coding", "env", "rules", "dislikes"] {
        let (_tmp, cfg) = test_config();
        let tool = MemoryFlavourTool::new(cfg);
        let result = tool.execute(json!({"flavour": alias})).await;
        assert!(result.is_ok(), "alias `{alias}` should be accepted");
        let result = result.unwrap();
        assert!(!result.is_error, "alias `{alias}` should not error");
        assert!(result.output().contains("No profile built yet"));
    }
}
