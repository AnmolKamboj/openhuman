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
//! What this file re-exported used to split cleanly in two; only one half is
//! left, and the other is documented so nobody re-derives where it went:
//!
//! - **The taxonomy** (`FailureCode`, `FailureClass`, `PipelineFailure`,
//!   `DegradedState`, `classify_embed_error{,_str}`) is `tinycortex`'s; the
//!   engine crate only re-exported it. It follows the
//!   `memory::sync::sync_status` precedent and is named on `tinycortex`
//!   directly below — same items, so no call site changed and none has to when
//!   the engine crate leaves the build. `rpc_part_02.rs` / `rpc_part_03.rs`
//!   and [`report`] still take these from here, so this half is production
//!   surface and stays.
//! - **The engine half is gone.** The process-global degradation flags
//!   (`mark_*` / `clear_*` / `current_degraded_state`), the engine's `doctor`
//!   report and its `test_guard` rode a `pub use tinymemory_core::tree::
//!   health::*;` glob here. Production stopped reading it in #5560 — the
//!   doctor and the degradation snapshot are `MemoryMaintenance::{diagnose,
//!   degraded_state}`, served host-side by [`report`] — which left `test_guard`
//!   as its only consumer, from four test files. A production `pub use` that
//!   exists only to serve a test is exactly what keeps a crate in
//!   `[dependencies]`, so those tests name
//!   `tinymemory_core::tree::health::test_guard` directly now (the
//!   `[dev-dependencies]` entry serves them) and the glob is deleted.
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

/// The doctor report and the degradation snapshot, read from the bound driver
/// rather than from this process's copy of the engine.
///
/// Its [`DoctorReport`](report::DoctorReport) shadows nothing: it is reached as
/// `health::report::DoctorReport`, deliberately kept out of this module's own
/// namespace so that "the host's response type" and "the engine's same-named
/// struct" cannot be confused for one another at a call site.
pub mod report;

pub(crate) mod user_error;
