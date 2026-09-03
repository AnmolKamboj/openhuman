//! Split off memory_tests.rs to stay under the repo's line-count gate.
//! Same module, same imports — see memory_tests.rs for what this covers.
use super::*;

/// The runtime-tree and flavour doors, driven against a **real** module.
///
/// The test above proves the `module_call!` arms exist by discriminating
/// `Other` from `Unsupported` against a disabled host; it cannot prove the wire
/// names are right, because a mistyped one fails the same way a disabled host
/// does. This one can: it loads an actual artifact and asserts the answers.
///
/// # What it deliberately does not drive
///
/// `runtime_summarize` and `runtime_rebuild` resolve a chat model on the
/// driver's side and then spend on it. A test that called them would either
/// reach the network or assert against a provider-resolution failure, and
/// neither says anything about the door. The five below are store-shaped and
/// answer from a fresh workspace with no ambiguity: a buffered write reports
/// where it landed, an empty tree has no root and no children, its status is
/// all zeroes, and nothing has been distilled for a persona scope.
///
/// Run it against a locally built module, one test per process:
///
/// ```text
/// TINYMEMORY_TEST_MODULE=/path/to/libtinymemory_module.dylib \
///   cargo test --lib -- --ignored --exact --test-threads=1 \
///   openhuman::modules::memory::tests::the_runtime_tree_doors_round_trip_through_a_real_module
/// ```
#[tokio::test]
#[ignore = "needs a built tinymemory module (TINYMEMORY_TEST_MODULE) and its own process: \
the bus belongs to whichever runtime creates it, so a second module-loading test in the same \
process finds a broker whose tasks are already gone and hangs rather than failing"]
async fn the_runtime_tree_doors_round_trip_through_a_real_module() {
    let module = std::env::var_os("TINYMEMORY_TEST_MODULE")
        .expect("set TINYMEMORY_TEST_MODULE to a built libtinymemory_module cdylib");
    let workspace = tempfile::TempDir::new().expect("tempdir");

    let mut config = Config::default();
    config.workspace_dir = workspace.path().to_path_buf();
    config.modules.enabled = true;
    config.modules.install_dir = Some(
        workspace
            .path()
            .join("modules")
            .to_string_lossy()
            .into_owned(),
    );
    config
        .modules
        .overrides
        .push(crate::openhuman::config::schema::ModuleOverride {
            id: MODULE_ID.to_string(),
            path: module.to_string_lossy().into_owned(),
        });

    let provider = ModuleMemoryProvider::new(Arc::new(config));
    let tree = provider.as_tree().expect("the Tree family");
    let at = chrono::Utc::now();

    let path = tree
        .runtime_buffer_write("team", "standup notes", at, None)
        .await
        .expect("RuntimeBufferWrite must reach the module");
    assert!(
        !path.trim().is_empty(),
        "the buffered write reports where it landed"
    );

    assert!(
        tree.runtime_read_node("team", "root")
            .await
            .expect("RuntimeReadNode must reach the module")
            .is_none(),
        "a buffered write creates no nodes; absence is data, not an error"
    );
    assert!(
        tree.runtime_read_children("team", "root")
            .await
            .expect("RuntimeReadChildren must reach the module")
            .is_empty(),
        "a parent that does not exist has no children"
    );

    let status = tree
        .runtime_tree_status("team")
        .await
        .expect("RuntimeTreeStatus must reach the module");
    assert_eq!(status.namespace, "team");
    assert_eq!(status.total_nodes, 0);
    assert_eq!(status.depth, 0);

    assert!(
        tree.flavour_profile("persona/communication")
            .await
            .expect("FlavourProfile must reach the module")
            .is_none(),
        "nothing has been distilled for this scope yet"
    );

    // The two refusals the doors make before touching the store, so a wrong
    // wire name cannot pass this test by answering plausibly to everything.
    let rejected = tree
        .runtime_buffer_write("../escape", "x", at, None)
        .await
        .expect_err("a traversal namespace is refused");
    assert!(
        matches!(rejected, MemoryError::Invalid(_)),
        "a rejected namespace is a caller mistake, not a backend failure: {rejected:?}"
    );
    let blank = tree
        .runtime_buffer_write("team", "   ", at, None)
        .await
        .expect_err("blank content is refused");
    assert!(
        matches!(blank, MemoryError::Invalid(_)),
        "blank content is a caller mistake: {blank:?}"
    );
}

#[test]
fn scoring_is_advertised_and_has_a_host_accessor() {
    // tinymemory v1.13.2 (tinymemory#110) added the family; advertising it and
    // forwarding it must land together, or the driver claims a family whose
    // accessor answers `None` — the #5598 over-claim in miniature.
    let mut config = Config::default();
    config.modules.enabled = false;
    let provider = ModuleMemoryProvider::new(Arc::new(config));
    assert!(super::super::capabilities_for(false).contains(Capability::Scoring));
    assert!(
        provider.as_scoring().is_some(),
        "Scoring is advertised, so the accessor must be wired"
    );
}
