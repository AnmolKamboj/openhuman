//! Tests for the memory module client.
//!
//! Nothing here loads a module. What is testable without one is what decides a
//! caller's next move: that construction is genuinely I/O-free, that the static
//! capability answer is the one the module actually serves, and that a bus error
//! comes back as the right `MemoryError` variant. The round trips are covered
//! where they can be honest — `tinymemory`'s own loader E2E, against a real
//! broker and a real `dlopen`.

use std::sync::Arc;

use crate::openhuman::memory::api::capabilities::{Capabilities, Capability};
use crate::openhuman::memory::api::error::MemoryError;
use crate::openhuman::memory::api::provider::MemoryProvider;

use super::{from_bus, ModuleMemoryProvider, MODULE_ID};
use crate::openhuman::config::Config;
use crate::openhuman::modules::registry;

fn provider() -> ModuleMemoryProvider {
    ModuleMemoryProvider::new(Arc::new(Config::default()))
}

/// A bus failure carrying `name`.
fn failure(name: &str) -> tinybus::Error {
    tinybus::Error::MethodFailed {
        name: name.to_string(),
        message: "something went wrong".to_string(),
    }
}

#[test]
fn construction_touches_no_io_and_needs_no_runtime() {
    // The load-bearing property of this type. `CoreContext::memory_binding` is
    // synchronous and roughly 4000 pre-boot tests call it with no tokio runtime,
    // so a constructor that loaded the module — or merely dialled the bus — would
    // panic across the whole suite rather than in one place.
    //
    // This test runs outside `#[tokio::test]` on purpose: that is what makes it a
    // test of the absence of a runtime requirement.
    let provider = provider();
    assert_eq!(provider.driver_id(), MODULE_ID);
}

#[test]
fn the_advertised_capabilities_cover_the_complete_memory_api() {
    // The compiled module owns the complete TinyMemory API, so the host can
    // assemble every memory RPC and tool family before the async bus starts.
    let capabilities = provider().capabilities();
    assert_eq!(capabilities, Capabilities::all());

    for mandatory in Capability::MANDATORY {
        assert!(capabilities.contains(mandatory), "{mandatory:?} is missing");
    }
    assert!(capabilities.contains(Capability::Tree));
}

#[test]
fn the_registry_record_matches_the_interface_the_module_serves() {
    // A record whose bus name or object path disagrees with the module produces a
    // proxy that resolves to nothing, and the failure surfaces as an unhelpful
    // transport error rather than a mismatch.
    let record = registry::find(MODULE_ID).expect("the memory module is registered");
    assert_eq!(record.bus_name, "ai.tinyhumans.tinymemory.Memory");
    assert_eq!(record.object_path, "/ai/tinyhumans/tinymemory/Memory");
}

#[test]
fn the_memory_record_publishes_one_asset_per_supported_host() {
    // The release exists now, so the question this test used to ask ("are the
    // assets deliberately absent?") is settled. What is worth pinning instead is
    // that the set is complete: a record missing a host silently reports
    // `Unsupported` there rather than failing loudly, so a platform can lose the
    // driver without anything saying so.
    //
    // The digests themselves are checked structurally by `registry`'s own tests
    // (lowercase, 64 hex chars) and semantically by tinybus, which refetches the
    // release manifest and refuses on disagreement. Nothing here can verify they
    // came from the release rather than a local build — that is a review rule,
    // and it is written on the record itself.
    let record = registry::find(MODULE_ID).expect("registered");
    assert_eq!(
        record.assets.len(),
        11,
        "expected one asset per released host, got {:?}",
        record.assets.iter().map(|a| a.host_key).collect::<Vec<_>>()
    );
    for asset in record.assets {
        assert!(
            asset.archive.contains(record.version),
            "{} names version-less or mismatched archive {}",
            asset.host_key,
            asset.archive
        );
    }
}

#[test]
fn a_not_found_survives_the_round_trip_as_not_found() {
    // `get`'s contract makes a missing entry `Ok(None)` and an `Invalid` a real
    // failure, so collapsing the two would be observable to a caller.
    let error = from_bus(&failure(crate::openhuman::memory::api::wire::NOT_FOUND));
    assert!(matches!(error, MemoryError::NotFound(_)), "{error:?}");
}

#[test]
fn an_invalid_input_is_reported_as_something_the_caller_can_fix() {
    let error = from_bus(&failure(crate::openhuman::memory::api::wire::INVALID));
    assert!(matches!(error, MemoryError::Invalid(_)), "{error:?}");
}

#[test]
fn a_path_escape_does_not_arrive_as_a_caller_mistake() {
    // The mapping's most security-relevant case: a sandbox escape must not be
    // reclassified as a malformed argument.
    let error = from_bus(&failure(crate::openhuman::memory::api::wire::PATH_ESCAPE));
    assert!(matches!(error, MemoryError::PathEscape(_)), "{error:?}");
}

#[test]
fn an_unsupported_capability_keeps_its_family_name() {
    let error = from_bus(&failure(crate::openhuman::memory::api::wire::UNSUPPORTED));
    assert!(
        matches!(error, MemoryError::Unsupported { .. }),
        "{error:?}"
    );
}

#[test]
fn an_unrecognised_wire_name_is_a_backend_failure_not_an_input_error() {
    // A module newer than this build may name an error the table lacks. Telling a
    // caller its input was wrong when it was not sends it into a rewrite loop over
    // something already correct.
    let error = from_bus(&failure("ai.tinyhumans.tinymemory.Error.SomethingNewer"));
    assert!(matches!(error, MemoryError::Other(_)), "{error:?}");
}

#[test]
fn a_missing_module_is_a_backend_failure_the_caller_cannot_fix() {
    let error = from_bus(&failure("ai.tinyhumans.tinybus.Error.ModuleUnavailable"));
    assert!(matches!(error, MemoryError::Other(_)), "{error:?}");
}

#[test]
fn the_debug_form_never_renders_the_config() {
    // `Config` carries credentials and `Debug` output reaches logs.
    let rendered = format!("{:?}", provider());
    assert!(rendered.contains("ModuleMemoryProvider"), "{rendered}");
    assert!(!rendered.contains("Config"), "{rendered}");
}

#[tokio::test]
async fn a_disabled_host_reports_down_rather_than_erroring() {
    // `health` is the one method whose job is to answer "is this reachable", so an
    // unreachable module is a `Down` health rather than a failure. Status output
    // depends on that distinction.
    let mut config = Config::default();
    config.modules.enabled = false;

    let provider = ModuleMemoryProvider::new(Arc::new(config));
    let health = provider.health().await;
    assert!(
        matches!(
            health,
            crate::openhuman::memory::api::health::MemoryHealth::Down { .. }
        ),
        "a disabled module host must report Down, got {health:?}"
    );
}

#[tokio::test]
async fn a_call_against_a_disabled_host_fails_instead_of_hanging() {
    let mut config = Config::default();
    config.modules.enabled = false;

    let provider = ModuleMemoryProvider::new(Arc::new(config));
    let outcome =
        crate::openhuman::memory::api::provider::mandatory::MemoryCore::get(&provider, "ns", "key")
            .await;
    assert!(outcome.is_err(), "expected an error, got {outcome:?}");
}

#[tokio::test]
async fn shutdown_on_an_unused_driver_is_a_no_op() {
    // A shutdown must not be the thing that downloads and loads a module. Nothing
    // has been used here, so there is nothing to release.
    let mut config = Config::default();
    config.modules.enabled = false;

    let provider = ModuleMemoryProvider::new(Arc::new(config));
    assert!(provider.shutdown().await.is_ok());
}
