use super::*;

/// The reset must be safe to run against a healthy (or absent) store: it
/// drops the cached handle and reopens without quarantining anything.
#[test]
fn reset_reopens_a_healthy_or_absent_store_without_quarantining() {
    let tmp = tempfile::tempdir().unwrap();
    let mut config = Config::default();
    config.workspace_dir = tmp.path().join("workspace");
    std::fs::create_dir_all(config.workspace_dir.join("memory_tree")).unwrap();

    reset_in_process_chunk_store(&config);

    let entries: Vec<String> = std::fs::read_dir(config.workspace_dir.join("memory_tree"))
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        !entries.iter().any(|name| name.contains(".corrupt-")),
        "a healthy or absent store must never be quarantined by the reset: {entries:?}"
    );
}
