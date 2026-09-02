//! End-to-end coverage for the wallet-not-configured reporting decision
//! (#5805, fixed by #5811).
//!
//! An unconfigured wallet is the default state of an **optional** feature, so
//! the condition must never reach Sentry as an error. #5805 measured 55 error
//! events in 72 minutes from one ordinary local session, while a genuine
//! turn-killing failure in the same session emitted nothing — severity
//! inverted in both directions at once.
//!
//! The part that regressed is specifically the **context-wrapped** form.
//! `jsonrpc.rs` already demoted the bare sentinel via an exact-equality
//! predicate, but `hosted/orchestration/schemas.rs` lifts the wallet error
//! into an RPC failure with
//!
//! ```text
//! .map_err(|e| format!("self_identity key_status: {e}"))?
//! ```
//!
//! and exact equality stops matching the moment any caller adds context.
//! `format!("{context}: {e}")` appears ~800 times in `src/`, so this is the
//! common shape, not an exotic one.
//!
//! These tests drive the real reporting entry point
//! [`report_error_or_expected`] and assert on what it actually emits, rather
//! than only on the classifier's return value: the classification is a means,
//! and the observable contract is "this does not page".

use std::io;
use std::sync::{Arc, Mutex};

use openhuman_core::core::observability::{
    expected_error_kind, report_error_or_expected, ExpectedErrorKind,
};
use openhuman_core::openhuman::web3::wallet::WALLET_NOT_CONFIGURED_MESSAGE;

/// The exact wrapper `hosted/orchestration/schemas.rs` applies, reproduced from
/// the log line quoted in #5805:
///
/// ```text
/// ERR report_error [observability] rpc.invoke_method failed:
///     self_identity key_status: wallet is not configured; run wallet setup first
/// ```
fn wrapped_like_issue_5805() -> String {
    format!("self_identity key_status: {WALLET_NOT_CONFIGURED_MESSAGE}")
}

// ---------------------------------------------------------------------------
// tracing capture
// ---------------------------------------------------------------------------

/// Collects formatted `tracing` output so a test can assert on the level and
/// fields the reporting path actually emitted.
#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl Capture {
    fn contents(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().expect("capture buffer poisoned")).into_owned()
    }
}

impl io::Write for Capture {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .expect("capture buffer poisoned")
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Capture {
    type Writer = Capture;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// Run `body` with a capturing subscriber installed and return what it logged.
///
/// `with_default` is scoped to this thread, so tests stay independent even
/// though the test binary runs them in parallel.
fn capture_reporting<F: FnOnce()>(body: F) -> String {
    let capture = Capture::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(capture.clone())
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .finish();

    tracing::subscriber::with_default(subscriber, body);
    capture.contents()
}

// ---------------------------------------------------------------------------
// classification
// ---------------------------------------------------------------------------

/// The regression #5811 fixed: the wrapped form must classify as expected.
///
/// This is the assertion that fails if the `is_wallet_not_configured_message`
/// arm is removed from `expected_error_kind`.
#[test]
fn a_context_wrapped_wallet_state_is_classified_as_expected() {
    let wrapped = wrapped_like_issue_5805();

    assert_eq!(
        expected_error_kind(&wrapped),
        Some(ExpectedErrorKind::WalletNotConfigured),
        "the RPC layer wraps the wallet sentinel as `{{context}}: {{e}}` \
         (#5805); a classifier that only matches the bare message lets this \
         page. Message under test: {wrapped}"
    );
}

/// Wrapping must not have to be single-layer: nothing constrains how many
/// `format!("{context}: {e}")` hops an error takes before it is reported, and
/// a predicate that only tolerated one would regress the moment a caller
/// gained an intermediate layer.
#[test]
fn a_multiply_wrapped_wallet_state_is_still_classified_as_expected() {
    let nested = format!(
        "rpc.invoke_method failed: self_identity key_status: {WALLET_NOT_CONFIGURED_MESSAGE}"
    );

    assert_eq!(
        expected_error_kind(&nested),
        Some(ExpectedErrorKind::WalletNotConfigured),
        "demotion must survive arbitrary nesting depth, not just one wrapper"
    );
}

/// The bare sentinel — the shape a direct RPC produces — must stay demoted too.
#[test]
fn the_bare_wallet_sentinel_is_classified_as_expected() {
    assert_eq!(
        expected_error_kind(WALLET_NOT_CONFIGURED_MESSAGE),
        Some(ExpectedErrorKind::WalletNotConfigured),
    );
}

/// Guard against the matcher being too permissive.
///
/// The predicate is substring-based on purpose, which buys wrapper-tolerance at
/// the cost of blast radius. This pins the other side of that trade: a genuine
/// wallet failure — one that is a defect, not user-state — must still reach
/// Sentry. Without this, a future widening of the needle could silently demote
/// real failures and nothing would fail.
#[test]
fn a_genuine_wallet_failure_is_not_demoted() {
    for genuine in [
        "wallet signing failed: invalid nonce",
        "wallet keychain read failed: keyring access denied",
        "self_identity key_status: wallet is configured but the key is corrupt",
    ] {
        assert_eq!(
            expected_error_kind(genuine),
            None,
            "a real wallet defect must still page; demoting it would hide the \
             failures this classification exists to keep visible. Message: {genuine}"
        );
    }
}

// ---------------------------------------------------------------------------
// the observable reporting decision
// ---------------------------------------------------------------------------

/// The contract #5805 is about: this condition is logged as an expected state,
/// not as an error.
///
/// Asserted on the emitted record rather than on the classifier alone, because
/// "does not page" is the property the issue is about — a classification that
/// did not change what is emitted would fix nothing.
#[test]
fn reporting_a_wrapped_wallet_state_emits_info_and_not_error() {
    let wrapped = wrapped_like_issue_5805();

    let logged = capture_reporting(|| {
        report_error_or_expected(wrapped.as_str(), "rpc", "invoke_method", &[]);
    });

    assert!(
        logged.contains("INFO"),
        "an unconfigured wallet is expected user-state and must be recorded as \
         a breadcrumb at INFO. Captured output:\n{logged}"
    );
    assert!(
        !logged.contains("ERROR"),
        "the wrapped wallet state must not be reported at ERROR — that is the \
         55-events-in-72-minutes behaviour #5805 reported. Captured output:\n{logged}"
    );
    assert!(
        logged.contains("wallet_not_configured"),
        "the demotion should be attributable via its `kind` field so the \
         breadcrumb can still be correlated. Captured output:\n{logged}"
    );
}

/// A genuine wallet defect must still be reported at ERROR.
///
/// The contrast case for the test above: together they pin that the two are
/// routed differently, so a change that demoted everything would fail here
/// rather than passing both.
#[test]
fn reporting_a_genuine_wallet_failure_still_emits_error() {
    let logged = capture_reporting(|| {
        report_error_or_expected(
            "wallet signing failed: invalid nonce",
            "rpc",
            "invoke_method",
            &[],
        );
    });

    assert!(
        logged.contains("ERROR"),
        "a real wallet failure must still page. Captured output:\n{logged}"
    );
}
