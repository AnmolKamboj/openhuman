//! Memory-driver implementations of the [`tinycortex_api`] contract.
//!
//! One subdirectory per driver. Today there is exactly one — [`embedded`],
//! which wraps the in-process tinycortex engine — plus the reference
//! `NullMemoryProvider` that ships inside the contract crate itself.
//!
//! Drivers live *under* `memory/` rather than in a sibling top-level directory
//! so the "one directory equals one feature gate" family rule holds: a memory
//! driver is memory, and gating it separately from the domain it implements
//! would be meaningless.

pub mod embedded;
pub mod module_adapter;
