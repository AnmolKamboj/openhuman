use super::*;

fn result(name: &str, success: bool, content: &str) -> CheckpointToolResult {
    CheckpointToolResult {
        name: name.to_string(),
        success,
        content: content.to_string(),
    }
}

/// The point of the rewrite: a capped turn hands back what it actually
/// gathered, not a list of tool names. Asserting on the names alone (which is
/// all the round24 e2e did) would stay green with the whole body dropped
/// (tinysweeper on #6068).
#[test]
fn the_deterministic_checkpoint_reproduces_each_result_body() {
    let out = build_deterministic_checkpoint(
        &[
            result("list_issues", true, "#41 flaky login\n#57 slow boot"),
            result("fetch_pr", false, "404 not found"),
        ],
        25,
    );

    assert!(out.contains("I reached the tool-call limit for this turn (25 steps)"));
    // Each body is reproduced, blockquoted line by line.
    assert!(out.contains("  > #41 flaky login"), "missing body: {out}");
    assert!(out.contains("  > #57 slow boot"), "missing body: {out}");
    assert!(out.contains("  > 404 not found"), "missing body: {out}");
    // Status is carried per result, so a failure is not read as data.
    assert!(out.contains("`list_issues` — ok"));
    assert!(out.contains("`fetch_pr` — failed"));
    // Nothing was dropped, so nothing is disclosed as dropped.
    assert!(!out.contains("omitted for length"));
}

/// Over the budget the checkpoint starts later rather than truncating every
/// body, and says how many it skipped — a silent drop reads as "that is all
/// there was".
#[test]
fn results_over_the_budget_are_omitted_with_a_disclosure() {
    // Six bodies of 1k each against a 4k budget: the oldest cannot fit.
    let body = "y".repeat(1_000);
    let results: Vec<CheckpointToolResult> = (0..6)
        .map(|i| result(&format!("tool_{i}"), true, &body))
        .collect();

    let out = build_deterministic_checkpoint(&results, 25);

    assert!(
        out.contains("earlier tool result(s) omitted for length"),
        "an omitted count must be disclosed: {out}"
    );
    // Newest kept, oldest dropped — and the kept ones are in original order.
    assert!(out.contains("`tool_5` — ok"), "newest must survive: {out}");
    assert!(
        !out.contains("`tool_0` — ok"),
        "oldest must be dropped: {out}"
    );
    let five = out.find("`tool_5`").expect("newest rendered");
    let four = out.find("`tool_4`").expect("second-newest rendered");
    assert!(four < five, "kept results must read oldest-first");
}

/// A single oversized payload must not empty the checkpoint: the newest result
/// is shown however long it is, or a capped turn concludes with nothing at all.
#[test]
fn one_oversized_result_is_still_shown() {
    let huge = "z".repeat(CHECKPOINT_TOTAL_CHARS * 3);
    let out = build_deterministic_checkpoint(&[result("dump", true, &huge)], 25);

    assert!(out.contains("`dump` — ok"));
    assert!(out.contains("  > zzz"), "the only result must be rendered");
    assert!(!out.contains("no tools completed yet"));
}

/// The cap can be reached before anything finishes; say so rather than
/// rendering an empty results section.
#[test]
fn no_completed_tools_says_so() {
    let out = build_deterministic_checkpoint(&[], 25);
    assert!(out.contains("(no tools completed yet)"));
    assert!(out.contains("**Next steps:**"));
}
