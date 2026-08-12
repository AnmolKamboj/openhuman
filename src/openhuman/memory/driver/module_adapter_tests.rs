//! The serde crossings are the risk this adapter takes on, so these tests are
//! about the crossings rather than about the forwarding.

use super::{cross, error_to_tinycortex};

#[test]
fn round_trip_preserves_every_field() {
    // This is the test the module docs point at. A serde round trip tolerates
    // drift where exhaustive destructuring would catch it, so the trade is only
    // acceptable while something checks that values still survive the crossing.
    //
    // Each case is populated with non-default values in every field it has, so a
    // dropped field shows up as an inequality rather than as two defaults that
    // happen to match. `..Default::default()` is deliberately not used here:
    // a field left on its default would round-trip back to that same default
    // even if the crossing dropped it, silently passing.
    let opts = tinycortex_api::recall::OwnedRecallOpts {
        namespace: Some("work".to_string()),
        category: Some(tinycortex_api::types::MemoryCategory::Core),
        session_id: Some("session-9".to_string()),
        min_score: Some(0.42),
        cross_session: true,
    };
    let crossed: tinymemory_api::recall::OwnedRecallOpts =
        cross(&opts, "recall options").expect("recall options cross");
    let back: tinycortex_api::recall::OwnedRecallOpts =
        cross(&crossed, "recall options").expect("recall options cross back");
    assert_eq!(back, opts, "recall options lost a field crossing contracts");
}

#[test]
fn an_export_page_survives_the_crossing_with_its_cursor() {
    // `next_cursor` is the terminator a caller keys on — an empty `records` is
    // explicitly not one — so losing it would turn a paged export into an
    // infinite loop or a silent truncation.
    let page = tinycortex_api::provider::types::ExportPage {
        records: Vec::new(),
        next_cursor: Some("cursor-42".to_string()),
    };
    let crossed: tinymemory_api::provider::types::ExportPage =
        cross(&page, "export page").expect("cross");
    assert_eq!(crossed.next_cursor.as_deref(), Some("cursor-42"));

    let back: tinycortex_api::provider::types::ExportPage =
        cross(&crossed, "export page").expect("cross back");
    assert_eq!(back, page);
}

#[test]
fn an_export_record_survives_the_crossing_with_its_payload_and_taint() {
    // `taint` and `payload` are exactly the fields a silent drop would corrupt
    // worst: a dropped taint would let externally-sourced content re-enter as
    // internal-trust, and a dropped payload would import an empty record.
    let record = tinycortex_api::provider::types::ExportRecord {
        kind: "entry".to_string(),
        id: "entry-7".to_string(),
        namespace: Some("work".to_string()),
        taint: tinycortex_api::types::MemoryTaint::ExternalSync,
        payload: serde_json::json!({"key": "value", "n": 3}),
    };
    let crossed: tinymemory_api::provider::types::ExportRecord =
        cross(&record, "export record").expect("cross");
    let back: tinycortex_api::provider::types::ExportRecord =
        cross(&crossed, "export record").expect("cross back");
    assert_eq!(back, record, "export record lost a field crossing contracts");
}

#[test]
fn an_import_outcome_survives_the_crossing_with_its_counts_and_errors() {
    // `errors` is operator-facing diagnosis; a dropped value here would leave
    // a non-zero `failed` count with nothing to explain it.
    let outcome = tinycortex_api::provider::types::ImportOutcome {
        imported: 4,
        skipped: 2,
        failed: 1,
        errors: vec!["record 'x' rejected: bad payload".to_string()],
    };
    let crossed: tinymemory_api::provider::types::ImportOutcome =
        cross(&outcome, "import outcome").expect("cross");
    let back: tinycortex_api::provider::types::ImportOutcome =
        cross(&crossed, "import outcome").expect("cross back");
    assert_eq!(
        back, outcome,
        "import outcome lost a field crossing contracts"
    );
}

#[test]
fn a_source_scope_survives_the_crossing_with_every_allowed_source() {
    // An empty scope is fail-closed (denies all source-attributed content, per
    // the type's own docs), so losing an entry here would silently widen what
    // a recall call is allowed to see.
    let scope = tinycortex_api::provider::types::SourceScope::new(["src-a", "src-b"]);
    let crossed: tinymemory_api::provider::types::SourceScope =
        cross(&scope, "source scope").expect("cross");
    let back: tinycortex_api::provider::types::SourceScope =
        cross(&crossed, "source scope").expect("cross back");
    assert_eq!(back, scope, "source scope lost an entry crossing contracts");
}

#[test]
fn capabilities_cross_as_family_names_not_bit_layouts() {
    // Both sides serialize a capability set as a JSON array of family names, so
    // the crossing is over stable wire strings. If it went over the bitset an
    // added family on one side would silently shift every bit above it.
    let capabilities = tinymemory_api::capabilities::Capabilities::mandatory();
    let crossed: tinycortex_api::capabilities::Capabilities =
        cross(&capabilities, "capabilities").expect("cross");

    for mandatory in tinycortex_api::capabilities::Capability::MANDATORY {
        assert!(
            crossed.contains(mandatory),
            "{mandatory:?} was lost crossing contracts"
        );
    }
}

#[test]
fn a_not_found_stays_not_found() {
    // A host re-raises these to its own callers, and `get`'s contract makes a
    // miss `Ok(None)` while an `Invalid` is a real failure — so collapsing the
    // two is observable.
    let error = error_to_tinycortex(tinymemory_api::error::MemoryError::NotFound(
        "absent".to_string(),
    ));
    assert!(
        matches!(error, tinycortex_api::error::MemoryError::NotFound(_)),
        "{error:?}"
    );
}

#[test]
fn a_path_escape_does_not_become_a_caller_mistake() {
    // The security-relevant one: a sandbox escape must not be reclassified as a
    // malformed argument.
    let error = error_to_tinycortex(tinymemory_api::error::MemoryError::PathEscape(
        "symlink leaves workspace".to_string(),
    ));
    assert!(
        matches!(error, tinycortex_api::error::MemoryError::PathEscape(_)),
        "{error:?}"
    );
}

#[test]
fn an_unsupported_capability_keeps_its_family_name() {
    let error = error_to_tinycortex(tinymemory_api::error::MemoryError::unsupported_raw(
        "vendor_extension",
    ));
    match error {
        tinycortex_api::error::MemoryError::Unsupported { capability } => {
            assert_eq!(capability, "vendor_extension");
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

#[test]
fn taint_and_category_do_not_go_through_serde() {
    // Guards the deliberate exception. Taint decides whether externally-sourced
    // content is treated as internal-trust content, so it uses the audited
    // destructuring conversion in `tinymemory_tinycortex::convert` rather than a
    // round trip that would tolerate an added variant.
    use tinymemory_tinycortex::convert;

    for taint in [
        tinycortex_api::types::MemoryTaint::Internal,
        tinycortex_api::types::MemoryTaint::ExternalSync,
    ] {
        let crossed = convert::taint_to_tinymemory(taint);
        assert_eq!(convert::taint_to_tinycortex(crossed), taint);
    }

    let category = tinycortex_api::types::MemoryCategory::Custom("notes".to_string());
    let crossed = convert::category_to_tinymemory(category.clone());
    assert_eq!(convert::category_to_tinycortex(crossed), category);
}
