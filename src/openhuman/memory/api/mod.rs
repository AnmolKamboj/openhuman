//! Stable public contracts for the TinyMemory memory system.
//!
//! This crate holds the value types, error enum, capability vocabulary, and
//! storage trait that memory engines and their embedding hosts compile
//! against. It is engine-neutral on purpose: `tinycortex` is the default
//! embedded engine, not the owner of the contract, and a second engine
//! (`supermemory`, `mem0`, a self-hosted HTTP backend) implements the same
//! traits without either engine learning about the other.
//! It is deliberately dependency-light (serde / serde_json /
//! chrono / sha2 / anyhow / thiserror / async-trait / uuid only) so depending on
//! the contract never drags in SQLite, git2, reqwest, regex, or an async
//! runtime.
//!
//! ## Self-contained by design
//!
//! Nothing here names a host type. A third-party memory driver must be able to
//! depend on this crate alone, and the *generic* subsystem/driver vocabulary of
//! the OpenHuman kernel (`Driver`, `DriverClass`, `SubsystemRegistry`, the
//! policy `Guard`) must not be inherited from a *memory* crate by whichever
//! subsystem is cut over next. So the contract carries its own identity,
//! capability, and health vocabulary, and the host's memory adapter converts at
//! the boundary — see [`health`] for the shape that conversion relies on.
//!
//! Driver *class* (embedded / external / null) is deliberately **absent**: that
//! is a host configuration fact about how a driver was bound, not something a
//! driver reports about itself.
//!
//! ## The TinyCortex engine's historical paths still resolve
//!
//! This contract used to live in the TinyCortex repository as `tinycortex-api`.
//! That crate is now a deprecated re-export of this one, and the engine crate
//! aliases these modules back into their historical paths
//! (`crate::openhuman::memory::engine::{types, error, traits}`,
//! `crate::openhuman::memory::engine::chunks::types`, `crate::openhuman::memory::engine::tree::runtime::types`,
//! `crate::openhuman::memory::engine::tool_memory::types`, `crate::openhuman::memory::engine::goals::types`),
//! so every existing path keeps resolving unchanged.
//!
//! ## Module map
//!
//! - [`types`]: pure data contracts (entries, hits, taint, namespaces).
//! - [`recall`]: the borrowed [`recall::RecallOpts`] and owned, serde-derived
//!   [`recall::OwnedRecallOpts`] recall filters (both re-exported from
//!   [`types`]).
//! - [`capabilities`]: the thirteen [`capabilities::Capability`] families and
//!   the [`capabilities::Capabilities`] set negotiated at bind time.
//! - [`provider`]: the driver contract — [`provider::MemoryProvider`] plus the
//!   thirteen capability family traits and the value types they need.
//! - [`null`]: [`null::NullMemoryProvider`], the reference driver a
//!   compiled-out or unconfigured memory subsystem binds to.
//! - [`health`]: [`health::MemoryHealth`], the liveness state a driver reports.
//! - [`version`]: [`CONTRACT_VERSION`] and the [`is_compatible`] bind rule.
//! - [`error`]: the typed [`error::MemoryError`] enum and its result alias.
//! - [`traits`]: the [`traits::Memory`] storage-backend trait.
//! - [`chunks`]: the persisted chunk model ([`chunks::Chunk`], [`chunks::Metadata`],
//!   [`chunks::SourceRef`], …) and the deterministic [`chunks::chunk_id`].
//! - [`tree`]: the markdown summary-tree node model ([`tree::TreeNode`],
//!   [`tree::NodeLevel`], [`tree::TreeStatus`], …).
//! - [`tool_memory`]: tool-scoped rule contracts ([`tool_memory::ToolMemoryRule`], …).
//! - [`goals`]: the long-term goals document ([`goals::GoalsDoc`], [`goals::GoalItem`]).
//! - [`host`]: the **host seam** — [`host::MemoryHostConfig`],
//!   [`host::EmbeddingProvider`], [`host::MemoryEventSink`], and the memory
//!   config sections whose serde form is persisted in a host's `config.toml`.
//! - [`wire`]: the error-name table a driver reached over a bus or a socket
//!   round-trips [`error::MemoryError`] through. Shared by both ends of every
//!   such transport, so the names cannot drift apart.

pub mod capabilities;
pub mod chunks;
pub mod error;
pub mod goals;
pub mod health;
pub mod host;
pub mod null;
pub mod provider;
pub mod recall;
pub mod tool_memory;
pub mod traits;
pub mod tree;
pub mod types;
pub mod version;
pub mod wire;

pub use version::{is_compatible, CONTRACT_VERSION};
