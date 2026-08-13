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

    let hook_env = super::hook::test_hook_env(Some(
        "'test.openhuman-inherited'='kept' 'core.hooksPath'='/definitely-not-the-openhuman-hook'",
    ));
    let output = Command::new("git")
        .args(["commit", "-q", "-m", "subject"])
        .current_dir(&repo)
        .envs(&hook_env)
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

    let inherited = Command::new("git")
        .args(["config", "--get", "test.openhuman-inherited"])
        .current_dir(&repo)
        .envs(&hook_env)
        .output()
        .unwrap();
    assert!(inherited.status.success());
    assert_eq!(String::from_utf8(inherited.stdout).unwrap().trim(), "kept");
}
