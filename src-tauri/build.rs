use std::env;
use std::path::PathBuf;

fn main() {
    maybe_override_tauri_config_for_tests();
    tauri_build::build();
}

fn maybe_override_tauri_config_for_tests() {
    let profile = env::var("PROFILE").unwrap_or_default();
    let skip_resources = env::var("TAURI_SKIP_RESOURCES").is_ok() || profile == "test";
    if !skip_resources {
        return;
    }

    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));
    let config_path = manifest_dir.join("tauri.conf.json");
    let Ok(raw) = std::fs::read_to_string(&config_path) else {
        println!("cargo:warning=Failed to read tauri.conf.json; keeping default config");
        return;
    };

    let mut value: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(err) => {
            println!("cargo:warning=Failed to parse tauri.conf.json: {err}");
            return;
        }
    };

    if let Some(bundle) = value.get_mut("bundle").and_then(|b| b.as_object_mut()) {
        bundle.insert("resources".to_string(), serde_json::Value::Array(Vec::new()));
    }

    let out_dir = env::var("OUT_DIR").unwrap_or_else(|_| ".".into());
    let override_path = PathBuf::from(out_dir).join("tauri.conf.test.json");
    if std::fs::write(&override_path, serde_json::to_string_pretty(&value).unwrap_or(raw)).is_ok()
    {
        env::set_var("TAURI_CONFIG", &override_path);
        println!(
            "cargo:warning=TAURI resources disabled for test build (using {})",
            override_path.display()
        );
    }
}

