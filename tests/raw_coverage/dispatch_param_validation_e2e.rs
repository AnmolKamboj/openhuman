//! What a caller actually observes when a dispatched call carries bad params.
//!
//! `core::all::validate_params` is the single pre-dispatch gate for every
//! registered controller, so a handler's own "missing required param" string can
//! never reach a caller. Plenty of handlers keep such a check anyway, worded
//! differently, and those are reachable only by invoking the handler directly —
//! which is exactly what most suites in this directory do. That makes it easy to
//! write a test that looks like it covers the dispatch path and does not: the
//! assertion passes on the handler's string while a real caller would have been
//! refused earlier, by a different message.
//!
//! These tests pin the observable behaviour so that the distinction cannot be
//! erased silently (#6073).

use serde_json::{json, Map};

use openhuman_core::core::all::{all_registered_controllers, rpc_method_name, schema_for_rpc_method};
use openhuman_core::core::dispatch::dispatch;
use openhuman_core::core::types::AppState;

fn state() -> AppState {
    AppState {
        core_version: "dispatch-param-validation-e2e".to_string(),
    }
}

/// The refusal a caller sees carries the *schema's* comment, not the handler's
/// wording — which is what makes the two impossible to mistake for each other.
#[tokio::test]
async fn missing_required_param_is_refused_with_the_schema_comment() {
    let schema = schema_for_rpc_method("openhuman.session_db_get")
        .expect("session_db_get is registered unconditionally");
    let comment = schema
        .inputs
        .iter()
        .find(|field| field.name == "id")
        .expect("`id` input")
        .comment;

    let err = dispatch(state(), "openhuman.session_db_get", json!({}))
        .await
        .expect_err("`id` is required");

    assert_eq!(err, format!("missing required param 'id': {comment}"));

    // `handle_session_db_get` refuses with `missing required param: id`. If that
    // ever starts matching, validation has moved into the handler and the shape
    // of what callers see has changed with it.
    assert!(!err.contains("missing required param: id"), "got: {err}");
}

/// A param that no schema declares is refused before dispatch. No handler
/// anywhere implements an unknown-param check, so this message can only come
/// from `validate_params` — it is the proof that the gate ran.
#[tokio::test]
async fn unknown_param_is_refused_by_the_gate_alone() {
    let err = dispatch(
        state(),
        "openhuman.memory_goals_list",
        json!({ "nonsense_param": 1 }),
    )
    .await
    .expect_err("unknown params are refused");

    assert_eq!(err, "unknown param 'nonsense_param' for memory_goals.list");
}

/// Declared types are enforced at the same gate, so a handler's own
/// `serde_json::from_value` error is not what a caller gets either.
#[tokio::test]
async fn mistyped_param_is_refused_with_the_declared_type() {
    let err = dispatch(
        state(),
        "openhuman.session_import_run",
        json!({ "dry_run": "yes" }),
    )
    .await
    .expect_err("`dry_run` is declared bool");

    assert_eq!(
        err,
        "invalid type for param 'dry_run' in session_import.run: expected bool, got string"
    );
    assert!(!err.contains("invalid params:"), "got: {err}");
}

/// The handler-side checks are real and they do fire — just never for a caller.
/// Reaching one takes a direct handler invocation, the way the suites in this
/// directory call controllers.
#[tokio::test]
async fn handler_checks_are_reachable_only_by_calling_the_handler_directly() {
    let dispatched = dispatch(state(), "openhuman.learning_get_facet", json!({}))
        .await
        .expect_err("`class` is required");
    assert!(
        dispatched.starts_with("missing required param 'class': "),
        "got: {dispatched}"
    );

    let controllers = all_registered_controllers();
    let controller = controllers
        .iter()
        .find(|controller| rpc_method_name(&controller.schema) == "openhuman.learning_get_facet")
        .expect("learning_get_facet is registered unconditionally");
    let direct = (controller.handler)(Map::new())
        .await
        .expect_err("the handler refuses too");

    assert_eq!(direct, "missing required `class`");
    assert_ne!(
        dispatched, direct,
        "the two refusals must stay distinguishable: a test written against the \
         handler's wording is testing the handler, not the dispatch path (#6073)"
    );
}
