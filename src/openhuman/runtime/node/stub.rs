//! `runtime-node` disabled-build stub.
//!
//! Mirrors the *type* surface that always-compiled callers name, with no-op
//! behaviour. Only what is actually reached from outside the gate lives here —
//! the download/extract/resolve machinery is compiled out entirely.
//!
//! Why a stub rather than a leaf gate: [`NodeBootstrap`] appears in the **field
//! type** of `tools::impl::system::ShellTool` (`Option<Arc<NodeBootstrap>>`, for
//! managed-Node `PATH` injection), and `shell.rs` is kernel — always compiled.
//! Deleting the module would take the shell tool with it. The registration
//! sites (`node_runtime_step`, the `javascript` controllers, `node_exec` /
//! `npm_exec`) are leaf-gated at their call sites instead, because registration
//! sites want absence.
//!
//! Off-state: `try_cached` / `probe_installed` return `None`, so the shell
//! simply never prepends a managed `bin/` dir — the same path taken today when
//! `node.enabled = false`. `resolve()` is the one erroring method and is only
//! reachable from `harness_init`'s bootstrap step, itself gated off; it returns
//! a build fact so a stray caller reports something actionable.
//!
//! Note: this stub carries **only** the managed-Node toolchain type surface.
//! [`super::ops`] and [`super::types`] are not stubbed — they are the generic
//! native-tool dispatcher and its inert serde types, always compiled so the
//! ungated `flows` `NativeToolBackend` can keep dispatching `oh:*` tools when
//! the managed Node runtime is off.

use std::path::PathBuf;

use anyhow::Result;

use crate::openhuman::config::schema::NodeConfig;

/// Returned by [`NodeBootstrap::resolve`] in a `runtime-node`-less build.
/// Phrased as a build fact, matching the `mcp` / `tui` CLI-arm convention.
pub const RUNTIME_NODE_DISABLED_MESSAGE: &str =
    "runtime-node feature disabled at compile time — rebuild with `--features runtime-node` \
     to use the managed Node.js toolchain";

/// Origin of a resolved toolchain. Never produced here; kept so caller `match`
/// arms and imports still resolve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeSource {
    System,
    Managed,
}

/// Fully-resolved Node toolchain. Never constructed in a disabled build.
#[derive(Debug, Clone)]
pub struct ResolvedNode {
    pub bin_dir: PathBuf,
    pub node_bin: PathBuf,
    pub npm_bin: PathBuf,
    pub version: String,
    pub source: NodeSource,
}

/// Disabled-build bootstrap: constructs, resolves to nothing.
#[derive(Debug)]
pub struct NodeBootstrap {
    _config: NodeConfig,
    _workspace_dir: PathBuf,
}

impl NodeBootstrap {
    /// Signature-compatible with the real constructor. The `reqwest::Client` is
    /// accepted and dropped — a disabled build never downloads.
    pub fn new(config: NodeConfig, workspace_dir: PathBuf, _client: reqwest::Client) -> Self {
        Self {
            _config: config,
            _workspace_dir: workspace_dir,
        }
    }

    /// Always `None` — nothing is cached because nothing resolves.
    pub fn try_cached(&self) -> Option<ResolvedNode> {
        None
    }

    /// Always `None`. The real implementation probes the on-disk install; there
    /// is no install path in a disabled build.
    pub async fn probe_installed(&self) -> Option<ResolvedNode> {
        None
    }

    /// Always `Err`. See [`RUNTIME_NODE_DISABLED_MESSAGE`].
    pub async fn resolve(&self) -> Result<ResolvedNode> {
        anyhow::bail!(RUNTIME_NODE_DISABLED_MESSAGE)
    }
}
