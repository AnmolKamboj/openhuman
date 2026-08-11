//! Host-side wiring for workspace-backed conversation thread/message storage.
//!
//! Conversations are stored as JSONL files under the workspace (thread metadata
//! append-only in `threads.jsonl`; each thread's messages in a dedicated JSONL
//! file). The store / inverted-index / tokenizer / types engine is the crate's
//! (a byte-identical port, incl. the D1 rank-before-materialize fix), and
//! consumers name `tinycortex::memory::conversations` directly — this module no
//! longer re-exports that surface under a second path.
//!
//! Host-retained:
//! - [`bus`] — the `core::bus` persistence subscriber that bridges typed channel
//!   events onto the crate store (the crate abstracts the bus behind its own
//!   `ConversationEventBus` trait; the host wires the real one).
//! - [`blocking`] — `spawn_blocking` wrappers around the store's synchronous
//!   entry points. Request paths must use these, never the sync API (#5156).

pub mod blocking;
mod bus;

pub use bus::register_conversation_persistence_subscriber;
