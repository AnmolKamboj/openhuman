# Diff Coverage
## Diff: origin/main...HEAD, staged and unstaged changes

- src/openhuman/agent/harness/subagent_runner/handoff&#46;rs (0.0%): Missing lines 58-64
- src/openhuman/agent/harness/tool_filter&#46;rs (100%)
- src/openhuman/agent/orchestration/agent_teams/ops&#46;rs (100%)
- src/openhuman/agent/orchestration/agent_teams/runtime&#46;rs (100%)
- src/openhuman/agent/orchestration/ops&#46;rs (81.6%): Missing lines 231-233,238,250,275,301,428,434-436,463,483-502,520-526,538
- src/openhuman/agent/orchestration/workflow_runs/engine&#46;rs (52.9%): Missing lines 676-683
- src/openhuman/agent/orchestration/workflow_runs/ops&#46;rs (100%)
- src/openhuman/agent/tinyagents/delegation&#46;rs (72.7%): Missing lines 99,115-116,137-138,154
- src/openhuman/mcp/server/tools/dispatch&#46;rs (0.0%): Missing lines 240-243

## Summary

- **Total**: 317 lines
- **Missing**: 65 lines
- **Coverage**: 79%



## src/openhuman/agent/harness/subagent_runner/handoff&#46;rs

```
  54     let effective_threshold = std::env::var("OPENHUMAN_TEST_HANDOFF_THRESHOLD_TOKENS")
  55         .ok()
  56         .and_then(|v| v.parse::<usize>().ok())
  57         .unwrap_or(HANDOFF_OVERSIZE_THRESHOLD_TOKENS);
! 58     tinyagents::harness::handoff::apply_handoff(
! 59         cache,
! 60         tool_name,
! 61         task_id,
! 62         agent_id,
! 63         result_text,
! 64         effective_threshold,
  65     )
  66 }
```


---



## src/openhuman/agent/orchestration/ops&#46;rs

```
  227     /// `wait_agents` resolves with a cancellation rather than a closed channel.
  228     pub async fn abort_all(&self) {
  229         let cancelled = match self.registry.cancel_all() {
  230             Ok(cancelled) => cancelled,
! 231             Err(err) => {
! 232                 log::warn!("[agent_orchestration] abort_all could not drain registry: {err:?}");
! 233                 return;
  234             }
  235         };
  236         for entry in &cancelled {
  237             if entry.status.is_terminal() {
! 238                 continue;
  239             }
  240             let _ = entry.metadata.status_tx.send(ChildState {
  241                 status: OrchestrationTaskStatus::Cancelled,
  242                 result_summary: entry.status.result_summary.clone(),
```


---


```
  246         }
  247         log::debug!(
  248             "[agent_orchestration] abort_all session={} cancelled={}",
  249             self.session_id,
! 250             cancelled.len()
  251         );
  252     }
  253 
  254     /// Wait one child to a terminal status, honouring the shared `deadline`.
```


---


```
  271                 DetachedTaskWaitOutcome::Terminal(state) => return Ok(state),
  272                 DetachedTaskWaitOutcome::TimedOut(state) => {
  273                     if deadline.is_some() {
  274                         return Ok(state);
! 275                     }
  276                     // Unbounded wait: the chunk expired, keep waiting.
  277                 }
  278             }
  279         }
```


---


```
  297         match err {
  298             DetachedTaskRegistryError::Unknown | DetachedTaskRegistryError::NotOwned => {
  299                 OrchestrationError::AgentNotFound(id.to_string())
  300             }
! 301             other => OrchestrationError::Registry(other),
  302         }
  303     }
  304 
  305     async fn spawn_agent_with_definition(
```


---


```
  424                 CancellationToken::new(),
  425                 handle.abort_handle(),
  426             )
  427             .map_err(|err| {
! 428                 log::error!("[agent_orchestration] duplicate detached child id: {err}");
  429                 // Registration failed, so the registry holds no record of this
  430                 // task and `abort_all`/`cancel_all` can never reach it. Dropping
  431                 // `handle` here would only detach the `JoinHandle` — the spawned
  432                 // task keeps running orphaned. Abort it explicitly so a failed
```


---


```
  430                 // task and `abort_all`/`cancel_all` can never reach it. Dropping
  431                 // `handle` here would only detach the `JoinHandle` — the spawned
  432                 // task keeps running orphaned. Abort it explicitly so a failed
  433                 // spawn never leaves a live, unreachable child behind.
! 434                 handle.abort();
! 435                 OrchestrationError::InvalidSpawnRequest
! 436             })?;
  437 
  438         log::debug!(
  439             "[agent_orchestration] spawned session={} orchestration_id={} agent_id={}",
  440             self.session_id,
```


---


```
  459     ) {
  460         // A cancelled child has already reached a terminal status via
  461         // `abort_all`; do not overwrite it with a late completion.
  462         if status_tx.borrow().is_terminal() {
! 463             return;
  464         }
  465 
  466         match result {
  467             Ok(outcome) => {
```


---


```
  479                     output_chars: outcome.output.chars().count(),
  480                     iterations: outcome.iterations,
  481                 });
  482                 if let Some(progress) = progress_sink {
! 483                     let _ = progress
! 484                         .send(AgentProgress::SubagentCompleted {
! 485                             agent_id: outcome.agent_id.clone(),
! 486                             task_id: orchestration_id.to_string(),
! 487                             elapsed_ms: outcome.elapsed.as_millis() as u64,
! 488                             iterations: outcome.iterations as u32,
! 489                             output_chars: outcome.output.chars().count(),
! 490                             output: outcome.output.clone(),
! 491                             // Not a dropped value: these three describe a
! 492                             // worker's *own* isolated checkout, and this path
! 493                             // never creates one — it only inherits the parent's
! 494                             // descriptor (above). `spawn_parallel_agents`
! 495                             // populates them from the descriptor it freshly
! 496                             // created per worker, and reports `None` for an
! 497                             // inherited one for the same reason.
! 498                             worktree_path: None,
! 499                             changed_files: Vec::new(),
! 500                             dirty_status: None,
! 501                         })
! 502                         .await;
  503                 }
  504             }
  505             Err(error) => {
  506                 let message = error.to_string();
```


---


```
  516                     agent_id: agent_id.to_string(),
  517                     error: message.clone(),
  518                 });
  519                 if let Some(progress) = progress_sink {
! 520                     let _ = progress
! 521                         .send(AgentProgress::SubagentFailed {
! 522                             agent_id: agent_id.to_string(),
! 523                             task_id: orchestration_id.to_string(),
! 524                             error: message,
! 525                         })
! 526                         .await;
  527                 }
  528             }
  529         }
  530     }
```


---


```
  534 fn mark_running(status_tx: &watch::Sender<ChildState>) {
  535     let updated_at = now();
  536     status_tx.send_if_modified(|state| {
  537         if state.is_terminal() || state.status == OrchestrationTaskStatus::Running {
! 538             return false;
  539         }
  540         state.status = OrchestrationTaskStatus::Running;
  541         state.updated_at = updated_at;
  542         true
```


---



## src/openhuman/agent/orchestration/workflow_runs/engine&#46;rs

```
  672                             )),
  673                         },
  674                         OrchestrationTaskStatus::Pending
  675                         | OrchestrationTaskStatus::Running
! 676                         | OrchestrationTaskStatus::Awaiting => PhaseWorkerOutcome {
! 677                             orchestration_id: Some(oid),
! 678                             output: None,
! 679                             error: Some(format!(
! 680                                 "child '{}' returned non-terminal status",
! 681                                 s.orchestration_id
! 682                             )),
! 683                         },
  684                     },
  685                     None => PhaseWorkerOutcome {
  686                         orchestration_id: Some(oid),
  687                         output: None,
```


---



## src/openhuman/agent/tinyagents/delegation&#46;rs

```
   95 where
   96     F: Fn(DelegationStage, DelegationState) -> Fut + Clone + Send + Sync + 'static,
   97     Fut: Future<Output = Result<DelegationStageOutput, String>> + Send + 'static,
   98 {
!  99     tinyagents::graph::delegation::run_delegation(with_tracing_sink(config), run_stage).await
  100 }
  101 
  102 /// Run the delegation graph and report whether it finalized or parked on a
  103 /// durable human-approval interrupt.
```


---


```
  111 where
  112     F: Fn(DelegationStage, DelegationState) -> Fut + Clone + Send + Sync + 'static,
  113     Fut: Future<Output = Result<DelegationStageOutput, String>> + Send + 'static,
  114 {
! 115     tinyagents::graph::delegation::run_delegation_durable(with_tracing_sink(config), run_stage)
! 116         .await
  117 }
  118 
  119 /// Resume a delegation graph parked on a durable human-approval interrupt,
  120 /// delivering the approver's `decision`.
```


---


```
  133 where
  134     F: Fn(DelegationStage, DelegationState) -> Fut + Clone + Send + Sync + 'static,
  135     Fut: Future<Output = Result<DelegationStageOutput, String>> + Send + 'static,
  136 {
! 137     tinyagents::graph::delegation::resume_delegation(with_tracing_sink(config), decision, run_stage)
! 138         .await
  139 }
  140 
  141 /// Run the delegation graph, resuming from the last checkpoint boundary when the
  142 /// configured thread has a live, compatible, non-terminal checkpoint, else
```


---


```
  150 where
  151     F: Fn(DelegationStage, DelegationState) -> Fut + Clone + Send + Sync + 'static,
  152     Fut: Future<Output = Result<DelegationStageOutput, String>> + Send + 'static,
  153 {
! 154     tinyagents::graph::delegation::run_or_resume_delegation(with_tracing_sink(config), run_stage)
  155         .await
  156 }
  157 
  158 #[cfg(test)]
```


---



## src/openhuman/mcp/server/tools/dispatch&#46;rs

```
  236 }
  237 
  238 async fn core_tool_instructions() -> Result<Value, ToolCallError> {
  239     let agent = build_orchestrator_agent().await?;
! 240     let schemas: Vec<_> = agent.tool_specs().iter().map(spec_to_schema).collect();
! 241     Ok(tool_text_success(
! 242         tinyagents::harness::tool::prompt_tool_instructions(&schemas),
! 243     ))
  244 }
  245 
  246 async fn list_subagents() -> Result<Value, ToolCallError> {
  247     let config = load_config_and_init_registry().await?;
```


---


