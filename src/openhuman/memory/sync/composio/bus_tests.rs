//! Unit tests for the composio connection-created event handler's gating.

use super::toolkit_is_memory_source_registrable;

/// #4957 regression, in the half that is still this host's.
///
/// The rule the original test locked — which toolkits have a native pipeline —
/// is the driver's now (`MemorySourceSync::is_toolkit_syncable`,
/// tinymemory#106), and it is tested there against the same offenders this one
/// named: `is_composio_toolkit_syncable("googlecalendar")` is false and
/// `" Gmail "` is true, so the normalisation is pinned upstream too. Asserting
/// the list again here would need a loaded module and would only re-check the
/// driver's answer.
///
/// What stays host-side is the **direction the host fails in**, and it is the
/// half that caused #4957: a toolkit that is auto-registered but cannot sync
/// reports ACTIVE and then fails forever, which is a silent lie. So when the
/// driver cannot answer — no binding, no `SourceSync` family, or `Unsupported`
/// — the host must treat the toolkit as *not* registrable rather than assume
/// it is fine.
///
/// A unit test has no loaded module, so every answer here is the
/// cannot-answer case, which is exactly the branch worth pinning.
#[tokio::test]
async fn a_driver_that_cannot_answer_makes_a_toolkit_not_registrable() {
    let config = crate::openhuman::config::Config::default();

    for toolkit in ["gmail", "slack", "github", "googlecalendar", ""] {
        assert!(
            !toolkit_is_memory_source_registrable(&config, toolkit).await,
            "with no module bound, '{toolkit}' must not be registrable — \
             registering a source that then fails every sync is #4957"
        );
    }
}
