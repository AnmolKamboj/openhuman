//! Host layer over [`tinymemory_core::tree::health`].
//!
//! The health model and its classifier are core; what stays here is the
//! `user_error` wire payload, which rides a web channel.
//!
//! # Do not repoint this at `memory::api::health` — it is a different thing (#5560)
//!
//! The name collides and the types do not. `tinymemory_api::health` is
//! `MemoryHealth`, driver *liveness*, one enum. This module is pipeline
//! *failure classification* — `FailureCode`, `FailureClass`, `PipelineFailure`,
//! `DegradedState` — and `tinycortex::memory::health`'s own docs call the
//! confusion out by name. A swap would compile at neither end.
//!
//! What this file re-exports splits cleanly in two, and the two halves are now
//! named apart rather than mixed into one `pub use`:
//!
//! - **The taxonomy** (`FailureCode`, `FailureClass`, `PipelineFailure`,
//!   `DegradedState`, `classify_embed_error{,_str}`) is `tinycortex`'s; the
//!   engine crate only re-exported it. It follows the
//!   `memory::sync::sync_status` precedent and is named on `tinycortex`
//!   directly below — same items, so no call site changed and none has to when
//!   the engine crate leaves the build.
//! - **The process-global degradation flags** (`mark_*` / `clear_*` /
//!   `current_degraded_state`), the `doctor` report and its `test_guard` are
//!   defined *in* `tinymemory-core`. They have no home to be repointed at, so
//!   they are what actually pins this file — and the reason mixing the two
//!   halves into one `pub use` line hides which is which.
//!
//! Worth knowing before trusting a reading of `current_degraded_state()`: the
//! flags are process statics, and the loaded module links its **own** copy of
//! `tinymemory-core`. A degradation the module observes never reaches the
//! statics this host reads. That is pre-existing and is not #5560's to fix, but
//! it is the reason moving the flags host-side is a design question rather than
//! a file move.

// The taxonomy half, named at the engine that **stays**. `tinycortex` is a
// direct dependency of this crate and is where these are defined
// (`tinycortex::memory::health`); `tinymemory_core::tree::health` re-exported
// them out of its own `engine::backend::health`, which is the same
// `pub use tinycortex::memory::health`. So this changes no item and no wire
// byte — an explicit `use` shadows the glob below with the identical item, the
// way `tree_runtime` already does for the contract's node model — and it
// records which half of this shim survives `tinymemory-core` leaving the build.
// Same precedent as `memory::people`, `memory::tool_memory` and
// `memory::sync::sync_status`.
pub use tinycortex::memory::health::{
    classify_embed_error, classify_embed_error_str, DegradedState, FailureClass, FailureCode,
    PipelineFailure,
};

// The half that genuinely pins the engine crate: the process-global
// degradation flags (`mark_*` / `clear_*` / `current_degraded_state`, plus the
// `test_guard` that serialises tests touching them) and the engine's own
// `doctor` report, all *defined in* `tinymemory-core`. Left as a glob
// deliberately — narrowing it would enumerate the flag set here and go stale
// the first time one is added, and it carries nothing else.
//
// **Nothing in production reads this glob any more.** `current_degraded_state`
// and `async_run_doctor` were its two live callers and both went to the driver
// in #5560 — see [`report`]. What is left is `test_guard`, which four test
// files use to serialise the process statics, and the flag setters those tests
// drive. So this line is now a test-only pin, and it goes when those tests stop
// needing an in-process engine to mark a flag on.
pub use tinymemory_core::tree::health::*;

/// The doctor report and the degradation snapshot, read from the bound driver
/// rather than from this process's copy of the engine.
///
/// Its [`DoctorReport`](report::DoctorReport) shadows nothing: it is reached as
/// `health::report::DoctorReport`, deliberately kept out of this module's own
/// namespace so that "the host's response type" and "the engine's struct the
/// glob above still carries" cannot be confused for one another at a call site.
pub mod report;

pub(crate) mod user_error;
