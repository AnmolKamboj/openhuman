//! Process-local inference overrides supplied by the standalone CLI.

mod ops;

pub(crate) use ops::{
    apply_cli_inference_overrides, restore_persisted_inference_fields, set_cli_inference_overrides,
};
// Public because it appears in `Config`'s public field list; the rest of this
// module stays crate-internal, so the module remains export-only as intended.
pub use ops::AppliedInferenceOverride;
