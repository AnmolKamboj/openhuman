//! The name table is a contract, so these tests pin it rather than exercise it.

use super::{from_wire, wire_message, wire_name};
use crate::openhuman::memory::api::capabilities::Capability;
use crate::openhuman::memory::api::error::MemoryError;

/// Every variant, so a new one fails to compile in `wire_name` and fails here.
fn every_variant() -> Vec<MemoryError> {
    vec![
        MemoryError::NotFound("thread-7".to_string()),
        MemoryError::Invalid("limit must be positive".to_string()),
        MemoryError::BudgetExceeded("depth 12 exceeds 8".to_string()),
        MemoryError::PathEscape("symlink leaves workspace".to_string()),
        MemoryError::Io(std::io::Error::other("disk gone")),
        MemoryError::Serde(serde_json::from_str::<u8>("nope").unwrap_err()),
        MemoryError::unsupported(Capability::Tree),
        MemoryError::Other(anyhow::anyhow!("engine stopped")),
    ]
}

#[test]
fn round_trips_every_variant() {
    for error in every_variant() {
        let name = wire_name(&error);
        let message = wire_message(&error);
        let rebuilt = from_wire(name, &message);

        // Io and Serde deliberately degrade to `Other`: neither foreign error
        // type can be reconstructed from a string. Everything else must come
        // back as the same variant, because a host re-raises it to its own
        // callers and the variant is what they match on.
        match (&error, &rebuilt) {
            (MemoryError::Io(_) | MemoryError::Serde(_), MemoryError::Other(_)) => {}
            _ => assert_eq!(
                std::mem::discriminant(&error),
                std::mem::discriminant(&rebuilt),
                "{name} did not round-trip to the same variant"
            ),
        }
        assert!(
            rebuilt.to_string().contains(message.trim()) || message.is_empty(),
            "{name} lost its message: {rebuilt}"
        );
    }
}

#[test]
fn every_name_is_distinct() {
    let mut names: Vec<&str> = every_variant().iter().map(wire_name).collect();
    let before = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(before, names.len(), "two variants share a wire name");
}

#[test]
fn an_unrecognised_name_is_a_backend_failure_not_an_input_error() {
    // The load-bearing case. A driver newer than this build names something we
    // have no variant for; classifying it as `Invalid` would tell a caller its
    // request was wrong and send it into a rewrite loop.
    let rebuilt = from_wire("ai.tinyhumans.tinymemory.Error.SomethingNewer", "hmm");
    assert!(matches!(rebuilt, MemoryError::Other(_)), "{rebuilt:?}");
}

#[test]
fn a_path_escape_does_not_collapse_onto_invalid() {
    // These were nearly given one shared name. A sandbox escape is not a
    // malformed argument, and a host may log or refuse to retry it differently.
    assert_ne!(
        wire_name(&MemoryError::PathEscape("x".to_string())),
        wire_name(&MemoryError::Invalid("x".to_string()))
    );
}

#[test]
fn a_missing_entry_stays_not_found() {
    // `get`'s contract makes a missing entry `Ok(None)` and an `Invalid` a real
    // failure, so conflating the two is observable to a caller.
    let rebuilt = from_wire(super::NOT_FOUND, "absent");
    assert!(matches!(rebuilt, MemoryError::NotFound(_)), "{rebuilt:?}");
}

#[test]
fn an_unsupported_capability_keeps_its_family_name() {
    let error = MemoryError::unsupported(Capability::Diff);
    let rebuilt = from_wire(wire_name(&error), &wire_message(&error));
    match rebuilt {
        MemoryError::Unsupported { capability } => {
            assert_eq!(capability, Capability::Diff.as_str());
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

#[test]
fn an_unknown_capability_name_off_the_wire_survives() {
    // A driver on a newer minor contract may name a family this build has no
    // `Capability` for. It must not be dropped or fail to parse.
    let rebuilt = from_wire(super::UNSUPPORTED, "vendor_extension");
    match rebuilt {
        MemoryError::Unsupported { capability } => assert_eq!(capability, "vendor_extension"),
        other => panic!("expected Unsupported, got {other:?}"),
    }
}
