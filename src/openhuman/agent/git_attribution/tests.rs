#[cfg(unix)]
#[test]
fn hook_adds_openhuman_trailer_without_disabling_repository_hook() {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .args(args)
            .current_dir(&repo)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["config", "user.name", "Test"]);
    git(&["config", "user.email", "test@example.com"]);
    let own_hook = repo.join(".git/hooks/prepare-commit-msg");
    std::fs::write(
        &own_hook,
        "#!/bin/sh\nprintf 'repo-hook-ran\\n' >> \"$1\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&own_hook, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::write(repo.join("a"), "a").unwrap();
    git(&["add", "a"]);

    let hook_dir = super::hook::test_hook_dir();
    let output = Command::new("git")
        .args(["commit", "-q", "-m", "subject"])
        .current_dir(&repo)
        .env("OPENHUMAN_GIT_ATTRIBUTION", super::hook::TRAILER)
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "core.hooksPath")
        .env("GIT_CONFIG_VALUE_0", &hook_dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = Command::new("git")
        .args(["log", "-1", "--format=%B"])
        .current_dir(&repo)
        .output()
        .unwrap();
    let message = String::from_utf8(output.stdout).unwrap();
    assert!(message.contains("repo-hook-ran"), "{message:?}");
    assert!(message.contains(super::hook::TRAILER), "{message:?}");
}
