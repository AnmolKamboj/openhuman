//! `write_pause_checkpoint` — the pause-persistence contract (#5928).
//!
//! Every failure branch returns `None`, and that `None` is what stops the
//! runner reporting a resumable `AwaitingUser` for a pause it never saved.
//! Before this, all three failures were logged at `warn` and then dropped, so
//! the orchestrator relayed a `task_id` and asked the user to answer a question
//! whose answer had nowhere to go.

use super::write_pause_checkpoint;
use crate::openhuman::agent::harness::subagent_runner::types::SubagentCheckpointData;

fn checkpoint_data(task_id: &str) -> SubagentCheckpointData {
    SubagentCheckpointData {
        task_id: task_id.to_string(),
        agent_id: "researcher".to_string(),
        worker_thread_id: None,
        history: Vec::new(),
        question: "Which region?".to_string(),
        options: None,
        toolkit_override: None,
        skill_filter_override: None,
        model_override: None,
        created_at: "2026-09-02T00:00:00Z".to_string(),
    }
}

#[test]
fn a_written_checkpoint_returns_the_path_it_wrote() {
    let dir = tempfile::tempdir().expect("tempdir");
    let checkpoint_dir = dir.path().join("subagent_checkpoints");

    let written = write_pause_checkpoint(&checkpoint_dir, "task-1", &checkpoint_data("task-1"))
        .expect("a writable directory must produce a checkpoint");

    assert_eq!(
        written,
        checkpoint_dir.join("task-1.json"),
        "the returned path must be the one the resume flow will read"
    );
    assert!(written.is_file(), "the checkpoint must exist on disk");

    // Round-trips: a resume reads this back with `serde_json::from_str`, so a
    // path that exists but cannot be parsed is no better than a missing one.
    let raw = std::fs::read_to_string(&written).expect("read back");
    let parsed: SubagentCheckpointData = serde_json::from_str(&raw).expect("checkpoint parses");
    assert_eq!(parsed.task_id, "task-1");
    assert_eq!(parsed.question, "Which region?");
}

#[test]
fn the_directory_is_created_on_demand() {
    // #5928 asks whether the checkpoint directory is created correctly on fresh
    // installs. It is: nothing pre-creates it, `write_pause_checkpoint` does,
    // including intermediate components.
    let dir = tempfile::tempdir().expect("tempdir");
    let nested = dir.path().join("a/b/c/subagent_checkpoints");
    assert!(!nested.exists(), "precondition: nothing has created it");

    let written = write_pause_checkpoint(&nested, "task-2", &checkpoint_data("task-2"))
        .expect("a missing directory must be created, not treated as a failure");

    assert!(written.is_file());
}

#[test]
fn an_uncreatable_directory_reports_no_checkpoint() {
    // A regular file where the directory should be: `create_dir_all` cannot
    // succeed, and the pause is not resumable from disk.
    let dir = tempfile::tempdir().expect("tempdir");
    let blocked = dir.path().join("not_a_dir");
    std::fs::write(&blocked, b"i am a file").expect("seed the blocker");

    assert!(
        write_pause_checkpoint(&blocked, "task-3", &checkpoint_data("task-3")).is_none(),
        "a checkpoint directory that cannot exist must report no checkpoint, \
         not a silently dropped warning"
    );
}

#[test]
fn an_unwritable_target_reports_no_checkpoint() {
    // The directory exists, but the checkpoint's own path is occupied by a
    // directory, so the write cannot land.
    let dir = tempfile::tempdir().expect("tempdir");
    let checkpoint_dir = dir.path().join("subagent_checkpoints");
    std::fs::create_dir_all(checkpoint_dir.join("task-4.json")).expect("occupy the target path");

    assert!(
        write_pause_checkpoint(&checkpoint_dir, "task-4", &checkpoint_data("task-4")).is_none(),
        "a checkpoint whose write fails must report no checkpoint"
    );
}
