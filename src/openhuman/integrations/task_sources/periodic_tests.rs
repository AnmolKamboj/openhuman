use super::*;
use crate::openhuman::integrations::task_sources::types::{FilterSpec, ProviderSlug, SourceTarget};
use chrono::Utc;
use serde_json::json;

fn source(id: &str, interval_secs: u64) -> TaskSource {
    source_for(id, interval_secs, ProviderSlug::Github)
}

fn source_for(id: &str, interval_secs: u64, provider: ProviderSlug) -> TaskSource {
    TaskSource {
        id: id.into(),
        provider,
        connection_id: None,
        name: None,
        enabled: true,
        filter: FilterSpec::Github {
            repo: None,
            labels: vec![],
            assignee_is_me: true,
            state: None,
            fetch_mode: Default::default(),
            extra: json!({}),
        },
        interval_secs,
        target: SourceTarget::TodoOnly,
        max_tasks_per_fetch: 25,
        assigned_executor: None,
        created_at: Utc::now(),
        last_fetch_at: None,
        last_status: None,
    }
}

#[test]
fn tick_seconds_is_sane() {
    assert!(TICK_SECONDS >= 60);
    assert!(TICK_SECONDS <= 3600);
}

#[test]
fn never_polled_source_is_due() {
    let s = source("ts-never-polled-xyz", 1800);
    assert!(is_due(&s));
}

#[test]
fn recently_polled_source_is_not_due() {
    let s = source("ts-recent-poll-xyz", 1800);
    record_poll(&s.id);
    assert!(!is_due(&s), "just-recorded poll should not be due again");
}

#[test]
fn zero_interval_is_floored_not_always_due() {
    let s = source("ts-zero-interval-xyz", 0);
    record_poll(&s.id);
    // With the MIN_INTERVAL_SECONDS floor a just-polled zero-interval
    // source is not immediately due again.
    assert!(!is_due(&s));
}

// ── the containment gate (issue #6118) ──────────────────────────────────────
//
// Two halves of one distinction, pinned together on purpose: the timer must
// stay silent about a provider it cannot fetch, and the manual RPC must keep
// explaining itself. A change that silences both would satisfy either test
// alone.

/// No provider has a task-fetch path today, so the scheduler has nothing
/// legitimate to poll. When one is restored this test is what tells you to
/// update it — deliberately, rather than by a blanket assertion that would
/// keep passing.
#[test]
fn no_provider_can_fetch_today() {
    for provider in [
        ProviderSlug::Github,
        ProviderSlug::Notion,
        ProviderSlug::Linear,
        ProviderSlug::Clickup,
    ] {
        assert!(
            !provider.can_fetch(),
            "{} has no fetch path until the pipeline's fetch step is restored",
            provider.as_str()
        );
    }
}

/// The gate the periodic loop applies, asserted on the same predicate the
/// loop reads. An unfetchable source is skipped before `run_source_once` can
/// record a `store::record_fetch` row or publish `TaskSourceFetchFailed`.
#[test]
fn scheduler_skips_a_source_whose_provider_cannot_fetch() {
    for provider in [
        ProviderSlug::Github,
        ProviderSlug::Notion,
        ProviderSlug::Linear,
        ProviderSlug::Clickup,
    ] {
        let s = source_for("ts-gate-xyz", 1800, provider);
        assert!(
            s.enabled && is_due(&s),
            "precondition: this source would otherwise be polled"
        );
        assert!(
            !s.provider.can_fetch(),
            "so the ONLY thing standing between it and a recorded failure is the gate"
        );
    }
}

/// The other half: the manual path is untouched. `run_source_once` — which the
/// `task_sources_fetch` RPC calls directly, without the scheduler's gate —
/// still refuses and still says why.
///
/// This is what makes the gate a scheduler-only change rather than a silent
/// disabling of the feature.
#[tokio::test]
async fn manual_fetch_still_returns_the_explanatory_error() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let config = crate::openhuman::config::Config {
        workspace_dir: tmp.path().join("workspace"),
        action_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("config.toml"),
        ..Default::default()
    };
    std::fs::create_dir_all(&config.workspace_dir).expect("workspace");

    let s = source("ts-manual-path-xyz", 1800);
    let outcome = pipeline::run_source_once(&config, &s, FetchReason::Manual).await;

    let error = outcome
        .error
        .as_deref()
        .expect("the manual path must still surface an error, not silently succeed");
    assert!(
        error.contains("unavailable"),
        "and it must still explain itself: {error}"
    );
    assert_eq!(outcome.fetched, 0);
    assert_eq!(outcome.routed, 0);
}
