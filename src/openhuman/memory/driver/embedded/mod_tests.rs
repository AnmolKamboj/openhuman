//! Identity + construction tests for the embedded driver.
//!
//! The load-bearing one is
//! `constructing_the_driver_does_no_io_and_needs_no_runtime`: it is a plain
//! `#[test]` on purpose. `MemoryClient::from_workspace_dir` spawns the
//! ingestion worker, so eagerly resolving the client would panic here — and
//! would panic identically in the ~4000 pre-boot unit tests that reach
//! `CoreContext::memory_binding` with no runtime.

use super::test_support::fresh_driver;
use super::*;

use tinycortex_api::capabilities::Capability;
use tinycortex_api::null::NULL_DRIVER_ID;
use tinycortex_api::provider::{audit_provider, MemoryCore};

#[test]
fn embedded_driver_id_is_tinycortex() {
    let (_tmp, provider) = fresh_driver();
    assert_eq!(provider.driver_id(), EMBEDDED_DRIVER_ID);
    assert_ne!(provider.driver_id(), NULL_DRIVER_ID);
}

#[test]
fn embedded_driver_advertises_the_mandatory_three() {
    let (_tmp, provider) = fresh_driver();
    let advertised = provider.capabilities();

    for mandatory in Capability::MANDATORY {
        assert!(
            advertised.contains(mandatory),
            "{mandatory} must be advertised"
        );
    }
}

#[test]
fn embedded_driver_advertises_documents_graph_and_tool_memory() {
    let (_tmp, provider) = fresh_driver();
    let advertised = provider.capabilities();

    for landed in [
        Capability::Documents,
        Capability::Graph,
        Capability::ToolMemory,
    ] {
        assert!(advertised.contains(landed), "{landed} landed in M3b");
    }
}

#[test]
fn embedded_driver_advertises_ingest_tree_and_entities() {
    let (_tmp, provider) = fresh_driver();
    let advertised = provider.capabilities();

    for landed in [Capability::Ingest, Capability::Tree, Capability::Entities] {
        assert!(advertised.contains(landed), "{landed} landed in M3c");
    }
}

#[test]
fn embedded_driver_advertises_diff_goals_sources_and_maintenance() {
    let (_tmp, provider) = fresh_driver();
    let advertised = provider.capabilities();

    for landed in [
        Capability::Diff,
        Capability::Goals,
        Capability::Sources,
        Capability::Maintenance,
    ] {
        assert!(advertised.contains(landed), "{landed} landed in M3d");
    }
}

/// The assertion the whole M3 milestone was aimed at.
#[test]
fn embedded_driver_advertises_every_capability() {
    let (_tmp, provider) = fresh_driver();
    assert_eq!(
        provider.capabilities(),
        Capabilities::all(),
        "a bound context must advertise the same thirteen families an unbound one does"
    );
    // Equality alone would still hold with every `as_*` returning `None`; that
    // is what `embedded_driver_passes_capability_audit` is for.
    for family in Capability::ALL {
        assert!(provider.capabilities().contains(family), "{family} missing");
    }
}

#[test]
fn bound_and_unbound_contexts_agree_on_the_capability_set() {
    use crate::openhuman::memory::binding;

    // The inversion this milestone existed to fix: before M3, binding a driver
    // *narrowed* the advertised set from thirteen families to three, so a bound
    // host looked less capable than an unbound one.
    let dir = tempfile::TempDir::new().unwrap();
    let binding = binding::for_workspace(
        dir.path(),
        &crate::openhuman::config::schema::MemorySubsystemConfig::default(),
    )
    .expect("default bind");

    assert_eq!(
        binding.capabilities(),
        binding::unbound_default_capabilities()
    );
}

#[test]
fn embedded_driver_passes_capability_audit() {
    let (_tmp, provider) = fresh_driver();
    assert!(
        audit_provider(&provider).is_ok(),
        "advertised set and reachable accessors disagree: {:?}",
        audit_provider(&provider).err()
    );
}

#[test]
fn constructing_the_driver_does_no_io_and_needs_no_runtime() {
    // Deliberately NOT `#[tokio::test]`, and deliberately a path that does not
    // exist: construction must neither spawn nor create anything.
    let tmp = tempfile::TempDir::new().unwrap();
    let workspace = tmp.path().join("never-created");

    let provider = EmbeddedMemoryProvider::new(workspace.clone(), MemoryHooksConfig::default());

    assert_eq!(provider.workspace_dir(), workspace.as_path());
    assert!(
        !workspace.exists(),
        "constructing the driver must not create the workspace"
    );
}

#[tokio::test]
async fn health_does_not_force_client_construction() {
    let tmp = tempfile::TempDir::new().unwrap();
    let workspace = tmp.path().join("never-created");
    let provider = EmbeddedMemoryProvider::new(workspace.clone(), MemoryHooksConfig::default());

    assert_eq!(provider.health().await, MemoryHealth::Ready);
    assert!(
        !workspace.exists(),
        "a health probe must not open (or create) the store"
    );
}

#[tokio::test]
async fn health_is_ready_once_the_client_is_resolved() {
    let (_tmp, provider) = fresh_driver();
    // Force resolution through a real contract call.
    provider.namespaces().await.expect("namespaces");

    assert_eq!(provider.health().await, MemoryHealth::Ready);
}

#[tokio::test]
async fn shutdown_is_a_no_op_and_is_idempotent() {
    let (_tmp, provider) = fresh_driver();
    provider.shutdown().await.expect("first shutdown");
    provider.shutdown().await.expect("second shutdown");
}

/// What the trait indirection actually costs, measured rather than asserted.
///
/// `docs/specs/plan-memory.md` §9 flags `#[async_trait]` dispatch as a
/// performance risk for the recall path. An end-to-end recall p50 cannot answer
/// that question — it is dominated by SQLite and, on the semantic path, by an
/// embedding call, both of which swamp a vtable hop by several orders of
/// magnitude. So this measures the *same* call twice, once statically and once
/// through `Arc<dyn MemoryProvider>`; the storage cost is identical in both arms
/// and only dispatch differs, so the delta is the indirection.
///
/// `#[ignore]`d and assertion-free on purpose: it prints, it does not gate.
///
/// ```text
/// GGML_NATIVE=OFF cargo test --lib \
///   openhuman::memory::driver::embedded::tests::trait_indirection_dispatch_cost \
///   -- --ignored --nocapture
/// ```
#[tokio::test]
#[ignore = "microbenchmark: prints ns/op, asserts nothing"]
async fn trait_indirection_dispatch_cost() {
    const ITERATIONS: u32 = 20_000;

    let (_tmp, provider) = fresh_driver();
    // Warm the lazy client so its one-time construction is not counted.
    provider.get("bench_ns", "absent").await.expect("warmup");

    let started = std::time::Instant::now();
    for _ in 0..ITERATIONS {
        let _ = provider.get("bench_ns", "absent").await;
    }
    let statik = started.elapsed();

    let dynamic_provider: Arc<dyn MemoryProvider> = Arc::new(provider);
    let started = std::time::Instant::now();
    for _ in 0..ITERATIONS {
        let _ = dynamic_provider.get("bench_ns", "absent").await;
    }
    let dynamic = started.elapsed();

    let per_call = |d: std::time::Duration| d.as_nanos() as f64 / f64::from(ITERATIONS);
    println!(
        "[bench] MemoryCore::get  static={:.0}ns/op  dyn={:.0}ns/op  delta={:+.0}ns/op",
        per_call(statik),
        per_call(dynamic),
        per_call(dynamic) - per_call(statik)
    );
}

#[test]
fn hooks_are_carried_through_from_config() {
    let tmp = tempfile::TempDir::new().unwrap();
    let hooks = MemoryHooksConfig {
        auto_recall: false,
        ..MemoryHooksConfig::default()
    };
    let provider = EmbeddedMemoryProvider::new(tmp.path().join("ws"), hooks);
    assert!(!provider.hooks().auto_recall);
}
