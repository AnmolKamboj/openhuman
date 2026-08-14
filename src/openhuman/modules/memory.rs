//! The host half of `ai.tinyhumans.tinymemory.Memory`.
//!
//! [`ModuleMemoryProvider`] implements `MemoryProvider` by forwarding each method
//! to the loaded module. Because the wire surface mirrors the trait one method
//! for one method, there is no translation layer here — only the bus call, the
//! error mapping, and the two decisions below.
//!
//! # Construction is synchronous and does no I/O
//!
//! `memory::binding::build` is called from `CoreContext::memory_binding`, which
//! roughly four thousand pre-boot tests invoke with no tokio runtime at all. So
//! [`ModuleMemoryProvider::new`] cannot load the module, cannot dial the bus, and
//! cannot await anything. It stores its configuration and resolves on first use,
//! the same lazy-loading contract used by the module host.
//!
//! That has one consequence worth stating plainly, because it looks like a
//! shortcut and is not:
//!
//! ## `capabilities()` is answered statically
//!
//! `MemoryProvider::capabilities` is a **synchronous** method, and the module can
//! only answer it over the bus. It therefore cannot be asked here.
//!
//! It does not need to be. The TinyMemory module serves the complete shared API,
//! and that is a property of the artifact's *source*, fixed at the version the
//! registry pins, not something to discover at runtime. So this returns
//! [`Capabilities::all`], and [`ModuleMemoryProvider::verify`] cross-checks it
//! against the module's own answer on first use and logs loudly on disagreement.
//!
//! Guessing high would be the dangerous direction: the kernel filters its RPC
//! surface and agent-tool assembly from this set, so an overstated capability
//! registers methods that answer errors. Guessing exactly is safe; the
//! cross-check catches a future artifact that widens its scope.
//!
//! # Errors round-trip through the shared table
//!
//! `crate::openhuman::memory::api::wire` maps a `MemoryError` to a `(name, message)` pair and
//! back, and **both ends use it**. Reimplementing the mapping here is what would
//! let a `PathEscape` arrive as an `Invalid`, silently reclassifying a sandbox
//! escape as a caller mistake.

use std::sync::Arc;

use crate::openhuman::memory::api::capabilities::Capabilities;
use crate::openhuman::memory::api::chunks::Chunk;
use crate::openhuman::memory::api::error::MemoryError;
use crate::openhuman::memory::api::goals::GoalsDoc;
use crate::openhuman::memory::api::health::MemoryHealth;
use crate::openhuman::memory::api::provider::types::{
    DiffReport, EntityHit, ExportPage, ExportRecord, ImportOutcome, IngestItem, IngestOutcome,
    MaintenanceReport, SnapshotRef, SourceItem, SourceScope,
};
use crate::openhuman::memory::api::provider::{
    MemoryCore, MemoryDiff, MemoryDocuments, MemoryEntities, MemoryGoals, MemoryGraph,
    MemoryIngest, MemoryMaintenance, MemoryPortability, MemoryProvider, MemoryRecall,
    MemorySourceSink, MemoryToolMemory, MemoryTree,
};
use crate::openhuman::memory::api::recall::OwnedRecallOpts;
use crate::openhuman::memory::api::tool_memory::ToolMemoryRule;
use crate::openhuman::memory::api::tree::{IngestRequest, QueryResult, TreeStatus};
use crate::openhuman::memory::api::types::{
    GraphRelationRecord, MemoryCategory, MemoryEntry, MemoryKvRecord, MemoryTaint,
    NamespaceDocumentInput, NamespaceRetrievalContext, NamespaceSummary, StoredMemoryDocument,
};
use crate::openhuman::memory::api::wire;
use async_trait::async_trait;

use super::{host, ops, registry};
use crate::openhuman::config::Config;

/// Registry id of the module these calls go to.
pub const MODULE_ID: &str = "tinymemory";

/// The `[modules]` policy this process was booted with.
///
/// # Why a process-global and not a constructor argument
///
/// `memory::binding::build` is where a module driver is constructed, and it
/// receives only a workspace dir and a `MemorySubsystemConfig`. What
/// [`ops::ensure_loaded`] needs is `modules.{enabled, allow_download,
/// install_dir}`, which lives on the full `Config` — and threading a whole
/// `Config` down through `MemoryBinding::for_workspace` would widen that
/// function's dependency and change a cache key that ~4000 pre-boot tests hit.
///
/// So the policy is published once during boot instead. This is the same shape
/// `tinymemory_core::embedding_host` and `api::product` already use, and for the
/// same stated reason: the construction sites sit too deep to thread through.
///
/// # Unset means disabled, deliberately
///
/// A pre-boot test, or a host that never called [`set_modules_policy`], gets
/// `None` — and [`policy`] then reports modules disabled rather than assuming
/// permissive defaults. Defaulting `enabled` to `true` here would silently
/// ignore an operator who turned modules off, and would let a unit test reach
/// for a download.
static MODULES_POLICY: std::sync::OnceLock<Arc<Config>> = std::sync::OnceLock::new();

/// Publish the config a module driver should load against.
///
/// Call once during boot, before any workspace is bound. Later calls are
/// ignored — a driver already resolved against the first value must not have the
/// policy change underneath it.
pub fn set_modules_policy(config: Arc<Config>) {
    let _ = MODULES_POLICY.set(config);
}

/// The published policy, if boot supplied one.
pub(crate) fn policy() -> Option<&'static Arc<Config>> {
    MODULES_POLICY.get()
}

/// A memory driver served by the loaded `tinymemory` module.
pub struct ModuleMemoryProvider {
    /// The id reported by [`MemoryProvider::driver_id`].
    driver_id: String,
    /// The config to load against, when the caller had one to give.
    ///
    /// `None` is the binding-site case: `build` has no `Config`, so the provider
    /// falls back to the policy published at boot. Tests pass one explicitly.
    config: Option<Arc<Config>>,
    /// Set once the module has answered `Capabilities`, so the cross-check runs
    /// once rather than per call.
    verified: std::sync::OnceLock<()>,
}

impl std::fmt::Debug for ModuleMemoryProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `Config` is not rendered: it carries credentials.
        f.debug_struct("ModuleMemoryProvider")
            .field("driver_id", &self.driver_id)
            .finish_non_exhaustive()
    }
}

impl ModuleMemoryProvider {
    /// Bind the module-backed driver.
    ///
    /// Synchronous and I/O-free by requirement — see the module docs. Nothing is
    /// loaded until the first call.
    #[must_use]
    pub fn new(config: Arc<Config>) -> Self {
        Self::with_optional_config(Some(config))
    }

    /// Bind against the policy published by [`set_modules_policy`].
    ///
    /// This is what `memory::binding::build` uses, because it has no `Config` to
    /// hand over. If boot published nothing, every call reports the module
    /// unavailable rather than guessing a permissive default.
    #[must_use]
    pub fn from_boot_policy() -> Self {
        Self::with_optional_config(None)
    }

    fn with_optional_config(config: Option<Arc<Config>>) -> Self {
        Self {
            driver_id: registry::find(MODULE_ID)
                .map_or_else(|| MODULE_ID.to_string(), |record| record.id.to_string()),
            config,
            verified: std::sync::OnceLock::new(),
        }
    }

    /// Ensure the module is serving, and hand back a proxy for its object.
    ///
    /// `operation` identifies the forwarded call (e.g. `"store"`, `"recall"`)
    /// for the diagnostic below. Never `namespace`, `key`, `content`, or any
    /// record value — those are user memory content, not correlation fields.
    async fn proxy(&self, operation: &str) -> Result<tinybus::Proxy, MemoryError> {
        log::debug!(
            "[modules:memory] driver_id={} operation={operation} resolving module proxy",
            self.driver_id,
        );
        let config = self.config.as_ref().or_else(|| policy()).ok_or_else(|| {
            MemoryError::Other(anyhow::anyhow!(
                "the module host policy was never published, so module '{MODULE_ID}' \
                 cannot be loaded; call modules::memory::set_modules_policy during boot"
            ))
        })?;
        let runtime = host::runtime().await.map_err(|error| {
            MemoryError::Other(anyhow::anyhow!("the module bus is not running: {error}"))
        })?;
        super::memory_host::install(runtime.connection(), Arc::clone(config))
            .await
            .map_err(|error| {
                MemoryError::Other(anyhow::anyhow!(
                    "the memory module host callbacks are unavailable: {error}"
                ))
            })?;
        // TinyMemory resolves its embedding provider while the native library
        // is admitted. Host callbacks must therefore exist before loading,
        // including in tests and explicit-path overrides where no boot policy
        // was available when the shared module runtime first started.
        ops::ensure_loaded(config, MODULE_ID)
            .await
            .map_err(|message| MemoryError::Other(anyhow::anyhow!(message)))?;

        let record = registry::find(MODULE_ID)
            .ok_or_else(|| MemoryError::Other(anyhow::anyhow!("unknown module '{MODULE_ID}'")))?;
        let proxy = runtime
            .proxy(record.bus_name, record.object_path)
            .map_err(|error| MemoryError::Other(anyhow::anyhow!(error.to_string())))?;

        self.verify(&proxy).await;
        Ok(proxy)
    }

    /// Cross-check the module's advertised capabilities against what this build
    /// assumes, once per process.
    ///
    /// Logged rather than fatal. Any mismatch means the registry pin and the
    /// artifact have diverged; advertising less is the dangerous direction,
    /// because the host assembled its full memory surface from this static set.
    async fn verify(&self, proxy: &tinybus::Proxy) {
        if self.verified.get().is_some() {
            return;
        }
        match proxy.call::<Capabilities>("Capabilities", ()).await {
            Ok(actual) => {
                let assumed = Capabilities::all();
                if actual != assumed {
                    log::warn!(
                        "[modules:memory] the module advertises {actual:?} but this build \
                         assumes {assumed:?}; the registry pin and the artifact have diverged"
                    );
                }
            }
            Err(error) => {
                log::warn!("[modules:memory] could not read module capabilities: {error}");
            }
        }
        let _ = self.verified.set(());
    }
}

/// Map a bus failure back onto a [`MemoryError`].
///
/// Uses the shared table so the host and the module cannot disagree about what a
/// name means. An unrecognised name becomes `Other`, never `Invalid`.
fn from_bus(error: &tinybus::Error) -> MemoryError {
    wire::from_wire(error.wire_name(), &error.to_string())
}

#[async_trait]
impl MemoryProvider for ModuleMemoryProvider {
    fn driver_id(&self) -> &str {
        &self.driver_id
    }

    /// Every family is implemented by the pinned compiled module.
    fn capabilities(&self) -> Capabilities {
        Capabilities::all()
    }

    async fn health(&self) -> MemoryHealth {
        // An unreachable module is a *health* answer, not an error: that is the
        // question this method exists to answer, and returning `Down` is how
        // status output shows an unsupported platform or a refused artifact.
        match self.proxy("health").await {
            Ok(proxy) => proxy
                .call::<MemoryHealth>("Health", ())
                .await
                .unwrap_or_else(|error| MemoryHealth::down(error.to_string())),
            Err(error) => MemoryHealth::down(error.to_string()),
        }
    }

    async fn shutdown(&self) -> Result<(), MemoryError> {
        // Deliberately does not load the module in order to shut it down: a
        // shutdown on a driver that was never used should be a no-op, not a
        // download. tinybus never unloads a library anyway, so this releases
        // backend resources only.
        if self.verified.get().is_none() {
            return Ok(());
        }
        let proxy = self.proxy("shutdown").await?;
        proxy
            .call::<()>("Shutdown", ())
            .await
            .map_err(|error| from_bus(&error))
    }

    fn as_ingest(&self) -> Option<&dyn MemoryIngest> {
        Some(self)
    }
    fn as_documents(&self) -> Option<&dyn MemoryDocuments> {
        Some(self)
    }
    fn as_tree(&self) -> Option<&dyn MemoryTree> {
        Some(self)
    }
    fn as_entities(&self) -> Option<&dyn MemoryEntities> {
        Some(self)
    }
    fn as_graph(&self) -> Option<&dyn MemoryGraph> {
        Some(self)
    }
    fn as_diff(&self) -> Option<&dyn MemoryDiff> {
        Some(self)
    }
    fn as_goals(&self) -> Option<&dyn MemoryGoals> {
        Some(self)
    }
    fn as_tool_memory(&self) -> Option<&dyn MemoryToolMemory> {
        Some(self)
    }
    fn as_sources(&self) -> Option<&dyn MemorySourceSink> {
        Some(self)
    }
    fn as_maintenance(&self) -> Option<&dyn MemoryMaintenance> {
        Some(self)
    }
}

#[async_trait]
impl MemoryCore for ModuleMemoryProvider {
    async fn store(
        &self,
        namespace: &str,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
        taint: MemoryTaint,
    ) -> Result<(), MemoryError> {
        // No log line carries `namespace`, `key` or `content`: all three are user
        // memory content.
        self.proxy("store")
            .await?
            .call::<()>(
                "Store",
                (
                    namespace,
                    key,
                    content,
                    category,
                    session_id.map(str::to_string),
                    taint,
                ),
            )
            .await
            .map_err(|error| from_bus(&error))
    }

    async fn get(&self, namespace: &str, key: &str) -> Result<Option<MemoryEntry>, MemoryError> {
        self.proxy("get")
            .await?
            .call("Get", (namespace, key))
            .await
            .map_err(|error| from_bus(&error))
    }

    async fn forget(&self, namespace: &str, key: &str) -> Result<bool, MemoryError> {
        self.proxy("forget")
            .await?
            .call("Forget", (namespace, key))
            .await
            .map_err(|error| from_bus(&error))
    }

    async fn list(
        &self,
        namespace: Option<&str>,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        self.proxy("list")
            .await?
            .call(
                "List",
                (
                    namespace.map(str::to_string),
                    category.cloned(),
                    session_id.map(str::to_string),
                ),
            )
            .await
            .map_err(|error| from_bus(&error))
    }

    async fn namespaces(&self) -> Result<Vec<NamespaceSummary>, MemoryError> {
        self.proxy("namespaces")
            .await?
            .call("Namespaces", ())
            .await
            .map_err(|error| from_bus(&error))
    }
}

#[async_trait]
impl MemoryRecall for ModuleMemoryProvider {
    async fn recall(
        &self,
        query: &str,
        limit: usize,
        opts: &OwnedRecallOpts,
        scope: Option<&SourceScope>,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        // `scope` crosses as a value because the driver must apply it as a query
        // predicate internally; narrowing the result here instead would let the
        // module spend its `limit` on entries the caller may not see.
        self.proxy("recall")
            .await?
            .call("Recall", (query, limit, opts, scope.cloned()))
            .await
            .map_err(|error| from_bus(&error))
    }
}

#[async_trait]
impl MemoryPortability for ModuleMemoryProvider {
    async fn export_page(
        &self,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<ExportPage, MemoryError> {
        self.proxy("export_page")
            .await?
            .call("ExportPage", (cursor.map(str::to_string), limit))
            .await
            .map_err(|error| from_bus(&error))
    }

    async fn import_records(
        &self,
        records: Vec<ExportRecord>,
    ) -> Result<ImportOutcome, MemoryError> {
        self.proxy("import_records")
            .await?
            .call("ImportRecords", (records,))
            .await
            .map_err(|error| from_bus(&error))
    }
}

macro_rules! module_call {
    ($self:expr, $operation:literal, $method:literal, $args:expr) => {
        $self
            .proxy($operation)
            .await?
            .call($method, $args)
            .await
            .map_err(|error| from_bus(&error))
    };
}

#[async_trait]
impl MemoryIngest for ModuleMemoryProvider {
    async fn ingest_document(&self, item: IngestItem) -> Result<IngestOutcome, MemoryError> {
        module_call!(self, "ingest_document", "IngestDocument", (item,))
    }
    async fn ingest_chat(&self, messages: Vec<IngestItem>) -> Result<IngestOutcome, MemoryError> {
        module_call!(self, "ingest_chat", "IngestChat", (messages,))
    }
}

#[async_trait]
impl MemoryDocuments for ModuleMemoryProvider {
    async fn put_document(&self, input: NamespaceDocumentInput) -> Result<String, MemoryError> {
        module_call!(self, "put_document", "PutDocument", (input,))
    }
    async fn get_document(
        &self,
        namespace: &str,
        key: &str,
    ) -> Result<Option<StoredMemoryDocument>, MemoryError> {
        module_call!(self, "get_document", "GetDocument", (namespace, key))
    }
    async fn list_documents(
        &self,
        namespace: Option<&str>,
    ) -> Result<serde_json::Value, MemoryError> {
        module_call!(
            self,
            "list_documents",
            "ListDocuments",
            (namespace.map(str::to_string),)
        )
    }
    async fn list_namespaces(&self) -> Result<Vec<String>, MemoryError> {
        module_call!(self, "list_namespaces", "ListNamespaces", ())
    }
    async fn delete_document(
        &self,
        namespace: &str,
        document_id: &str,
    ) -> Result<serde_json::Value, MemoryError> {
        module_call!(
            self,
            "delete_document",
            "DeleteDocument",
            (namespace, document_id)
        )
    }
    async fn clear_namespace(&self, namespace: &str) -> Result<(), MemoryError> {
        module_call!(self, "clear_namespace", "ClearNamespace", (namespace,))
    }
    async fn query_documents(
        &self,
        namespace: &str,
        query: &str,
        limit: usize,
    ) -> Result<NamespaceRetrievalContext, MemoryError> {
        module_call!(
            self,
            "query_documents",
            "QueryDocuments",
            (namespace, query, limit)
        )
    }
    async fn recall_documents(
        &self,
        namespace: &str,
        limit: usize,
    ) -> Result<NamespaceRetrievalContext, MemoryError> {
        module_call!(
            self,
            "recall_documents",
            "RecallDocuments",
            (namespace, limit)
        )
    }
}

#[async_trait]
impl MemoryTree for ModuleMemoryProvider {
    async fn append(&self, request: IngestRequest) -> Result<(), MemoryError> {
        module_call!(self, "append", "Append", (request,))
    }
    async fn query_source(
        &self,
        namespace: &str,
        source_id: &str,
        limit: usize,
        scope: Option<&SourceScope>,
    ) -> Result<Vec<Chunk>, MemoryError> {
        module_call!(
            self,
            "query_source",
            "QuerySource",
            (namespace, source_id, limit, scope.cloned())
        )
    }
    async fn drill_down(&self, namespace: &str, node_id: &str) -> Result<QueryResult, MemoryError> {
        module_call!(self, "drill_down", "DrillDown", (namespace, node_id))
    }
    async fn seal(&self, namespace: &str) -> Result<TreeStatus, MemoryError> {
        module_call!(self, "seal", "Seal", (namespace,))
    }
    async fn cascade(&self, namespace: &str) -> Result<TreeStatus, MemoryError> {
        module_call!(self, "cascade", "Cascade", (namespace,))
    }
}

#[async_trait]
impl MemoryEntities for ModuleMemoryProvider {
    async fn entities(
        &self,
        namespace: &str,
        query: Option<&str>,
        limit: usize,
    ) -> Result<Vec<EntityHit>, MemoryError> {
        module_call!(
            self,
            "entities",
            "Entities",
            (namespace, query.map(str::to_string), limit)
        )
    }
    async fn entity_edges(
        &self,
        namespace: &str,
        entity_id: &str,
        limit: usize,
    ) -> Result<Vec<GraphRelationRecord>, MemoryError> {
        module_call!(
            self,
            "entity_edges",
            "EntityEdges",
            (namespace, entity_id, limit)
        )
    }
    async fn touch_entities(
        &self,
        namespace: &str,
        entity_ids: &[String],
    ) -> Result<(), MemoryError> {
        module_call!(
            self,
            "touch_entities",
            "TouchEntities",
            (namespace, entity_ids.to_vec())
        )
    }
}

#[async_trait]
impl MemoryGraph for ModuleMemoryProvider {
    async fn kv_get(
        &self,
        namespace: Option<&str>,
        key: &str,
    ) -> Result<Option<MemoryKvRecord>, MemoryError> {
        module_call!(
            self,
            "kv_get",
            "KvGet",
            (namespace.map(str::to_string), key)
        )
    }
    async fn kv_put(
        &self,
        namespace: Option<&str>,
        key: &str,
        value: serde_json::Value,
    ) -> Result<(), MemoryError> {
        module_call!(
            self,
            "kv_put",
            "KvPut",
            (namespace.map(str::to_string), key, value)
        )
    }
    async fn kv_delete(&self, namespace: Option<&str>, key: &str) -> Result<bool, MemoryError> {
        module_call!(
            self,
            "kv_delete",
            "KvDelete",
            (namespace.map(str::to_string), key)
        )
    }
    async fn kv_list(
        &self,
        namespace: Option<&str>,
        prefix: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryKvRecord>, MemoryError> {
        module_call!(
            self,
            "kv_list",
            "KvList",
            (
                namespace.map(str::to_string),
                prefix.map(str::to_string),
                limit
            )
        )
    }
    async fn relations(
        &self,
        namespace: Option<&str>,
        subject: Option<&str>,
        predicate: Option<&str>,
        limit: usize,
    ) -> Result<Vec<GraphRelationRecord>, MemoryError> {
        module_call!(
            self,
            "relations",
            "Relations",
            (
                namespace.map(str::to_string),
                subject.map(str::to_string),
                predicate.map(str::to_string),
                limit
            )
        )
    }
    async fn put_relation(&self, relation: GraphRelationRecord) -> Result<(), MemoryError> {
        module_call!(self, "put_relation", "PutRelation", (relation,))
    }
}

#[async_trait]
impl MemoryDiff for ModuleMemoryProvider {
    async fn capture_snapshot(&self, source_id: &str) -> Result<SnapshotRef, MemoryError> {
        module_call!(self, "capture_snapshot", "CaptureSnapshot", (source_id,))
    }
    async fn snapshots(
        &self,
        source_id: &str,
        limit: usize,
    ) -> Result<Vec<SnapshotRef>, MemoryError> {
        module_call!(self, "snapshots", "Snapshots", (source_id, limit))
    }
    async fn diff(
        &self,
        source_id: &str,
        from: Option<&str>,
        to: &str,
    ) -> Result<DiffReport, MemoryError> {
        module_call!(
            self,
            "diff",
            "Diff",
            (source_id, from.map(str::to_string), to)
        )
    }
}

#[async_trait]
impl MemoryGoals for ModuleMemoryProvider {
    async fn goals(&self) -> Result<GoalsDoc, MemoryError> {
        module_call!(self, "goals", "Goals", ())
    }
    async fn set_goals(&self, goals: GoalsDoc) -> Result<(), MemoryError> {
        module_call!(self, "set_goals", "SetGoals", (goals,))
    }
}

#[async_trait]
impl MemoryToolMemory for ModuleMemoryProvider {
    async fn tool_rules(&self, tool_name: &str) -> Result<Vec<ToolMemoryRule>, MemoryError> {
        module_call!(self, "tool_rules", "ToolRules", (tool_name,))
    }
    async fn put_tool_rule(&self, rule: ToolMemoryRule) -> Result<(), MemoryError> {
        module_call!(self, "put_tool_rule", "PutToolRule", (rule,))
    }
    async fn delete_tool_rule(&self, tool_name: &str, rule_id: &str) -> Result<bool, MemoryError> {
        module_call!(
            self,
            "delete_tool_rule",
            "DeleteToolRule",
            (tool_name, rule_id)
        )
    }
}

#[async_trait]
impl MemorySourceSink for ModuleMemoryProvider {
    async fn accept_source_items(
        &self,
        source_id: &str,
        source_kind: &str,
        items: Vec<SourceItem>,
        taint: MemoryTaint,
    ) -> Result<IngestOutcome, MemoryError> {
        module_call!(
            self,
            "accept_source_items",
            "AcceptSourceItems",
            (source_id, source_kind, items, taint)
        )
    }
    async fn forget_source(&self, source_id: &str) -> Result<u64, MemoryError> {
        module_call!(self, "forget_source", "ForgetSource", (source_id,))
    }
}

#[async_trait]
impl MemoryMaintenance for ModuleMemoryProvider {
    async fn reembed(&self) -> Result<MaintenanceReport, MemoryError> {
        module_call!(self, "reembed", "Reembed", ())
    }
    async fn compact(&self) -> Result<MaintenanceReport, MemoryError> {
        module_call!(self, "compact", "Compact", ())
    }
    async fn consolidate(&self) -> Result<MaintenanceReport, MemoryError> {
        module_call!(self, "consolidate", "Consolidate", ())
    }
    async fn doctor(&self) -> Result<MaintenanceReport, MemoryError> {
        module_call!(self, "doctor", "Doctor", ())
    }
}

#[cfg(test)]
#[path = "memory_tests.rs"]
mod tests;
