//! #6112 regression guard: no aggregated suite may send a hard-coded bearer.
//!
//! `core::auth::RPC_TOKEN` is a process-global `OnceLock` and `init_rpc_token`
//! returns early once it is set — deliberately, so a second call cannot 401 live
//! clients. Since `tests/raw_coverage/` is one binary, only the first suite to
//! initialise it pins its own `TEST_RPC_TOKEN`; any suite that then sends its own
//! literal is answered `401` and trips its own `assert_eq!(status, OK)`. Which
//! suites fail depends on libtest scheduling, so the failure is load-dependent
//! and invisible to CI, which runs one process per module filter.
//!
//! The fix is for every suite to send `get_rpc_token()` — the token the process
//! actually validates — rather than the literal it hoped to install. This scans
//! the sibling sources for the literal form so a *new* suite cannot reintroduce
//! it, which a runtime assertion in one file could not do.

use std::path::Path;

/// Substrings that mean "this suite sends its own literal".
const FORBIDDEN: &[&str] = &[
    "bearer_auth(TEST_RPC_TOKEN)",
    "Bearer {TEST_RPC_TOKEN}",
];

#[test]
fn no_raw_coverage_suite_sends_a_hard_coded_bearer() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/raw_coverage");
    let mut offenders = Vec::new();
    let mut scanned = 0usize;

    for entry in std::fs::read_dir(&dir).expect("tests/raw_coverage must be readable") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        if path.file_name().and_then(|n| n.to_str()) == Some("rpc_bearer_hygiene_e2e.rs") {
            continue; // this file names the literals on purpose
        }
        let source = std::fs::read_to_string(&path).expect("suite source must be readable");
        scanned += 1;
        for needle in FORBIDDEN {
            if source.contains(needle) {
                offenders.push(format!(
                    "{}: sends {needle} — use the process token instead",
                    path.file_name().and_then(|n| n.to_str()).unwrap_or("?")
                ));
            }
        }
    }

    // An empty scan is a failure, not a pass: if the glob ever stops matching,
    // this test would otherwise report clean having read nothing.
    assert!(
        scanned > 10,
        "expected to scan the raw_coverage suites, only saw {scanned} file(s) in {}",
        dir.display()
    );
    assert!(
        offenders.is_empty(),
        "raw_coverage suites must send `core::auth::get_rpc_token()`, not their own \
         `TEST_RPC_TOKEN` literal — only the first suite in the process wins that race \
         (#6112):\n  {}",
        offenders.join("\n  ")
    );
}
