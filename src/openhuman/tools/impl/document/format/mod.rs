//! Agent-friendly document synthesis and text extraction in Rust.
//!
//! Typed, validated document contracts shared with the document bus module.
//! They are built for hosts that let a language model produce documents:
//! the spec types are the JSON tool schema, validation rejects a malformed
//! spec with a structured [`Error::InvalidInput`] naming the exact field so
//! the model can self-correct, and synthesis returns a plain byte buffer.
//!
//! # What this module deliberately does not do
//!
//! No filesystem access, no subprocesses, no async runtime, no deadline
//! handling. Synthesis runs in the document bus module; this host module owns
//! the wire contract and validation only.
//!
//! # Layout
//!
//! - [`error`](self::Error) — the crate-wide [`Error`] and [`Result`].
//! - [`spec`] — the typed document specs and their validation. Compiled in
//!   every build, including `--no-default-features`, so a host whose synthesis
//!   happens elsewhere still shares one definition of the wire contract.
//!
//! Writer and extractor implementations are intentionally absent: the host
//! sends these contract values over TinyBus.

mod error;

pub mod spec;

pub use error::{Error, Result};
