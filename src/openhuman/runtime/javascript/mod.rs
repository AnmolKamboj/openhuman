//! First-class JavaScript runtime surface.
//!
//! Today the implementation backend is the managed Node.js runtime in
//! [`crate::openhuman::runtime::node`]. This module exists so the rest of the
//! core talks to a language slot (`javascript`) rather than directly to a
//! specific backend. That keeps the door open for future sibling modules like
//! `python`, `ruby`, or a different JavaScript backend.

//! ## Gating (`runtime-node`)
//!
//! The facade itself is always compiled — `ShellTool` imports `NodeBootstrap`
//! through it — but the re-exports split. The bootstrap type surface comes from
//! `node`'s stub when the feature is off; the download/extract/dispatch
//! machinery and the controller pair are gated, because their only consumers
//! are themselves gated off.

pub use crate::openhuman::runtime::node::{
    ExecuteToolOutcome, NodeBootstrap, NodeSource, ResolvedNode,
};

#[cfg(feature = "runtime-node")]
pub use crate::openhuman::runtime::node::types::RuntimeToolSummary;
#[cfg(feature = "runtime-node")]
pub use crate::openhuman::runtime::node::{
    all_runtime_node_controller_schemas as all_javascript_controller_schemas,
    all_runtime_node_registered_controllers as all_javascript_registered_controllers,
};
#[cfg(feature = "runtime-node")]
pub use crate::openhuman::runtime::node::{
    atomic_install, detect_system_node, download_distribution, execute_tool, extract_distribution,
    fetch_shasums, list_tools, parse_node_version, NodeDistribution, SystemNode,
};
