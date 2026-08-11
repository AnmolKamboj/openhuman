//! Managed Node.js runtime and tool bridge.
//!
//! Responsibilities are split across submodules:
//!
//! * [`resolver`] — detect a compatible system `node` on `PATH`. Cheap,
//!   synchronous, called first so we can skip the download path when a
//!   matching toolchain already exists on the host.
//! * [`bootstrap`] / [`downloader`] / [`extractor`] — resolve or install the
//!   managed Node.js toolchain shipped with the core.
//! * [`ops`] / [`types`] — the generic runtime tool bridge: build / list /
//!   classify / execute against the native agent tool registry.
//! * [`schemas`] / [`rpc`] — the gated `javascript.*` controller pair.

//! ## Gating (`runtime-node`)
//!
//! Facade: this module is always declared, but only the *managed-Node*
//! machinery (`bootstrap` / `downloader` / `extractor` / `resolver` / `rpc` /
//! `schemas`) is `#[cfg(feature = "runtime-node")]`; a `stub` carries
//! `NodeBootstrap`'s type surface when the feature is off. The forcing
//! constraint is `ShellTool`, which holds `Option<Arc<NodeBootstrap>>` as a
//! field and is kernel — always compiled.
//!
//! [`ops`] and [`types`] are deliberately **not** gated. They are the generic
//! native-tool dispatcher over the agent tool registry (`oh:*` tools such as
//! `memory_search`, file, and shell tools) — they back both the gated
//! `javascript.*` controllers *and* the ungated `flows` `oh:` `NativeToolBackend`,
//! which must keep dispatching native tools even when the managed Node runtime
//! itself is compiled out. Only the JavaScript RPC and the Node-specific
//! `node_exec` / `npm_exec` tools are gated.

#[cfg(feature = "runtime-node")]
pub mod bootstrap;
#[cfg(feature = "runtime-node")]
pub mod downloader;
#[cfg(feature = "runtime-node")]
pub mod extractor;
#[cfg(feature = "runtime-node")]
pub mod resolver;
#[cfg(feature = "runtime-node")]
pub mod rpc;
#[cfg(feature = "runtime-node")]
mod schemas;

/// Generic runtime tool bridge. Always compiled — see module docs.
pub mod ops;
/// Inert serde types shared by [`ops`] and the gated JS RPC. Always compiled.
pub mod types;

#[cfg(not(feature = "runtime-node"))]
mod stub;
#[cfg(not(feature = "runtime-node"))]
pub use stub::{NodeBootstrap, NodeSource, ResolvedNode, RUNTIME_NODE_DISABLED_MESSAGE};

#[cfg(feature = "runtime-node")]
pub use bootstrap::{NodeBootstrap, NodeSource, ResolvedNode};
#[cfg(feature = "runtime-node")]
pub use downloader::{download_distribution, fetch_shasums, NodeDistribution};
#[cfg(feature = "runtime-node")]
pub use extractor::{atomic_install, extract_distribution};
pub use ops::{execute_tool, list_tools};
#[cfg(feature = "runtime-node")]
pub use resolver::{detect_system_node, parse_node_version, SystemNode};
#[cfg(feature = "runtime-node")]
pub use schemas::{
    all_controller_schemas as all_runtime_node_controller_schemas,
    all_registered_controllers as all_runtime_node_registered_controllers,
};
pub use types::ExecuteToolOutcome;
