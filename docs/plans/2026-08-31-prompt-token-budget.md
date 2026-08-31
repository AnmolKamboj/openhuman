# Prompt / token-budget audit and plan

Date: 2026-08-31
Reference implementation studied: [nousresearch/hermes-agent](https://github.com/NousResearch/hermes-agent)

## Measured baseline

Numbers from `openhuman-core agent dump-all` run against the signed-in
workspace on this machine (`~/.openhuman/users/…/workspace`), 2026-08-31.
Token figures are bytes/4.

| Agent | Prompt text | Tool schemas (as dumped) | Tools |
| --- | --- | --- | --- |
| orchestrator | 37,703 B (~9,400 tok) | 175,476 B (~43,900 tok) | 220 |
| workflow_builder | 80,353 B (~20,100 tok) | — | — |
| integrations_agent@gmail | 23,667 B (~5,900 tok) | — | 25 |
| median specialist | ~7,000 B (~1,750 tok) | — | 2–6 (declared belt) |

Applying the default toolpack posture (`GroupMode::Withheld` for every
group) to the orchestrator's dumped catalogue:

```
220 tools                 175,476 B  ~43,869 tok
  - workflows   32 tools   40,803 B  ~10,200 tok
  - system      19 tools    5,058 B   ~1,264 tok
  - integrations 11 tools    5,484 B   ~1,371 tok
  - skills       9 tools    5,381 B   ~1,345 tok
  - composio     6 tools    3,867 B     ~966 tok
  - goals        3 tools    1,254 B     ~313 tok
  - crypto       1 tool     1,605 B     ~401 tok
= 139 tools advertised    112,024 B  ~28,006 tok
```

**The orchestrator's fixed per-turn prefix is roughly 37,400 tokens
before the user has typed anything** — ~28k of tool schema and ~9.4k of
prompt text. The prompt text is the smaller half of the problem.

Prompt-text breakdown (orchestrator, largest first):

| Section | ~tokens |
| --- | --- |
| Delegation (direct-first) | 1,978 |
| Rules | 1,325 |
| Installed Skills | 1,078 |
| …Running several workers at once | 632 |
| Connected Integrations | 551 |
| Writing style | 475 |
| …Capability questions about connected toolkits | 430 |
| Grounding and tool use | 399 |
| When OpenHuman is criticized | 268 |
| everything else (19 sections) | ~2,200 |

Single most expensive tool schemas: `propose_workflow` 1,892 tok,
`spawn_subagent` 909, `cron_add` 790, `memory_tree` 781,
`edit_workflow` 700, `suggest_workflows` 634. The top ten tools are
~6,500 tokens — more than every prose section except Delegation.

---

## Findings

### F1 — There is no prompt caching anywhere in the stack. (critical)

`grep -rn cache_control` returns **zero hits** across `src/`,
`vendor/tinyagents`, and `backend/src`. Every turn re-pays full input
price on the whole ~37k prefix.

Hermes treats this as the central design constraint. `agent/system_prompt.py`
builds the prompt as three ordered cache tiers — `stable` / `context` /
`volatile` — and its docstring states the rule plainly: *"Hermes never
re-renders parts of this string mid-session — that's the only way to keep
upstream prompt caches warm across turns."* It backs that with
`agent/prompt_caching.py`, `prompt_cache_boundary.py` (builder-declared
stable prefixes, so a webhook/cron scaffold gets a breakpoint at the exact
byte where the volatile tail starts) and `prompt_cache_scope.py`.

OpenHuman already does the hard half: `session/turn/core.rs:241` builds the
system prompt once on turn 1 and reuses it verbatim thereafter, with a
comment about preserving the KV prefix. What is missing is telling the
provider about it.

**This is the highest-value item on the list by a wide margin** — roughly a
90% discount on ~37k tokens of every turn on Anthropic-family models, for
no behavioural change.

### F2 — Tool schemas are 3× the prompt text, and nothing measures them.

Everything in the repo that talks about prompt size talks about prose. The
schemas are the actual budget. `propose_workflow` at 1,892 tokens is larger
than any single prose section; nobody would ship a 1,892-token prose block
without noticing.

### F3 — The dumper does not model the wire, so the budget is invisible.

`src/openhuman/agent/debug/mod.rs::render_via_session` dumps `agent.tools()`
— the whole registry — and passes `empty_visible` as the visible set. It
never calls `strip_packed_from_visible` and never applies the agent's
declared `[tools] named` belt. Consequences:

- Every specialist reports `tools=197` / 47k tok in `SUMMARY.txt`. The real
  belts are 2–6 tools (`researcher` = `web_search_tool`, `web_fetch`;
  `critic` = `read_diff`, `run_linter`, `run_tests`, `file_read`).
- The orchestrator's 220 overstates its real 139.
- There is no per-section or per-tool byte breakdown at all.

Hermes ships `hermes prompt-size` (`hermes_cli/prompt_size.py`, with
`--json`): it builds a real offline agent with dummy credentials so the
numbers match the wire, then reports system-prompt total, the
`<available_skills>` index, memory + profile, and tool-schema JSON, plus a
per-skill table. Its module docstring names the goal: *"Lets users see where
their fixed prompt budget goes … without parsing a saved session JSON by
hand."*

### F4 — Toolpack withholding is all-or-nothing; there is no middle setting.

`GroupMode` is `Advertised` / `Withheld` / `Off`. Withheld removes the
schema entirely and substitutes a `load_skill` round trip. There is no way
to say "keep the name, drop the description and parameter schema", which is
the cheap 80% for a tool the model needs to *know exists* but rarely calls.

Hermes does exactly this for its skills index: a category outside the
current posture renders as one `category [names only]: a, b, c` line, and
the code carries an explicit warning not to remove entries entirely —
*"agent-created skills are the model's project memory, and models don't
reach for skills_list to rediscover what the index stops showing them."*

### F5 — Section order puts volatile content in the cache prefix.

`SystemPromptBuilder::with_defaults()` orders: Identity, **UserFiles
(PROFILE.md, MEMORY.md)**, AgentsInstructions, **UserMemory**, Tools,
Safety, Workspace, DateTime, Runtime. The two most volatile inputs in the
whole prompt sit at positions 2 and 4, ahead of every byte-stable block.
Any memory write invalidates the entire remainder of the prefix.

Hermes puts precisely these in the volatile tier: *"volatile — skills index,
memory snapshot, user profile, external memory provider block, timestamp
line"*, rendered last, with the ordering rationale spelled out for both
explicit-breakpoint and longest-prefix backends.

Smaller instance of the same bug in the current render: `## Workspace`
(~185 tok) and `# Writing style` (~475 tok) are both byte-stable but are
emitted *after* `## Current Date & Time`.

### F6 — Flat character caps, no budget.

`BOOTSTRAP_MAX_CHARS = 20_000` applies per injected file, and both the
global and the project `AGENTS.md` get one — up to ~10k tokens of project
instructions alone. `USER_FILE_MAX_CHARS = 2_000`. There is no total
budget, no scaling to the resolved context window, and no test asserting a
ceiling on the assembled prompt.

Hermes resolves the model's context window once per session and scales its
context-file caps to it (`_dynamic_context_file_max_chars`), and surfaces
truncation as a user-visible status message rather than a log line.

### F7 — Prose bloat is real but second-order.

`workflow_builder` renders an 80KB / ~20k-token system prompt — twice the
orchestrator's. Delegation (1,978) + Rules (1,325) + the two `###`
subsections under them (1,062) are ~4.4k tokens of the orchestrator's 9.4k.
Worth trimming, but it is ~3k of a 37k problem. Do it after F1–F5.

---

## Plan

### P0 — Make the budget visible (prerequisite for everything else)

1. Fix `render_via_session` to model the wire: apply the definition's
   `ToolScope` belt and call `strip_packed_from_visible(&mut visible,
   agent_id)` before collecting specs. The `.tools.json` must be the
   advertised set.
2. Add `openhuman-core agent prompt-size [--agent <id>] [--json]`, modelled
   on `hermes_cli/prompt_size.py`: total, per-section bytes/tokens, tool
   schemas with a per-tool table, skills index, memory + profile, AGENTS.md
   layers. `--json` for CI.
3. Add `scripts/prompt-budget.limits` + a ratchet lane, same shape as
   `scripts/kernel-floor.limits` / `check-kernel-floor.sh` — fail on growth,
   fail on an unratcheted improvement. An unmeasured prompt grows.

*Exit criteria: `SUMMARY.txt` shows `researcher tools=2`, and CI fails if
the orchestrator prefix grows.*

### P1 — Prompt caching (largest single win)

4. Split `build_system_prompt` into three tiers mirroring Hermes'
   `build_system_prompt_parts`: `stable` (identity, role, grounding, safety,
   style, delegation, rules), `context` (AGENTS.md layers, workspace), and
   `volatile` (skills index, connected integrations, memory, profile,
   date/time). Join for the wire; keep the tier boundaries as offsets.
5. Emit `cache_control: {type: "ephemeral"}` breakpoints at the tier
   boundaries for Anthropic-family providers, and place the tool array
   ahead of the system block so the schemas are inside the cached prefix.
   Add the equivalent for the backend's OpenAI-compatible proxy.
6. Reorder `SystemPromptBuilder::with_defaults()` to match the tiers —
   `UserFilesSection` and `UserMemorySection` move to the tail, after
   `DateTimeSection`; `WorkspaceSection` and the style block move ahead of
   it.

*Estimated: ~37k tokens/turn at ~10% of list price on cache hits.*

### P2 — Tool-schema diet

7. Add a per-tool schema budget to the P0 ratchet (suggested: warn > 400
   tok, fail > 800). Rewrite the six offenders above `600`; `propose_workflow`
   at 1,892 is the priority.
8. Add `GroupMode::NamesOnly` between `Advertised` and `Withheld`: renders
   the tool name and a one-line description, no parameter schema, with a
   note that `get_tool_contract` fetches the full signature. Default the
   `workflows` pack (10,200 tok on the orchestrator) to it.
9. Audit the orchestrator's 139 advertised tools against actual call
   frequency from Langfuse traces; demote the long tail.

### P3 — Budget-aware injection

10. Replace the flat `BOOTSTRAP_MAX_CHARS` with a cap scaled to the resolved
    context window, and give the *combined* injected-file set one budget
    rather than one cap each.
11. Surface truncation to the user as a status event, not only a log line.

### P4 — Prose trim

12. `workflow_builder` (~20k tok) — split the reference material out into a
    skill the agent loads on demand.
13. Trim Delegation and Rules on the orchestrator; both read as accreted
    rather than authored.

## Sequencing note

P0 before everything: three of these findings (F2, F3, F4) exist because the
numbers were never on screen. Land the measurement and the ratchet first,
then P1, which is worth more than P2–P4 combined.
