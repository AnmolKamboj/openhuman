//! Registry of in-flight async sub-agents that can be **steered** mid-run.
//!
//! `spawn_async_subagent` runs a child as a detached `tokio` task. On its own
//! that task is opaque: the parent gets a `task_id` back but has no channel into
//! the running loop and no way to collect the result inline. This registry
//! closes both gaps.
//!
//! Each running async sub-agent registers in TinyAgents'
//! [`DetachedTaskRegistry`], keyed by its `task_id`, with:
//! - an `Arc<RunQueue>` — the same steering channel the steering forwarder in
//!   `run_turn_via_tinyagents_shared` drains mid-turn, so `steer_subagent` can
//!   inject a message when no crate-native steering handle is registered;
//! - a TinyAgents `SteeringHandle` in the process-local
//!   `SteeringRegistry` while the child TinyAgents run is active, so
//!   steer/collect controls can deliver directly to the crate queue;
//! - a `watch::Receiver<SubagentStatus>` — so `wait_subagent` can block until the
//!   child reaches a terminal status;
//! - an `AbortHandle` — used by `subagent_cancel`/`close_subagent` paths to stop
//!   detached work.
//!
//! TinyAgents owns the process-local watch/cancel/abort/steering mechanics.
//! OpenHuman retains product metadata, durable task-store projection, and the
//! legacy `RunQueue` steering fallback. Ownership is enforced by parent session;
//! terminal entries are pruned on `wait` and swept at the registry soft cap.
//!
//! ## Typed lifecycle ledger (issue #4249)
//!
//! Alongside the executor plumbing (abort handle + steering queue + watch
//! status), every detached sub-agent is also recorded in a process-wide
//! [`tinyagents` orchestration `TaskStore`](crate::openhuman::agent::tinyagents::orchestration)
//! as an `OrchestrationTaskKind::SubAgent` task. `register` inserts it
//! (`Pending` → `Running`) and spawns a watcher that mirrors the child's
//! terminal status into the store (`Completed`/`Failed`/`Awaiting`); the cancel
//! paths record `Cancelled`. This gives a typed, queryable lifecycle
//! (`task_records`) alongside the crate-owned runtime registry.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use tokio::sync::watch;
use tokio::task::AbortHandle;

use crate::openhuman::agent::harness::run_queue::{QueueMode, QueuedMessage, RunQueue};
use crate::openhuman::agent::tinyagents::orchestration::{
    open_jsonl_task_store_or_memory, reconcile_orphaned_tasks, shared_steering_registry,
    DetachedTaskRegistry, DetachedTaskRegistryError, DetachedTaskWaitOutcome, InMemoryTaskStore,
    OrchestrationTaskFilter, OrchestrationTaskKind, OrchestrationTaskRecord,
    OrchestrationTaskResult, OrchestrationTaskSpec, OrchestrationTaskStatus, SteeringCommand,
    SteeringCommandKind, TaskStore, TaskStoreRegistry,
};
use tinyagents::harness::ids::TaskId;
use tinyagents::harness::message::Message as TaMessage;
use tinyagents::CancellationToken;

/// Where a workspace's detached-task ledger lives.
///
/// A product path, not a generic one: TinyAgents opens whatever file it is
/// given, and this is where OpenHuman keeps it.
fn task_store_path(workspace_dir: &Path) -> PathBuf {
    workspace_dir
        .join(".openhuman")
        .join("orchestration_tasks.jsonl")
}

#[cfg(test)]
fn default_task_store_workspace() -> PathBuf {
    crate::openhuman::config::default_root_openhuman_dir()
        .map(|root| root.join("workspace"))
        .unwrap_or_else(|_| PathBuf::from(".openhuman").join("workspace"))
}

/// Process-wide typed lifecycle ledger for detached sub-agents (issue #4249),
/// one durable store per workspace.
///
/// The caching and the durable→memory fallback are TinyAgents'
/// (`TaskStoreRegistry` / `open_jsonl_task_store_or_memory`): opening a second
/// store over the same append log would give two writers with independently
/// replayed state, and a workspace that cannot be written should degrade to an
/// in-memory ledger rather than take orchestration down. What stays here is the
/// path layout above.
static TASK_STORES: OnceLock<TaskStoreRegistry<PathBuf>> = OnceLock::new();

fn task_stores() -> &'static TaskStoreRegistry<PathBuf> {
    TASK_STORES.get_or_init(|| {
        TaskStoreRegistry::new(|workspace_dir: &PathBuf| {
            open_jsonl_task_store_or_memory(&task_store_path(workspace_dir))
        })
    })
}

/// The ledger for `workspace_dir`, opening it on first use.
///
/// A poisoned registry lock is degraded to a throwaway in-memory store rather
/// than propagated: every caller here is a best-effort bookkeeping path, and a
/// panic in an unrelated task must not turn sub-agent spawning into a second
/// panic.
fn task_store_for_workspace(workspace_dir: &Path) -> Arc<dyn TaskStore> {
    let key = workspace_dir.to_path_buf();
    match task_stores().get_or_open(&key) {
        Ok(store) => store,
        Err(err) => {
            log::warn!(
                "[running_subagents] task store registry unavailable for {}; using a detached in-memory ledger: {}",
                workspace_dir.display(),
                err
            );
            Arc::new(InMemoryTaskStore::new())
        }
    }
}

#[cfg(test)]
fn task_store() -> Arc<dyn TaskStore> {
    let workspace = default_task_store_workspace();
    task_store_for_workspace(&workspace)
}

/// Record a freshly-spawned sub-agent in the store (`Pending` → `Running`).
/// Insert errors (e.g. a re-used task id across tests) are intentionally ignored.
fn record_spawned(
    task_id: &str,
    agent_id: &str,
    parent_session: &str,
    session_parent_prefix: Option<&str>,
    subagent_session_id: Option<&str>,
    workspace_dir: &Path,
    parent_thread_id: Option<&str>,
) {
    let store = task_store_for_workspace(workspace_dir);
    let root_run_id = session_parent_prefix
        .and_then(|prefix| prefix.split("__").next())
        .filter(|root| !root.is_empty())
        .unwrap_or(parent_session);
    let mut spec = OrchestrationTaskSpec::new(
        task_id.to_string(),
        OrchestrationTaskKind::SubAgent {
            agent: agent_id.to_string(),
        },
    )
    .with_lineage(parent_session.to_string(), root_run_id.to_string())
    .with_timeout_ms(DETACHED_LEDGER_TIMEOUT_MS)
    .with_metadata("parentSession", parent_session.to_string())
    .with_metadata("rootSession", root_run_id.to_string())
    .with_metadata(
        "defaultWaitTimeoutMs",
        DETACHED_LEDGER_TIMEOUT_MS.to_string(),
    )
    .with_metadata("workspaceDir", workspace_dir.display().to_string());
    if let Some(session_parent_prefix) = session_parent_prefix {
        spec = spec.with_metadata("sessionParentPrefix", session_parent_prefix.to_string());
    }
    if let Some(parent_thread_id) = parent_thread_id {
        spec = spec
            .with_thread(parent_thread_id.to_string())
            .with_metadata("parentThreadId", parent_thread_id.to_string());
    }
    if let Some(subagent_session_id) = subagent_session_id {
        spec = spec.with_metadata("subagentSessionId", subagent_session_id.to_string());
    }
    let _ = store.insert(spec);
    let _ = store.mark_running(&TaskId::new(task_id));
}

/// Mirror a child's published [`SubagentStatus`] into the typed store. Transition
/// errors (already terminal / cancelled) are ignored — first writer wins.
fn record_status(workspace_dir: &Path, task_id: &str, status: &SubagentStatus) {
    let store = task_store_for_workspace(workspace_dir);
    let id = TaskId::new(task_id);
    log::debug!(
        "[running_subagents] recording task status task_id={} workspace_dir={} terminal={}",
        task_id,
        workspace_dir.display(),
        status.is_terminal()
    );
    match status {
        SubagentStatus::Completed { output, .. } => {
            let _ = store.complete(&id, OrchestrationTaskResult::text(output.clone()));
        }
        SubagentStatus::Failed { error } => {
            let _ = store.fail(&id, error.clone());
        }
        SubagentStatus::AwaitingUser { .. } => {
            let _ = store.mark_awaiting(&id);
        }
        SubagentStatus::Running => {}
    }
}

/// Record a cancellation (`CancelRequested` → `Cancelled`) for `task_id`.
fn record_cancelled(workspace_dir: &Path, task_id: &str) {
    let store = task_store_for_workspace(workspace_dir);
    let id = TaskId::new(task_id);
    log::debug!(
        "[running_subagents] recording task cancellation task_id={} workspace_dir={}",
        task_id,
        workspace_dir.display()
    );
    let _ = store.request_cancel(&id);
    let _ = store.mark_cancelled(&id);
}

fn list_task_records(workspace_dir: &Path) -> Vec<OrchestrationTaskRecord> {
    let store = task_store_for_workspace(workspace_dir);
    store.list(OrchestrationTaskFilter::default().with_kind("sub_agent"))
}

/// Restart/resume reconciliation for detached sub-agents (issue #4249 / 07.2
/// steps 2 & 4).
///
/// A detached sub-agent runs as a `tokio` task owned by the process that spawned
/// it. When the core restarts, that task — and its live [`AbortHandle`] /
/// [`CancellationToken`] — is gone, but the durable [`JsonlTaskStore`] still
/// holds a non-terminal (`Pending`/`Running`/`Awaiting`/`CancelRequested`)
/// record for it. Such a record is **orphaned**: there is no live executor to
/// re-attach to (OpenHuman spawns child processes, so an in-flight run from a
/// dead parent cannot be resumed), and the run-ledger finalizer never observed a
/// terminal event, so it would otherwise render as a perpetual "running" entry.
///
/// This scans the workspace-scoped store for those orphans and reconciles each
/// to a terminal state — `Cancelled` if a cancel had been requested, otherwise
/// `Failed` with an "orphaned by restart" reason — then emits the existing typed
/// terminal lifecycle event ([`subagent_events::publish_subagent_failed`]) so the
/// run ledger finalizes. Best-effort and non-fatal: per-task transition errors
/// (e.g. a record that raced to terminal) are logged and skipped, and a
/// store-open failure simply reconciles nothing. Returns the count reconciled.
/// The reason an orphaned sub-agent record is settled with.
///
/// Built in one place because it is written twice — into the store by the
/// reconciler, and into the lifecycle event the run ledger reads. If those two
/// ever disagreed, the ledger would explain a failure differently from the
/// record behind it.
fn orphaned_subagent_reason(prior_status: OrchestrationTaskStatus) -> String {
    format!(
        "sub-agent orphaned by core restart (was `{}`)",
        task_status_label(prior_status)
    )
}

pub(crate) fn reconcile_orphaned_tasks_on_boot(workspace_dir: &Path) -> usize {
    let store = task_store_for_workspace(workspace_dir);

    // The sweep itself — which statuses are live, and which terminal state each
    // becomes — is TinyAgents'. What stays here is the reason a *sub-agent*
    // orphan carries, and the lifecycle event that finalizes OpenHuman's run
    // ledger afterwards.
    let report = reconcile_orphaned_tasks(
        store.as_ref(),
        OrchestrationTaskFilter::default().with_kind("sub_agent"),
        &|record| orphaned_subagent_reason(record.status),
    );

    if report.is_empty() {
        log::debug!(
            "[running_subagents] reconcile found no orphaned sub-agent tasks workspace_dir={}",
            workspace_dir.display()
        );
        return 0;
    }

    for task in report.settled() {
        let task_id = task.task_id.as_str().to_string();
        let prior = task_status_label(task.prior_status);
        let reason = orphaned_subagent_reason(task.prior_status);
        let parent_session = record_parent_session(&task.record)
            .unwrap_or_default()
            .to_string();
        let agent_id = record_agent_id(&task.record);
        // Reuse the 05.2 typed terminal lifecycle helper so the run ledger
        // finalizes exactly as it would for a live failure.
        super::subagent_events::publish_subagent_failed(
            parent_session,
            task_id.clone(),
            agent_id,
            reason,
        );
        log::info!(
            "[running_subagents] reconciled orphaned sub-agent task_id={} prior_status={} -> terminal",
            task_id,
            prior
        );
    }

    let reconciled = report.reconciled_count();
    log::info!(
        "[running_subagents] reconciled {reconciled} orphaned sub-agent task(s) on boot workspace_dir={} errors={}",
        workspace_dir.display(),
        report.error_count()
    );
    reconciled
}

fn record_parent_session(record: &OrchestrationTaskRecord) -> Option<&str> {
    record
        .spec
        .metadata
        .get("parentSession")
        .map(String::as_str)
}

fn record_subagent_session_id(record: &OrchestrationTaskRecord) -> Option<&str> {
    record
        .spec
        .metadata
        .get("subagentSessionId")
        .map(String::as_str)
}

fn record_agent_id(record: &OrchestrationTaskRecord) -> String {
    match &record.spec.kind {
        OrchestrationTaskKind::SubAgent { agent } => agent.clone(),
        _ => "subagent".to_string(),
    }
}

pub(crate) fn task_record_for_task_in_workspace(
    workspace_dir: &Path,
    task_id: &str,
    parent_session: &str,
) -> Result<OrchestrationTaskRecord, WaitError> {
    let id = TaskId::new(task_id);
    let Some(record) = task_store_for_workspace(workspace_dir).get(&id) else {
        return Err(WaitError::Unknown);
    };
    if !matches!(record.spec.kind, OrchestrationTaskKind::SubAgent { .. }) {
        return Err(WaitError::Unknown);
    }
    if record_parent_session(&record) != Some(parent_session) {
        return Err(WaitError::NotOwned);
    }
    Ok(record)
}

fn record_to_status(record: OrchestrationTaskRecord) -> WaitOutcome {
    match record.status {
        OrchestrationTaskStatus::Completed => {
            let output = record
                .result
                .and_then(|result| {
                    result
                        .text
                        .or_else(|| result.output.map(|output| output.to_string()))
                })
                .unwrap_or_default();
            WaitOutcome::Terminal(SubagentStatus::Completed {
                output,
                iterations: 0,
            })
        }
        OrchestrationTaskStatus::Awaiting => WaitOutcome::Terminal(SubagentStatus::AwaitingUser {
            question: record.error.unwrap_or_else(|| {
                "sub-agent is awaiting user input; no clarification text was available from the durable task store".to_string()
            }),
        }),
        OrchestrationTaskStatus::Failed
        | OrchestrationTaskStatus::TimedOut
        | OrchestrationTaskStatus::Abandoned => WaitOutcome::Terminal(SubagentStatus::Failed {
            error: record.error.unwrap_or_else(|| {
                format!(
                    "sub-agent reached durable task status `{}`",
                    task_status_label(record.status)
                )
            }),
        }),
        OrchestrationTaskStatus::Cancelled => WaitOutcome::Terminal(SubagentStatus::Failed {
            error: "sub-agent was cancelled".to_string(),
        }),
        OrchestrationTaskStatus::Pending
        | OrchestrationTaskStatus::Running
        | OrchestrationTaskStatus::CancelRequested => WaitOutcome::TimedOut(SubagentStatus::Running),
    }
}

fn task_status_label(status: OrchestrationTaskStatus) -> &'static str {
    match status {
        OrchestrationTaskStatus::Pending => "pending",
        OrchestrationTaskStatus::Running => "running",
        OrchestrationTaskStatus::Awaiting => "awaiting",
        OrchestrationTaskStatus::Completed => "completed",
        OrchestrationTaskStatus::Failed => "failed",
        OrchestrationTaskStatus::CancelRequested => "cancel_requested",
        OrchestrationTaskStatus::Cancelled => "cancelled",
        OrchestrationTaskStatus::TimedOut => "timed_out",
        OrchestrationTaskStatus::Abandoned => "abandoned",
    }
}

/// Snapshot the typed lifecycle records, optionally scoped to a `parent_session`.
#[cfg(test)]
fn task_records(parent_session: Option<&str>) -> Vec<OrchestrationTaskRecord> {
    let _ = task_store();
    let stores: Vec<Arc<dyn TaskStore>> = task_stores().values().unwrap_or_default();
    let all: Vec<OrchestrationTaskRecord> = stores
        .into_iter()
        .flat_map(|store| store.list(OrchestrationTaskFilter::default()))
        .collect();
    log::trace!(
        "[running_subagents] task_records loaded records={} parent_session_filter={:?}",
        all.len(),
        parent_session
    );
    match parent_session {
        Some(ps) => all
            .into_iter()
            .filter(|r| r.spec.metadata.get("parentSession").map(String::as_str) == Some(ps))
            .collect(),
        None => all,
    }
}

/// Terminal/transient state of a running async sub-agent, published by the
/// spawner's background task and observed by `wait_subagent`.
#[derive(Debug, Clone)]
pub(crate) enum SubagentStatus {
    /// Still executing its inner tool-call loop.
    Running,
    /// Finished normally with a final response.
    Completed { output: String, iterations: usize },
    /// Paused on `ask_user_clarification`; resume via `continue_subagent`.
    AwaitingUser { question: String },
    /// The run errored out.
    Failed { error: String },
}

impl SubagentStatus {
    fn is_terminal(&self) -> bool {
        !matches!(self, SubagentStatus::Running)
    }
}

#[derive(Clone)]
struct RunningSubagentMetadata {
    agent_id: String,
    subagent_session_id: Option<String>,
    workspace_dir: PathBuf,
    /// Parent chat thread that spawned this sub-agent, captured at registration.
    /// `None` for a headless spawn with no originating thread. Used to abort the
    /// sub-agent when its parent thread is deleted (see [`cancel_for_thread`]).
    parent_thread_id: Option<String>,
    run_queue: Arc<RunQueue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SubagentResumeRef {
    pub(crate) task_id: String,
    pub(crate) agent_id: String,
    pub(crate) subagent_session_id: Option<String>,
}

/// Soft cap on registry size. Terminal entries are only swept when the table
/// grows past this, so the common case (a handful of live sub-agents) never
/// evicts a still-uncollected terminal result out from under a `wait`/`steer`.
const REGISTRY_SOFT_CAP: usize = 256;
/// Metadata-only timeout mirrored into the TinyAgents task ledger. It matches
/// `wait_subagent`'s default wait window; execution remains governed by the
/// existing detached task and wait-tool paths.
const DETACHED_LEDGER_TIMEOUT_MS: u64 = 120_000;

static REGISTRY: OnceLock<DetachedTaskRegistry<RunningSubagentMetadata, SubagentStatus>> =
    OnceLock::new();

fn registry() -> &'static DetachedTaskRegistry<RunningSubagentMetadata, SubagentStatus> {
    REGISTRY.get_or_init(|| {
        DetachedTaskRegistry::new(
            shared_steering_registry().clone(),
            REGISTRY_SOFT_CAP,
            SubagentStatus::is_terminal,
        )
    })
}

/// Create the status channel a spawner threads into [`register`].
///
/// The spawner moves the [`watch::Sender`] into its detached task and `send`s a
/// terminal [`SubagentStatus`] on completion. Dropping the sender (e.g. a
/// panicked/aborted task) closes the channel, which `wait_subagent` surfaces as
/// a failure rather than hanging.
pub(crate) fn status_channel() -> (
    watch::Sender<SubagentStatus>,
    watch::Receiver<SubagentStatus>,
) {
    watch::channel(SubagentStatus::Running)
}

/// Register a running async sub-agent so it can be steered and waited on.
///
/// Call this *after* `tokio::spawn` so the [`AbortHandle`] is available; the
/// task owns the matching [`watch::Sender`] from [`status_channel`]. Once the
/// table passes [`REGISTRY_SOFT_CAP`], registration sweeps already-terminal
/// entries so it stays bounded even if a parent never calls `wait_subagent`.
pub(crate) fn register(
    task_id: String,
    agent_id: String,
    parent_session: String,
    session_parent_prefix: Option<String>,
    subagent_session_id: Option<String>,
    workspace_dir: PathBuf,
    parent_thread_id: Option<String>,
    run_queue: Arc<RunQueue>,
    abort: AbortHandle,
    status: watch::Receiver<SubagentStatus>,
) {
    // Typed lifecycle ledger: record the spawn and mirror the child's terminal
    // status into the store via a lightweight watcher (issue #4249). Done before
    // the entry is moved into the map so the metadata is still in scope.
    record_spawned(
        &task_id,
        &agent_id,
        &parent_session,
        session_parent_prefix.as_deref(),
        subagent_session_id.as_deref(),
        &workspace_dir,
        parent_thread_id.as_deref(),
    );
    spawn_status_watcher(task_id.clone(), workspace_dir.clone(), status.clone());

    let metadata = RunningSubagentMetadata {
        agent_id,
        subagent_session_id,
        workspace_dir,
        parent_thread_id,
        run_queue,
    };
    registry()
        .register(
            TaskId::new(task_id.clone()),
            parent_session,
            metadata,
            status,
            // Cooperative cancellation is flipped before the registry invokes
            // the hard abort. The child executor can adopt this token without
            // changing the registry/control API.
            CancellationToken::new(),
            abort,
        )
        .expect("duplicate detached sub-agent task id");
    log::debug!(
        "[running_subagents] registered task_id={} live_entries={}",
        task_id,
        registry()
            .len()
            .expect("detached task registry lock poisoned")
    );
}

/// Watch a child's status channel and mirror the first terminal status into the
/// typed lifecycle store. A dropped sender (aborted/panicked task) without a
/// terminal status is recorded as a failure, matching [`wait`].
fn spawn_status_watcher(
    task_id: String,
    workspace_dir: PathBuf,
    mut status: watch::Receiver<SubagentStatus>,
) {
    tokio::spawn(async move {
        loop {
            let snapshot = status.borrow_and_update().clone();
            if snapshot.is_terminal() {
                record_status(&workspace_dir, &task_id, &snapshot);
                break;
            }
            if status.changed().await.is_err() {
                record_status(
                    &workspace_dir,
                    &task_id,
                    &SubagentStatus::Failed {
                        error: "sub-agent task ended without reporting a result".to_string(),
                    },
                );
                break;
            }
        }
    });
}

/// Compact, read-only view of one registered sub-agent, for ambient injection
/// into a parent's turn context (see [`active_subagents_context_block`]).
#[derive(Debug, Clone)]
pub(crate) struct SubagentSnapshot {
    /// Worker *type* (e.g. `researcher`). Not unique — two parallel researchers
    /// share this; disambiguate on `subagent_session_id` / `task_id`.
    pub(crate) agent_id: String,
    /// Durable, stable per-worker reference the prompt steers/waits/closes by.
    pub(crate) subagent_session_id: Option<String>,
    /// Transient registry key.
    pub(crate) task_id: String,
    /// Stable status label: `running` / `awaiting_user` / `completed` / `failed`.
    pub(crate) status: &'static str,
}

/// Snapshot the sub-agents registered under `parent_session`, with each status
/// read live from its watch channel. Read-only: it takes the registry lock only
/// long enough to clone out the small summaries, never blocks on a child, and
/// never mutates the table. Ordered by `agent_id` then `task_id` so the rendered
/// roster is stable across turns (the underlying map is unordered).
pub(crate) fn snapshot_for_parent(parent_session: &str) -> Vec<SubagentSnapshot> {
    let mut out: Vec<SubagentSnapshot> = registry()
        .snapshots(Some(parent_session))
        .expect("detached task registry lock poisoned")
        .into_iter()
        .map(|entry| {
            let status = match &entry.status {
                SubagentStatus::Running => "running",
                SubagentStatus::Completed { .. } => "completed",
                SubagentStatus::AwaitingUser { .. } => "awaiting_user",
                SubagentStatus::Failed { .. } => "failed",
            };
            SubagentSnapshot {
                agent_id: entry.metadata.agent_id,
                subagent_session_id: entry.metadata.subagent_session_id,
                task_id: entry.task_id.as_str().to_string(),
                status,
            }
        })
        .collect();
    out.sort_by(|a, b| {
        a.agent_id
            .cmp(&b.agent_id)
            .then_with(|| a.task_id.cmp(&b.task_id))
    });
    out
}

/// Most-recent durable sessions surfaced in the roster when they are not in
/// the live registry (cold boot / later turn). Bounds prompt growth on
/// threads with a long delegation history.
const DURABLE_ROSTER_CAP: usize = 12;

/// Build the ambient `[active_subagents]` block prepended to a parent's turn
/// context. Returns `None` when the parent owns no sub-agents at all, so the
/// block only appears when it is actionable — turns for agents that never
/// spawn are untouched. Mirrors the thread-goal `[active_goal]` block: it
/// rides the per-turn context (not the cached system-prompt prefix), so it
/// reflects live status every turn.
///
/// The roster merges two sources:
/// 1. the in-memory registry (live async workers spawned this process), and
/// 2. the durable per-workspace `subagent_sessions` store — workers from
///    EARLIER turns / process lifetimes. Without this second source a
///    cold-booted parent had no idea its previous sub-agents existed and
///    would re-delegate from scratch instead of resuming by
///    `subagent_session_id` (the "fresh context from day 0" bug).
pub(crate) fn active_subagents_context_block(
    parent_session: &str,
    workspace_dir: &std::path::Path,
) -> Option<String> {
    let workers = snapshot_for_parent(parent_session);

    // Durable sessions not already represented by a live registry entry.
    let live_session_ids: std::collections::HashSet<String> = workers
        .iter()
        .filter_map(|w| w.subagent_session_id.clone())
        .collect();
    let store = crate::openhuman::agent::orchestration::subagent_sessions::SubagentSessionStore {
        workspace_dir: workspace_dir.to_path_buf(),
    };
    let durable: Vec<_> =
        match crate::openhuman::agent::orchestration::subagent_sessions::list_for_parent(
            &store,
            parent_session,
            None,
        ) {
            Ok(sessions) => sessions
                .into_iter()
                .filter(|s| {
                    use crate::openhuman::agent::orchestration::subagent_sessions::DurableSubagentStatus;
                    s.status != DurableSubagentStatus::Closed
                        && !live_session_ids.contains(&s.subagent_session_id)
                })
                .take(DURABLE_ROSTER_CAP)
                .collect(),
            Err(err) => {
                log::warn!(
                    "[running_subagents] durable roster load failed parent_session={parent_session} error={err}"
                );
                Vec::new()
            }
        };

    if workers.is_empty() && durable.is_empty() {
        return None;
    }
    let mut block = format!(
        "[active_subagents]\n\
         You have {} sub-agent worker(s) for this conversation (live and/or from earlier \
         turns). This is your authoritative roster — trust it over memory. Track each by \
         subagent_session_id; use wait_subagent to collect a `completed` one, steer_subagent \
         to redirect a `running` one, continue_subagent to answer an `awaiting_user` one or \
         to RESUME an `idle` one with a follow-up (it keeps its full prior context — do NOT \
         re-delegate the same task from scratch), close_subagent when done, and \
         list_subagents to re-enumerate. Never fabricate a result for a worker still running \
         or one that has failed.\n",
        workers.len() + durable.len()
    );
    for w in &workers {
        let session = w.subagent_session_id.as_deref().unwrap_or("(none)");
        block.push_str(&format!(
            "- {} · session={} · task={} · status={}\n",
            w.agent_id, session, w.task_id, w.status
        ));
    }
    for s in &durable {
        use crate::openhuman::agent::orchestration::subagent_sessions::DurableSubagentStatus;
        let status = match s.status {
            DurableSubagentStatus::Running => "running",
            DurableSubagentStatus::Idle => "idle",
            DurableSubagentStatus::AwaitingUser => "awaiting_user",
            DurableSubagentStatus::Failed => "failed",
            DurableSubagentStatus::Closed => "closed",
        };
        let task = s.current_task_id.as_deref().unwrap_or("(none)");
        block.push_str(&format!(
            "- {} · session={} · task={} · status={} · about: {}\n",
            s.agent_id, s.subagent_session_id, task, status, s.task_title
        ));
    }
    block.push_str("[/active_subagents]\n\n");
    Some(block)
}

/// Resolve a durable `subagent_session_id` to the currently-running transient
/// `task_id`, enforcing parent-session ownership.
pub(crate) fn task_id_for_session(
    subagent_session_id: &str,
    parent_session: &str,
) -> Result<String, WaitError> {
    let mut saw_unowned = false;
    let mut owned_terminal: Option<String> = None;
    for snapshot in registry()
        .snapshots(None)
        .expect("detached task registry lock poisoned")
        .into_iter()
        .filter(|snapshot| {
            snapshot.metadata.subagent_session_id.as_deref() == Some(subagent_session_id)
        })
    {
        if snapshot.owner_id != parent_session {
            saw_unowned = true;
            continue;
        }
        let task_id = snapshot.task_id.as_str().to_string();
        if !snapshot.status.is_terminal() {
            return Ok(task_id);
        }
        owned_terminal.get_or_insert(task_id);
    }
    if let Some(task_id) = owned_terminal {
        return Ok(task_id);
    }
    if saw_unowned {
        return Err(WaitError::NotOwned);
    }
    Err(WaitError::Unknown)
}

pub(crate) fn task_id_for_session_in_workspace(
    subagent_session_id: &str,
    parent_session: &str,
    workspace_dir: &Path,
) -> Result<String, WaitError> {
    match task_id_for_session(subagent_session_id, parent_session) {
        Ok(task_id) => return Ok(task_id),
        Err(WaitError::NotOwned) => return Err(WaitError::NotOwned),
        Err(WaitError::Unknown) => {}
    }

    let mut saw_unowned = false;
    let mut matches: Vec<OrchestrationTaskRecord> = list_task_records(workspace_dir)
        .into_iter()
        .filter(|record| record_subagent_session_id(record) == Some(subagent_session_id))
        .collect();
    matches.sort_by_key(|item| std::cmp::Reverse(item.updated_at));

    for record in matches {
        if record_parent_session(&record) != Some(parent_session) {
            saw_unowned = true;
            continue;
        }
        let task_id = record.spec.task_id.as_str().to_string();
        log::debug!(
            "[running_subagents] resolved session from task store subagent_session_id={} task_id={} workspace_dir={}",
            subagent_session_id,
            task_id,
            workspace_dir.display()
        );
        return Ok(task_id);
    }
    if saw_unowned {
        return Err(WaitError::NotOwned);
    }
    Err(WaitError::Unknown)
}

pub(crate) fn resume_ref_for_task(
    task_id: &str,
    parent_session: &str,
) -> Result<SubagentResumeRef, WaitError> {
    let snapshot = registry()
        .snapshot(&TaskId::new(task_id), parent_session)
        .map_err(wait_error_from_registry)?;
    Ok(SubagentResumeRef {
        task_id: task_id.to_string(),
        agent_id: snapshot.metadata.agent_id,
        subagent_session_id: snapshot.metadata.subagent_session_id,
    })
}

pub(crate) fn resume_ref_for_task_in_workspace(
    task_id: &str,
    parent_session: &str,
    workspace_dir: &Path,
) -> Result<SubagentResumeRef, WaitError> {
    match resume_ref_for_task(task_id, parent_session) {
        Ok(reference) => return Ok(reference),
        Err(WaitError::NotOwned) => return Err(WaitError::NotOwned),
        Err(WaitError::Unknown) => {}
    }

    let record = task_record_for_task_in_workspace(workspace_dir, task_id, parent_session)?;
    log::debug!(
        "[running_subagents] resolved resume ref from task store task_id={} workspace_dir={}",
        task_id,
        workspace_dir.display()
    );
    Ok(SubagentResumeRef {
        task_id: task_id.to_string(),
        agent_id: record_agent_id(&record),
        subagent_session_id: record_subagent_session_id(&record).map(ToOwned::to_owned),
    })
}

/// Why a steer could not be delivered.
#[derive(Debug, PartialEq, Eq)]
pub enum SteerError {
    /// No such sub-agent — never existed, or already finished and pruned.
    Unknown,
    /// The caller's `parent_session` does not own this sub-agent.
    NotOwned,
    /// The sub-agent already reached a terminal status.
    AlreadyDone,
}

fn steering_command_for_mode(mode: QueueMode, text: String) -> Option<SteeringCommand> {
    match mode {
        QueueMode::Steer => Some(SteeringCommand::InjectMessage(TaMessage::user(format!(
            "[User steering message]: {text}"
        )))),
        QueueMode::Collect => Some(SteeringCommand::InjectMessage(TaMessage::user(format!(
            "[Additional context from user]: {text}"
        )))),
        QueueMode::Interrupt | QueueMode::Followup | QueueMode::Parallel => None,
    }
}

fn send_registered_steering(
    handle: &tinyagents::harness::steering::SteeringHandle,
    text: String,
    mode: QueueMode,
) -> bool {
    let Some(command) = steering_command_for_mode(mode, text) else {
        return false;
    };
    handle.send(command);
    true
}

/// Crate-native steering directives beyond the `InjectMessage`/collect lanes.
///
/// These map 1:1 onto the tinyagents [`SteeringCommand`] control variants that
/// the crate exposes (`Redirect`, `Pause`, `Resume`, `Cancel`). They are
/// delivered **only** through a registered [`SteeringHandle`] and therefore land
/// only at a safe loop boundary (the crate drains before each model call) —
/// never mid-stream, and never through the `RunQueue` fallback (which has no
/// equivalent lane). Approval/security is never bypassed: `Redirect` lowers to a
/// system instruction the normal approval-gated loop still governs, and
/// `Pause`/`Resume`/`Cancel` are pure control-flow.
///
/// The crate's `SetMetadata` command is intentionally *not* mapped here: no
/// OpenHuman control surface owns run-metadata mutation yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SteeringDirective {
    /// Redirect the run toward a new instruction (`SteeringCommand::Redirect`).
    Redirect(String),
    /// Cooperatively pause at the next checkpoint (`SteeringCommand::Pause`).
    Pause,
    /// Clear a pending pause (`SteeringCommand::Resume`).
    Resume,
    /// Cooperatively terminate at the next checkpoint (`SteeringCommand::Cancel`) —
    /// a graceful, safe-boundary alternative to the hard `AbortHandle` cancel.
    Cancel,
}

impl SteeringDirective {
    fn kind(&self) -> SteeringCommandKind {
        match self {
            SteeringDirective::Redirect(_) => SteeringCommandKind::Redirect,
            SteeringDirective::Pause => SteeringCommandKind::Pause,
            SteeringDirective::Resume => SteeringCommandKind::Resume,
            SteeringDirective::Cancel => SteeringCommandKind::Cancel,
        }
    }

    fn into_command(self) -> SteeringCommand {
        match self {
            SteeringDirective::Redirect(instruction) => SteeringCommand::Redirect { instruction },
            SteeringDirective::Pause => SteeringCommand::Pause,
            SteeringDirective::Resume => SteeringCommand::Resume,
            SteeringDirective::Cancel => SteeringCommand::Cancel,
        }
    }
}

/// Why a crate-native steering directive could not be delivered.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SteerDirectiveError {
    /// No such sub-agent — never existed, or already finished and pruned.
    Unknown,
    /// The caller's `parent_session` does not own this sub-agent.
    NotOwned,
    /// The sub-agent already reached a terminal status.
    AlreadyDone,
    /// The sub-agent has no live crate-native `SteeringHandle` registered
    /// (e.g. a legacy `RunQueue`-only run), so control-flow steering that has no
    /// `RunQueue` lane cannot be delivered.
    NoRegisteredHandle,
    /// The run's [`SteeringPolicy`] does not permit this directive's command
    /// kind. Enqueuing it anyway would abort the run with
    /// `TinyAgentsError::Steering`, so we refuse up front.
    PolicyRejected,
}

/// Deliver a crate-native control-flow [`SteeringDirective`] to a running
/// sub-agent through its registered TinyAgents [`SteeringHandle`].
///
/// Unlike [`steer`], this has **no** `RunQueue` fallback: the crate control
/// variants (`Redirect`/`Pause`/`Resume`/`Cancel`) have no OpenHuman queue lane,
/// so a run must have a live registered handle to receive them. The directive's
/// command kind is checked against the run's own `SteeringPolicy` *before*
/// enqueue — a disallowed command would otherwise abort the run — so this can
/// never smuggle a control kind past a policy that a tighter run class installed.
pub(crate) fn steer_directive(
    task_id: &str,
    parent_session: &str,
    directive: SteeringDirective,
) -> Result<(), SteerDirectiveError> {
    let handle = registry()
        .steering_handle(&TaskId::new(task_id), parent_session)
        .map_err(steer_directive_error_from_registry)?;
    let kind = directive.kind();
    if !handle.policy().is_allowed(kind) {
        log::warn!(
            "[running_subagents] directive rejected by run policy task_id={} kind={}",
            task_id,
            kind.as_str()
        );
        return Err(SteerDirectiveError::PolicyRejected);
    }
    handle.send(directive.into_command());
    log::info!(
        "[running_subagents] steered task_id={} directive={} via=tinyagents_registry",
        task_id,
        kind.as_str()
    );
    Ok(())
}

/// Inject a message into a running sub-agent. Prefer the crate-native
/// TinyAgents steering registry when the child run has registered its live
/// handle, and fall back to the OpenHuman `RunQueue` compatibility path.
pub async fn steer(
    task_id: &str,
    parent_session: &str,
    text: String,
    mode: QueueMode,
) -> Result<(), SteerError> {
    let task_id_key = TaskId::new(task_id);
    let snapshot = registry()
        .snapshot(&task_id_key, parent_session)
        .map_err(steer_error_from_registry)?;
    if snapshot.status.is_terminal() {
        return Err(SteerError::AlreadyDone);
    }

    let steered_via_registry = registry()
        .steering_handle(&task_id_key, parent_session)
        .map(|handle| send_registered_steering(&handle, text.clone(), mode))
        .unwrap_or(false);
    if steered_via_registry {
        log::info!(
            "[running_subagents] steered task_id={} mode={} via=tinyagents_registry",
            task_id,
            mode
        );
        return Ok(());
    }

    snapshot
        .metadata
        .run_queue
        .push(QueuedMessage {
            text,
            mode,
            client_id: "steer_subagent".to_string(),
            thread_id: task_id.to_string(),
            queued_at_ms: now_ms(),
            model_override: None,
            temperature: None,
            profile_id: None,
            locale: None,
        })
        .await;
    log::info!(
        "[running_subagents] steered task_id={} mode={}",
        task_id,
        mode
    );
    Ok(())
}

/// Trusted-control variant used by JSON-RPC sub-agent controls.
///
/// This intentionally does not require the caller to provide `parent_session`:
/// the RPC layer is already bearer-protected and mirrors the existing
/// `subagent_cancel` control surface, which can abort a task by id. The function
/// still refuses unknown or terminal tasks and never logs the steered text.
pub(crate) async fn steer_control(
    task_id: &str,
    text: String,
    mode: QueueMode,
) -> Result<(), SteerError> {
    let task_id_key = TaskId::new(task_id);
    let snapshot = registry()
        .snapshot_trusted(&task_id_key)
        .map_err(steer_error_from_registry)?;
    if snapshot.status.is_terminal() {
        return Err(SteerError::AlreadyDone);
    }

    let steered_via_registry = registry()
        .steering_handle_trusted(&task_id_key)
        .map(|handle| send_registered_steering(&handle, text.clone(), mode))
        .unwrap_or(false);
    if steered_via_registry {
        log::info!(
            "[running_subagents] control_steered task_id={} mode={} via=tinyagents_registry",
            task_id,
            mode
        );
        return Ok(());
    }

    snapshot
        .metadata
        .run_queue
        .push(QueuedMessage {
            text,
            mode,
            client_id: "subagent_control_rpc".to_string(),
            thread_id: task_id.to_string(),
            queued_at_ms: now_ms(),
            model_override: None,
            temperature: None,
            profile_id: None,
            locale: None,
        })
        .await;
    log::info!(
        "[running_subagents] control_steered task_id={} mode={}",
        task_id,
        mode
    );
    Ok(())
}

/// Why a wait could not be set up.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum WaitError {
    Unknown,
    NotOwned,
}

fn wait_error_from_registry(error: DetachedTaskRegistryError) -> WaitError {
    match error {
        DetachedTaskRegistryError::NotOwned => WaitError::NotOwned,
        _ => WaitError::Unknown,
    }
}

fn steer_error_from_registry(error: DetachedTaskRegistryError) -> SteerError {
    match error {
        DetachedTaskRegistryError::NotOwned => SteerError::NotOwned,
        DetachedTaskRegistryError::AlreadyDone => SteerError::AlreadyDone,
        _ => SteerError::Unknown,
    }
}

fn steer_directive_error_from_registry(error: DetachedTaskRegistryError) -> SteerDirectiveError {
    match error {
        DetachedTaskRegistryError::NotOwned => SteerDirectiveError::NotOwned,
        DetachedTaskRegistryError::AlreadyDone => SteerDirectiveError::AlreadyDone,
        DetachedTaskRegistryError::NoSteeringHandle => SteerDirectiveError::NoRegisteredHandle,
        _ => SteerDirectiveError::Unknown,
    }
}

/// Result of waiting on a sub-agent.
#[derive(Debug)]
pub(crate) enum WaitOutcome {
    /// The sub-agent reached a terminal status (entry pruned).
    Terminal(SubagentStatus),
    /// The timeout elapsed first; the entry is left intact so the parent can
    /// wait again. Carries the latest (non-terminal) status snapshot.
    TimedOut(SubagentStatus),
}

/// Block until `task_id` reaches a terminal status or `timeout` elapses.
pub(crate) async fn wait(
    task_id: &str,
    parent_session: &str,
    timeout: Duration,
) -> Result<WaitOutcome, WaitError> {
    match registry()
        .wait(&TaskId::new(task_id), parent_session, timeout)
        .await
    {
        Ok(DetachedTaskWaitOutcome::Terminal(status)) => Ok(WaitOutcome::Terminal(status)),
        Ok(DetachedTaskWaitOutcome::TimedOut(status)) => Ok(WaitOutcome::TimedOut(status)),
        Err(DetachedTaskRegistryError::StatusChannelClosed) => {
            Ok(WaitOutcome::Terminal(SubagentStatus::Failed {
                error: "sub-agent task ended without reporting a result".to_string(),
            }))
        }
        Err(error) => Err(wait_error_from_registry(error)),
    }
}

pub(crate) async fn wait_in_workspace(
    task_id: &str,
    parent_session: &str,
    workspace_dir: &Path,
    timeout: Duration,
) -> Result<WaitOutcome, WaitError> {
    match wait(task_id, parent_session, timeout).await {
        Ok(outcome) => return Ok(outcome),
        Err(WaitError::NotOwned) => return Err(WaitError::NotOwned),
        Err(WaitError::Unknown) => {}
    }

    let record = task_record_for_task_in_workspace(workspace_dir, task_id, parent_session)?;
    log::debug!(
        "[running_subagents] resolved wait from task store task_id={} status={} workspace_dir={}",
        task_id,
        task_status_label(record.status),
        workspace_dir.display()
    );
    Ok(record_to_status(record))
}

/// Metadata captured when a sub-agent is cancelled, so the caller can surface
/// the cancellation back in the parent chat (record a "cancelled" completion
/// for idle-gated delivery).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CancelledSubagent {
    pub(crate) agent_id: String,
    pub(crate) parent_session: String,
    pub(crate) subagent_session_id: Option<String>,
    pub(crate) workspace_dir: PathBuf,
    pub(crate) parent_thread_id: Option<String>,
}

/// Abort and drop the sub-agent with `task_id`, returning its metadata so the
/// caller can deliver a "cancelled" notice into the parent chat. Returns `None`
/// if no such sub-agent is registered (already finished, or unknown id).
///
/// Unlike the parent-session-owned steering and close paths, this is keyed by
/// `task_id` alone with no ownership check — it backs the user-facing "Cancel"
/// affordance, and the desktop user owns every sub-agent in their own core.
pub(crate) fn cancel_by_task(task_id: &str) -> Option<CancelledSubagent> {
    let cancelled = registry().cancel_trusted(&TaskId::new(task_id)).ok()?;
    let metadata = cancelled.metadata;
    record_cancelled(&metadata.workspace_dir, task_id);
    log::debug!(
        "[running_subagents] cancel_by_task task_id={} agent_id={} parent_thread_id={:?} live_entries={}",
        task_id,
        metadata.agent_id,
        metadata.parent_thread_id,
        registry()
            .len()
            .expect("detached task registry lock poisoned")
    );
    Some(CancelledSubagent {
        agent_id: metadata.agent_id,
        parent_session: cancelled.owner_id,
        subagent_session_id: metadata.subagent_session_id,
        workspace_dir: metadata.workspace_dir,
        parent_thread_id: metadata.parent_thread_id,
    })
}

pub(crate) fn cancel_by_session(
    subagent_session_id: &str,
    parent_session: &str,
) -> Option<CancelledSubagent> {
    let task_id = task_id_for_session(subagent_session_id, parent_session).ok()?;
    cancel_by_task(&task_id)
}

pub(crate) fn cancel_by_session_in_workspace(
    subagent_session_id: &str,
    parent_session: &str,
    workspace_dir: &Path,
) -> Option<CancelledSubagent> {
    let task_id =
        task_id_for_session_in_workspace(subagent_session_id, parent_session, workspace_dir)
            .ok()?;
    cancel_by_task(&task_id)
}

/// Abort and drop every running sub-agent whose parent chat thread is
/// `thread_id`. Called when that thread is deleted so detached children don't
/// keep running (and later try to deliver) against a thread that no longer
/// exists. Returns the number of sub-agents cancelled.
pub(crate) fn cancel_for_thread(thread_id: &str) -> usize {
    let cancelled = registry()
        .cancel_where(|metadata| metadata.parent_thread_id.as_deref() == Some(thread_id))
        .expect("detached task registry lock poisoned");
    for entry in &cancelled {
        record_cancelled(&entry.metadata.workspace_dir, entry.task_id.as_str());
    }
    let count = cancelled.len();
    log::debug!(
        "[running_subagents] cancel_for_thread thread_id={} cancelled={} live_entries={}",
        thread_id,
        count,
        registry()
            .len()
            .expect("detached task registry lock poisoned")
    );
    count
}

/// Abort and drop **every** registered sub-agent. Called on a full thread purge
/// where no parent thread survives. Returns the **distinct parent thread ids**
/// that had sub-agents, so the purge path can tombstone them in
/// [`super::background_completions`] and drop any straggler completion that wins
/// the cooperative-abort race. Headless sub-agents (no parent thread) are still
/// aborted but contribute no id.
pub(crate) fn cancel_all() -> Vec<String> {
    let cancelled = registry()
        .cancel_all()
        .expect("detached task registry lock poisoned");
    let count = cancelled.len();
    let mut thread_ids: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for entry in cancelled {
        record_cancelled(&entry.metadata.workspace_dir, entry.task_id.as_str());
        if let Some(thread_id) = entry.metadata.parent_thread_id {
            if seen.insert(thread_id.clone()) {
                thread_ids.push(thread_id);
            }
        }
    }
    log::debug!(
        "[running_subagents] cancel_all cancelled={} distinct_threads={}",
        count,
        thread_ids.len()
    );
    thread_ids
}

fn prune(task_id: &str) {
    let _ = registry().cancel_trusted(&TaskId::new(task_id));
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
#[path = "running_subagents_tests.rs"]
mod tests;
