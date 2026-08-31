//! Host layer over the engine's people domain.
//!
//! The domain itself lives in the engine crate; what stays here is its
//! JSON-RPC surface — handlers and controller schemas name OpenHuman's
//! `RpcOutcome` and `ControllerSchema`, which the engine crate cannot see.
//! The glob re-export keeps every historical `memory::people::…` path resolving.
//!
//! # Why this names `tinycortex` rather than `tinymemory_core` (#5560)
//!
//! It used to read `pub use tinymemory_core::people::*;`, and that path was
//! itself a re-export: `tinymemory_core::people` is
//! `pub use crate::engine::backend::people::{address_book, migrations,
//! resolver, scorer, store, types};`, and `engine::backend::people` is
//! `pub use tinycortex::memory::people`. Both spellings therefore resolve to
//! the **same six items**; naming the engine crate directly changes no item,
//! only which crate alias holds them in the build.
//!
//! That matters because `tinymemory-core` is what #5560 is removing from the
//! production dependency graph, while `tinycortex` stays — it is a direct
//! dependency of this crate (`Cargo.toml`), it is where the memory engine
//! actually lives, and ~40 files here already name `tinycortex::memory::…`.
//! Same precedent as the `Memory` / `MemoryCategory` type re-exports in
//! `memory/mod.rs`, which were repointed at the contract for the same reason.
//!
//! **The `contacts` gate has to follow.** `address_book`'s macOS reader is
//! `#[cfg(all(target_os = "macos", feature = "contacts"))]` *inside tinycortex*,
//! and this crate reaches it today through `contacts = ["tinymemory-core/contacts"]`
//! → `tinymemory-core`'s `contacts = ["tinycortex/contacts"]`. When the
//! `tinymemory-core` normal dependency is dropped, that forward has to become
//! `contacts = ["tinycortex/contacts"]` directly, or the gate test below stops
//! testing anything.

pub use tinycortex::memory::people::*;

pub mod rpc;
pub mod schemas;

// The controller aggregators this domain's RPC surface defines. Aliased
// exactly as the pre-extraction module exported them.
pub use schemas::{
    all_controller_schemas as all_people_controller_schemas,
    all_registered_controllers as all_people_registered_controllers,
};

#[cfg(test)]
mod schemas_tests;

#[cfg(test)]
#[path = "mod_contacts_gate_tests_tests.rs"]
mod contacts_gate_tests;
