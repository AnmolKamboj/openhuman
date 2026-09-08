fn handle_reclaim_stale(params: Map<String, Value>) -> ControllerFuture {
    Box::pin(async move {
        let p = parse::<ReclaimStaleParams>(params)?;
        let loc = thread_location(&p.thread_id).await?;
        let limits = runs::RunLimits {
            heartbeat_stale_secs: p
                .heartbeat_stale_secs
                .unwrap_or(runs::DEFAULT_HEARTBEAT_STALE_SECS),
            claim_ttl_secs: p.claim_ttl_secs.unwrap_or(runs::DEFAULT_CLAIM_TTL_SECS),
            max_reclaim_count: p
                .max_reclaim_count
                .unwrap_or(runs::DEFAULT_MAX_RECLAIM_COUNT),
        };
        tracing::debug!(
            thread_id = %p.thread_id,
            ?limits,
            "[rpc][todos] reclaim_stale entry"
        );
        let result = runs::reclaim_stale(&loc, &limits).await?;
        serde_json::to_value(&result).map_err(|e| format!("serialize reclaim result: {e}"))
    })
}

// ── helpers ──────────────────────────────────────────────────────────

async fn thread_location(thread_id: &str) -> Result<BoardLocation, String> {
    let trimmed = thread_id.trim();
    if trimmed.is_empty() {
        return Err("thread_id must not be empty".to_string());
    }
    let config = crate::openhuman::config::Config::load_or_init()
        .await
        .map_err(|e| format!("load config: {e}"))?;
    Ok(BoardLocation::Thread {
        workspace_dir: config.workspace_dir,
        thread_id: trimmed.to_string(),
    })
}

fn parse<T: DeserializeOwned>(params: Map<String, Value>) -> Result<T, String> {
    serde_json::from_value(Value::Object(params)).map_err(|e| format!("invalid params: {e}"))
}

fn snapshot_to_json(snap: TodosSnapshot) -> Result<Value, String> {
    serde_json::to_value(&snap).map_err(|e| format!("serialize snapshot: {e}"))
}

fn thread_id_input() -> FieldSchema {
    FieldSchema {
        name: "thread_id",
        ty: TypeSchema::String,
        comment: "Conversation thread identifier (same id used by `threads.task_board_*`).",
        required: true,
    }
}

fn required_string(name: &'static str, comment: &'static str) -> FieldSchema {
    FieldSchema {
        name,
        ty: TypeSchema::String,
        comment,
        required: true,
    }
}

fn optional_string(name: &'static str, comment: &'static str) -> FieldSchema {
    FieldSchema {
        name,
        ty: TypeSchema::Option(Box::new(TypeSchema::String)),
        comment,
        required: false,
    }
}

fn string_array_input(name: &'static str, comment: &'static str) -> FieldSchema {
    FieldSchema {
        name,
        ty: TypeSchema::Option(Box::new(TypeSchema::Array(Box::new(TypeSchema::String)))),
        comment,
        required: false,
    }
}

/// The `cards` input of `todos.replace`, spelled out field by field.
///
/// This was a bare `TypeSchema::Json` commented "Array of card objects (id may
/// be empty — server generates)", which is not enough to construct one and was
/// actively misleading in two ways (#6087):
///
///  * `handle_replace` deserializes each entry into `TaskBoardCard`, whose
///    `id`, `title` and `status` carry **no** `#[serde(default)]` — so all
///    three keys are mandatory. "id may be empty" is true of the *string* and
///    false of the *key*: omitting it fails with ``missing field `id` ``.
///  * the text field is `title`, while the sibling `todos.add` / `todos.edit`
///    inputs in this same namespace call it `content`. A caller who reached for
///    the namespace's own vocabulary got ``missing field `title` ``.
///
/// Names below are the wire names: `TaskBoardCard` is
/// `#[serde(rename_all = "camelCase")]` and `TaskCardStatus` is
/// `#[serde(rename_all = "snake_case")]`, so the declaration must use
/// `assignedAgent` (not `assigned_agent`) and `in_progress` (not `inProgress`).
/// Every optional field carries `#[serde(default)]` upstream, so the optional
/// markings here are load-bearing rather than decorative.
fn replace_cards_input() -> FieldSchema {
    FieldSchema {
        name: "cards",
        ty: TypeSchema::Array(Box::new(TypeSchema::Object {
            fields: vec![
                FieldSchema {
                    name: "id",
                    ty: TypeSchema::String,
                    comment: "Stable card id (`task-<n>`). The KEY is required; \
                              pass an empty string to have the server generate one.",
                    required: true,
                },
                FieldSchema {
                    name: "title",
                    ty: TypeSchema::String,
                    comment: "One-line title. NOTE: `todos.add` / `todos.edit` \
                              call this same field `content`; here it is `title`.",
                    required: true,
                },
                FieldSchema {
                    name: "status",
                    ty: TypeSchema::Enum {
                        variants: vec![
                            "todo",
                            "awaiting_approval",
                            "ready",
                            "in_progress",
                            "blocked",
                            "done",
                            "rejected",
                        ],
                    },
                    comment: "Lifecycle state. At most one card may be `in_progress`.",
                    required: true,
                },
                optional_string("objective", "Richer objective for the card."),
                string_array_input("plan", "Ordered plan steps."),
                optional_string("assignedAgent", "Agent assigned to run this card."),
                string_array_input("allowedTools", "Tools the assigned agent may use."),
                optional_string(
                    "approvalMode",
                    "Plan-approval mode, when the card is gated.",
                ),
                string_array_input(
                    "acceptanceCriteria",
                    "Acceptance criteria that define \"done\".",
                ),
                string_array_input("evidence", "Evidence gathered toward completion."),
                optional_string("notes", "Free-form notes."),
                optional_string("blocker", "Reason, when `status == blocked`."),
                optional_string(
                    "sessionThreadId",
                    "Thread the card's own agent session runs in.",
                ),
                FieldSchema {
                    name: "sourceMetadata",
                    ty: TypeSchema::Option(Box::new(TypeSchema::Json)),
                    comment: "Provenance blob carried through untouched.",
                    required: false,
                },
                FieldSchema {
                    name: "order",
                    ty: TypeSchema::Option(Box::new(TypeSchema::U64)),
                    comment: "Sort position; defaults to 0.",
                    required: false,
                },
                optional_string("updatedAt", "Last-update stamp; server-maintained."),
            ],
        })),
        comment: "Full replacement list. Each entry MUST carry `id`, `title` and \
                  `status`; every other field is optional. Note `title`, not \
                  `content` — see the field comments.",
        required: true,
    }
}

fn snapshot_output() -> FieldSchema {
    FieldSchema {
        name: "snapshot",
        ty: TypeSchema::Json,
        comment: "Object with `threadId`, `cards`, and a `markdown` rendering of the list.",
        required: true,
    }
}
