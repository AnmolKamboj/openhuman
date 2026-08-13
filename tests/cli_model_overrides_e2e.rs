//! End-to-end coverage for transient model/provider selection on the CLI.

use std::process::Command;

use serde_json::Value;

#[test]
fn cli_model_and_provider_flags_override_the_loaded_session_without_persisting() {
    let workspace = tempfile::tempdir().expect("temporary OpenHuman workspace");
    let output = Command::new(env!("CARGO_BIN_EXE_openhuman-core"))
        .args([
            "--provider",
            "ollama",
            "--model",
            "qwen3:8b",
            "--no-tui",
            "inference",
            "get_client_config",
        ])
        .env("OPENHUMAN_WORKSPACE", workspace.path())
        .output()
        .expect("run OpenHuman CLI");

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let response: Value = serde_json::from_slice(&output.stdout).expect("JSON CLI response");
    let result = response.get("result").expect("RPC result");
    for field in [
        "chat_provider",
        "reasoning_provider",
        "agentic_provider",
        "coding_provider",
    ] {
        assert_eq!(
            result.get(field).and_then(Value::as_str),
            Some("ollama:qwen3:8b"),
            "{field} did not receive the CLI override"
        );
    }
    assert_eq!(
        result.get("default_model").and_then(Value::as_str),
        Some("qwen3:8b")
    );

    let persisted =
        std::fs::read_to_string(workspace.path().join("config.toml")).expect("persisted config");
    assert!(
        !persisted.contains("qwen3:8b"),
        "transient CLI model leaked into config.toml"
    );
}

#[test]
fn cli_rejects_a_missing_model_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_openhuman-core"))
        .arg("--model")
        .output()
        .expect("run OpenHuman CLI");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing value for --model"));
}
