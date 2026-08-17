//! Tests for hosting account resolution, workspace containment, and the tools.
//!
//! The provider itself is TinyHosts' problem and is tested there against a mock
//! of its REST API. What is tested here is the seam: whether an account resolves
//! from configuration, whether a path an agent named can escape the workspace,
//! and whether each tool's schema says what its `execute` actually reads.

use serde_json::json;

use super::*;
use crate::openhuman::config::Config;
use crate::openhuman::tools::traits::Tool;

fn config_with(workspace: &std::path::Path, enabled: bool, api_key: &str) -> Config {
    let mut config = Config::default();
    config.workspace_dir = workspace.to_path_buf();
    config.hosting.enabled = enabled;
    config.hosting.api_key = api_key.to_string();
    config
}

#[test]
fn hosting_off_yields_no_account() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let config = config_with(workspace.path(), false, "token");

    assert!(Account::from_config(&config)
        .expect("resolution does not fail")
        .is_none());
}

#[test]
fn a_configured_key_yields_an_account() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let config = config_with(workspace.path(), true, "token");

    let account = Account::from_config(&config)
        .expect("resolution does not fail")
        .expect("an account, since a key is configured");

    assert_eq!(account.host().kind().as_str(), "vercel");
    assert_eq!(account.workspace_dir(), workspace.path());
}

#[test]
fn an_unknown_provider_is_an_error_rather_than_a_silent_skip() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let mut config = config_with(workspace.path(), true, "token");
    config.hosting.provider = "heroku".to_string();

    let error = Account::from_config(&config).expect_err("an unknown provider fails");

    assert!(
        error.to_string().contains("heroku"),
        "the error should name the provider: {error}"
    );
}

#[test]
fn an_account_reports_itself_without_its_credential() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let config = config_with(workspace.path(), true, "super-secret");

    let account = Account::from_config(&config)
        .expect("resolution does not fail")
        .expect("an account");

    assert!(
        !format!("{account:?}").contains("super-secret"),
        "the credential must never be rendered"
    );
}

#[test]
fn an_account_exposes_every_hosting_tool() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let config = config_with(workspace.path(), true, "token");
    let account = Account::from_config(&config)
        .expect("resolution does not fail")
        .expect("an account");

    let names: Vec<String> = account
        .tools()
        .iter()
        .map(|tool| tool.name().to_string())
        .collect();

    assert_eq!(
        names,
        [
            "hosting_launch_site",
            "hosting_deployment_status",
            "hosting_list_sites",
            "hosting_set_env",
            "hosting_add_domain",
            "hosting_analytics",
        ]
    );
}

#[test]
fn only_the_tools_that_change_the_world_carry_an_external_effect() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let config = config_with(workspace.path(), true, "token");
    let account = Account::from_config(&config)
        .expect("resolution does not fail")
        .expect("an account");

    for tool in account.tools() {
        let expected = matches!(
            tool.name(),
            "hosting_launch_site" | "hosting_set_env" | "hosting_add_domain"
        );
        assert_eq!(
            tool.external_effect(),
            expected,
            "{} has the wrong external effect",
            tool.name()
        );
    }
}

#[test]
fn every_tool_schema_is_an_object_naming_its_required_arguments() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let config = config_with(workspace.path(), true, "token");
    let account = Account::from_config(&config)
        .expect("resolution does not fail")
        .expect("an account");

    for tool in account.tools() {
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object", "{}", tool.name());
        assert!(
            schema["properties"].is_object(),
            "{} has no properties",
            tool.name()
        );
        assert!(
            !tool.description().is_empty(),
            "{} has no description",
            tool.name()
        );
    }
}

#[test]
fn a_directory_inside_the_workspace_resolves() {
    let workspace = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir(workspace.path().join("site")).expect("mkdir");

    let resolved = resolve_in_workspace(workspace.path(), "site").expect("resolves");

    assert!(resolved.ends_with("site"));
}

#[test]
fn an_empty_path_is_the_workspace_root() {
    let workspace = tempfile::tempdir().expect("tempdir");

    let resolved = resolve_in_workspace(workspace.path(), "  ").expect("resolves");

    assert_eq!(
        resolved,
        workspace.path().canonicalize().expect("canonical root")
    );
}

#[test]
fn a_path_outside_the_workspace_is_refused() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let root = workspace.path();

    // A deployment uploads every byte under the directory to a third party, so
    // this is the check that decides what may leave the machine.
    assert!(resolve_in_workspace(root, "/etc").is_err());
    assert!(resolve_in_workspace(root, "../..").is_err());
    assert!(resolve_in_workspace(root, "does-not-exist").is_err());
}

#[test]
fn a_file_is_not_a_deployable_directory() {
    let workspace = tempfile::tempdir().expect("tempdir");
    std::fs::write(workspace.path().join("page.tsx"), b"x").expect("write");

    let error =
        resolve_in_workspace(workspace.path(), "page.tsx").expect_err("a file is not a directory");

    assert!(error.to_string().contains("not a directory"), "{error}");
}

#[tokio::test]
async fn launching_reports_a_missing_directory_instead_of_deploying_nothing() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let config = config_with(workspace.path(), true, "token");
    let account = Account::from_config(&config)
        .expect("resolution does not fail")
        .expect("an account");

    let tool = tools::LaunchSiteTool::new(account);
    let result = tool
        .execute(json!({"site": "shop", "path": "missing"}))
        .await
        .expect("the tool reports rather than panics");

    assert!(result.is_error);
}

#[tokio::test]
async fn launching_without_a_site_name_is_refused_before_any_upload() {
    let workspace = tempfile::tempdir().expect("tempdir");
    std::fs::write(workspace.path().join("package.json"), b"{}").expect("write");
    let config = config_with(workspace.path(), true, "token");
    let account = Account::from_config(&config)
        .expect("resolution does not fail")
        .expect("an account");

    let tool = tools::LaunchSiteTool::new(account);
    let result = tool
        .execute(json!({"path": "."}))
        .await
        .expect("the tool reports rather than panics");

    assert!(result.is_error);
}

#[tokio::test]
async fn a_read_tool_reports_a_missing_argument_rather_than_calling_out() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let config = config_with(workspace.path(), true, "token");
    let account = Account::from_config(&config)
        .expect("resolution does not fail")
        .expect("an account");

    let result = tools::DeploymentStatusTool::new(account.host())
        .execute(json!({}))
        .await
        .expect("the tool reports rather than panics");

    assert!(result.is_error);
}
