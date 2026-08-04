//! Managed Node.js runtime and tool bridge.
//!
//! Responsibilities are split across submodules:
//!
//! * [`resolver`] — detect a compatible system `node` on `PATH`. Cheap,
//!   synchronous, called first so we can skip the download path when a
//!   matching toolchain already exists on the host.
//! * [`bootstrap`] / [`downloader`] / [`extractor`] — resolve or install the
//!   managed Node.js toolchain shipped with the core.
//! * [`ops`] / [`schemas`] — expose a minimal top-level runtime surface for
//!   listing agent-callable tools and dispatching a tool by name.

//! ## Gating (`runtime-node`)
//!
//! Facade: this module is always declared, but every submodule below is
//! `#[cfg(feature = "runtime-node")]` and a `stub` takes over when the feature
//! is off. The forcing constraint is `ShellTool`, which holds
//! `Option<Arc<NodeBootstrap>>` as a field and is kernel — always compiled.

#[cfg(feature = "runtime-node")]
pub mod bootstrap;
#[cfg(feature = "runtime-node")]
pub mod downloader;
#[cfg(feature = "runtime-node")]
pub mod extractor;
#[cfg(feature = "runtime-node")]
pub mod ops;
#[cfg(feature = "runtime-node")]
pub mod resolver;
#[cfg(feature = "runtime-node")]
pub mod rpc;
#[cfg(feature = "runtime-node")]
mod schemas;
#[cfg(feature = "runtime-node")]
pub mod types;

#[cfg(not(feature = "runtime-node"))]
mod stub;
#[cfg(not(feature = "runtime-node"))]
pub use stub::{
    ops, ExecuteToolOutcome, NodeBootstrap, NodeSource, ResolvedNode, RUNTIME_NODE_DISABLED_MESSAGE,
};

#[cfg(feature = "runtime-node")]
pub use bootstrap::{NodeBootstrap, NodeSource, ResolvedNode};
#[cfg(feature = "runtime-node")]
pub use downloader::{download_distribution, fetch_shasums, NodeDistribution};
#[cfg(feature = "runtime-node")]
pub use extractor::{atomic_install, extract_distribution};
#[cfg(feature = "runtime-node")]
pub use ops::{execute_tool, list_tools};
#[cfg(feature = "runtime-node")]
pub use resolver::{detect_system_node, parse_node_version, SystemNode};
#[cfg(feature = "runtime-node")]
pub use schemas::{
    all_controller_schemas as all_runtime_node_controller_schemas,
    all_registered_controllers as all_runtime_node_registered_controllers,
};
#[cfg(feature = "runtime-node")]
pub use types::ExecuteToolOutcome;
