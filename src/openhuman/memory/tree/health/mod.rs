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
//! What the glob carries splits cleanly in two, which is why it cannot go yet
//! and equally cannot be half-repointed without saying so:
//!
//! - **The taxonomy** (`FailureCode`, `FailureClass`, `PipelineFailure`,
//!   `DegradedState`, `classify_embed_error{,_str}`) is `tinycortex`'s; the
//!   engine crate only re-exports it. These would follow the
//!   `memory::sync::sync_status` precedent and be named on `tinycortex`
//!   directly.
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

pub use tinymemory_core::tree::health::*;

pub(crate) mod user_error;
