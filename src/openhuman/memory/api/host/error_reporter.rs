//! [`ErrorReporter`] — the host's crash/error telemetry, as the core sees it.
//!
//! The memory subsystem reports a handful of failures that are worth a
//! developer's attention: a corrupt SQLite database, host filesystem I/O
//! errors, a sync run that failed for a non-user reason. *Where* those go —
//! Sentry, a log sink, nowhere — and which of them count as expected rather
//! than exceptional is host policy, so the core states the fact and the host
//! decides what to do with it.
//!
//! # The two methods are not interchangeable
//!
//! [`ErrorReporter::report_error`] is unconditional: the caller has already
//! decided this is a real defect. [`ErrorReporter::report_error_or_expected`]
//! asks the host to classify first, so routine user- and config-caused failures
//! (an unreachable local runtime, a revoked OAuth token) do not page anyone.
//! Collapsing them into one would either spam the error channel or hide real
//! bugs, which is why both exist.

/// Receives error reports from the memory subsystem.
///
/// Takes the **already-rendered** message rather than a concrete error type:
/// the trait has to be object-safe, so it cannot be generic over `E: Display`
/// the way the host's own `report_error` is. The core's free functions keep
/// that generic signature and render with `{:#}` — the alternate specifier that
/// makes `anyhow::Error` print its full context chain — before crossing.
pub trait ErrorReporter: Send + Sync + std::fmt::Debug {
    /// Report `error` as a defect worth investigating.
    ///
    /// `domain` and `operation` are stable, low-cardinality strings used for
    /// grouping (`"memory"` / `"tree_jobs_worker_corrupt"`); `tags` carries
    /// additional non-sensitive key/value context.
    fn report_error(&self, rendered: &str, domain: &str, operation: &str, tags: &[(&str, &str)]);

    /// Report `error`, letting the host classify it as a defect or an expected
    /// user/config failure and route it accordingly.
    fn report_error_or_expected(
        &self,
        rendered: &str,
        domain: &str,
        operation: &str,
        tags: &[(&str, &str)],
    );
}
