# OpenHuman (Anmol fork) — New Chat Context

> **Purpose:** Paste or `@`-reference this file at the start of a new Cursor chat so the agent has full fork context without re-reading the entire upstream repo or prior conversation.
>
> **Last updated:** 2026-08-14
> **Owner:** Anmol — software engineer, MS CS from Florida Atlantic University
> **Related:** [`../README.md`](../README.md) (upstream setup), Smriti context at `D:\Projects\smriti\docs\CONTEXT.md` (legacy Jarvis bot)

---

## One-line pitch

**OpenHuman** is Anmol's daily personal AI (desktop + Telegram) with a local Obsidian vault as the brain. This checkout is a fork of [tinyhumansai/openhuman](https://github.com/tinyhumansai/openhuman) that adds Cursor as an LLM provider and wires Anmol's Smriti vault into OpenHuman memory.

**Daily driver:** this fork, not the Jarvis/Smriti Python bot (`D:\Projects\smriti`). Smriti remains the vault structurer + job hunter; OpenHuman is the chat/runtime.

---

## What problem this fork solves

1. **Upstream OpenHuman has no Cursor provider.** Anmol's school Cursor account is the primary chat/coding LLM. This fork runs a local OpenAI-compatible sidecar (`scripts/cursor-bridge`) so OpenHuman can call Composer / Grok via `cursor:<model>`.
2. **The vault (brain) and the bot LTM were disconnected.** Identity lives in Obsidian; OpenHuman workspace had no `PROFILE.md` / `MEMORY.md`, Cursor native tool-calling was a no-op, and embeddings were the wrong size for the memory tree — so "what do you know about me?" answered from junk auto-citations.
3. **Installer `OpenHuman.exe` is upstream/old.** Local daily use must run the debug build from this repo.

---

## Locked architecture decisions

| Decision | Choice | Do not change without discussion |
|---|---|---|
| Daily runtime | **OpenHuman** (this fork) | Smriti/Jarvis is legacy for vault writes + jobs |
| Code repo | `D:\Projects\openhuman` | Fork of upstream; do not vendor the vault here |
| Git remotes | **origin** = `AnmolKamboj/openhuman`; **upstream** = `tinyhumansai/openhuman` | Personal rollup branch: **`mine`**. Cursor provider PR: **`feat/cursor-provider`** ([PR #5504](https://github.com/tinyhumansai/openhuman/pull/5504)) |
| Vault (brain) | `D:\Projects\Obsidian\smriti-vault` (clone of `D:\Projects\Obsidian\Anmol`) | Remote: private GitHub `AnmolKamboj/smriti-vault`. **Never mix vault into this repo** |
| Identity note | `People/Anmol.md` in the vault | Canonical facts. OpenHuman `PROFILE.md` is a short injection copy |
| OpenHuman user | `C:\Users\anmol\.openhuman\users\local-topg\` | `config.toml`, workspace, memory tree live here — not in git |
| Secrets | `D:\Projects\smriti\.env` + OpenHuman credential store | **Never print keys.** Never commit `.env` |
| Chat / agentic / memory LLM | Cursor Composer 2.5 (`cursor:composer-2.5~p=fast:false`) | Bridge at `http://127.0.0.1:8790/v1` |
| Reasoning / coding LLM | Cursor Grok 4.6 high, not fast | `cursor:grok-4.6~p=effort:high~p=fast:false` |
| Vision | `google:models/gemini-3.5-flash-lite` | |
| Learning | `nvidia:z-ai/glm-5.2` | |
| Embeddings | NVIDIA NIM, **1024 dims** (tree hard-requires 1024) | Not `nemotron-3-embed-1b` (native 2048 only — HTTP 400 at 1024). Use `nvidia/nv-embedqa-e5-v5` + `input_type` |
| Tool dispatcher | `auto`, but **Cursor is always XML/prompt-guided** | Cursor SDK `Agent.prompt` ignores OpenAI `tools`. Native function-calling never reaches Cursor |
| Telegram | Same Jarvis bot, `allowed_users = ["7579931038"]` | DMs: chat id = user id |
| Timezone | America/New_York | |
| Dev run | `scripts/run-dev-win.sh` (Git bash) | Auto-starts cursor-bridge. Port 1420 leftover Vite must be killed if bind fails |

---

## Two memory stores (must stay joined)

```text
1. User brain  = Obsidian vault
   D:\Projects\Obsidian\smriti-vault
   People/Anmol.md, Daily/, Places/, Traits/, Ideas/, ...

2. Bot LTM     = OpenHuman workspace
   C:\Users\anmol\.openhuman\users\local-topg\workspace\
   PROFILE.md, MEMORY.md, memory_tree/, transcripts
```

They join via:

- `[[memory_sources]]` folder `smriti` → vault `**/*.md` ingest + vector index
- `memory_recall` / `delegate_retrieve_memory` at chat time (on-demand, not pre-fetch)
- `PROFILE.md` + `MEMORY.md` injected into the orchestrator prompt (`omit_profile = false`, `omit_memory_md = false`)

Orchestrator has `omit_memory_context = true` — it does **not** auto-dump the tree. The model must **call tools**. That only works if the dispatcher is XML for Cursor.

---

## Cursor bridge (why native tools fail)

`scripts/cursor-bridge` is a Node sidecar:

- Binds **127.0.0.1:8790** only
- `GET /v1/models`, `POST /v1/chat/completions` → Cursor SDK `Agent.prompt`
- Auth: the same `crsr_…` key pasted in OpenHuman → Connections → LLM → Cursor
- Model params encoded as `model~p=param:value` (e.g. `composer-2.5~p=fast:false`)
- **Hard-codes `tools: []`.** OpenAI function-calling never reaches Cursor.

So for `cursor:*` providers this fork:

1. Forces `native_tool_calling = false` on the crate OpenAI client
2. Forces the session dispatcher to **XML** (tool schemas in the prompt text, which `messagesToPrompt` does send)
3. Forces `TurnModelSource` text mode so the harness does not attach a native `tools` array

XML calls look like `<tool_call>{"name":"memory_recall",...}</tool_call>`. That is how Cursor (or any LLM behind a tools-stripping bridge) reaches the brain.

OpenAI-compatible providers with **more than 128 tools** also fall back to XML (`OPENAI_COMPAT_MAX_NATIVE_TOOLS`). Telegram previously 400'd with 369 tools against OpenAI.

---

## Routing (Quick vs Reasoning vs Coding)

| UI | Hint | Config slot |
|---|---|---|
| Quick (default) | `hint:chat` | `chat_provider` |
| Reasoning pill | `hint:reasoning` | `reasoning_provider` |
| Coding | `hint:coding` | `coding_provider` |

`default_model = "chat-v1"` is a managed-tier name; with BYOK Cursor it still resolves through `chat_provider`.

Session gate: `verify_session_active` is a no-op in this fork so BYOK (Cursor / NVIDIA / Google) does not require an OpenHuman cloud JWT. **Must rebuild from this repo** — the installer exe will not pick that up.

---

## Embeddings / memory graph

The memory tree **hard-requires 1024-dim** vectors (`EMBEDDING_DIM`). Config `embedding_dimensions` must be 1024.

| Model | Native dim | 1024? | Notes |
|---|---|---|---|
| `nvidia/nemotron-3-embed-1b` | 2048 | **No** | HTTP 400 if `dimensions=1024`; not MRL. Do not use |
| `nvidia/nv-embedqa-e5-v5` | 1024 | **Yes** | Needs `input_type` `query`/`passage` (or `-query`/`-passage` model suffix) |
| `nvidia/llama-nemotron-embed-1b-v2` | 2048 | Yes (MRL) | Supports 384/512/768/1024/2048 |
| `baai/bge-m3` on NVIDIA | 1024 | flaky | Previously HTTP 500 on NIM |
| OpenAI `text-embedding-3-large` | 3072 | Yes via `dimensions` | No OpenAI key in this setup |
| Gemini `gemini-embedding-001` | 3072 | 768/1536/3072 MRL | Not 1024 |

This fork's custom NVIDIA embedder (NIM endpoint detected) sends `input_type` and will not request illegal dimensions for `nemotron-3-embed-1b`.

After changing embed model/dims: restart, then sync folder source `src_35d886dfdb134ed3b6c630de14a4ec59` (label `smriti`). Old incompatible vectors are unusable.

`embedding_strict = false` — failed embeds are skipped rather than aborting ingest. Check logs if recall is empty after a sync.

---

## System architecture (this machine)

```text
Telegram  ←→  OpenHuman (this repo, debug build)
Desktop UI ←→  same process
                    ↓
              cursor-bridge :8790  →  Cursor SDK (Composer / Grok)
              NVIDIA NIM           →  GLM chat (learning) + embeddings
              Google Gemini        →  vision
                    ↓
              memory_recall / folder ingest
                    ↓
         D:\Projects\Obsidian\smriti-vault     (brain, private git)
         ~/.openhuman/users/local-topg/        (bot LTM + config)
```

---

## Repo file map (fork-specific)

```text
D:\Projects\openhuman\
  docs/CONTEXT.md                 # this file
  scripts/cursor-bridge/          # Node OpenAI-compat sidecar for Cursor
  scripts/run-dev-win.sh          # Windows dev: app + bridge
  src/openhuman/inference/provider/factory.rs   # Cursor → prompt-guided tools
  src/openhuman/inference/embeddings/           # 1024-dim + NVIDIA NIM input_type
  src/openhuman/agent/harness/session/builder/  # dispatcher XML for cursor:

C:\Users\anmol\.openhuman\users\local-topg\
  config.toml                     # BYOK routes, Telegram, memory source
  workspace/PROFILE.md            # injected identity (cap ~2k chars)
  workspace/MEMORY.md             # how to recall the vault
```

---

## How to run locally (Windows)

```powershell
# From Git bash (the script expects bash):
& "C:\Program Files\Git\bin\bash.exe" "D:/Projects/openhuman/scripts/run-dev-win.sh"
```

- Cursor bridge also has a Startup shortcut `OpenHuman-cursor-bridge.cmd`.
- If Vite/UI fails on port 1420, kill the leftover process and retry.
- **Rebuild after Rust changes.** The running debug app will not pick up `factory.rs` edits until restart.

---

## What NOT to do (common mistakes for new chats)

- Do **not** put the vault inside this code repo
- Do **not** print or commit API keys (`crsr_`, `nvapi-`, Gemini, Groq, Telegram, GitHub PAT)
- Do **not** use `nvidia/nemotron-3-embed-1b` at 1024 dims
- Do **not** assume Cursor native `tools` work — they are stripped at the bridge
- Do **not** expect the installed `OpenHuman.exe` to contain this fork
- Do **not** push to **upstream** unless the user explicitly asks; origin is the fork
- Do **not** push from the local vault — GitHub is the source of truth (same rule as Smriti)
- Do **not** treat Telegram `allowed_users` as a group chat id — DMs use the user id
- Do **not** set `tool_dispatcher = "native"` for Cursor; even `auto` is forced to XML for `cursor:`

---

## Known gaps (as of 2026-08-14)

| Gap | Status |
|---|---|
| Cursor bridge ignores native tools | **Fixed in this fork** via forced XML / prompt-guided tools |
| PROFILE.md / MEMORY.md missing | Seeded into the OpenHuman workspace from the vault identity note |
| Embeddings 2048 vs tree 1024 | **Fixed:** NVIDIA NIM client + `nv-embedqa-e5-v5` @ 1024 |
| Folder source ingest empty | Re-sync `smriti` after rebuild; vectors were previously dropped |
| OpenAI 128-tool cap | Already on `mine` (XML fallback when tool count > 128) |
| BYOK session/JWT gate | Already on `mine` (`verify_session_active` allows BYOK) |
| Cursor provider upstream | Open PR #5504; daily work stays on `mine` |

---

## Prompt for new agent sessions

Copy-paste this into a new chat:

```text
I'm working on my OpenHuman fork (daily driver, replacing Jarvis/Smriti chat).
Read @docs/CONTEXT.md before making changes.
Key rules: vault stays at D:\Projects\Obsidian\smriti-vault, branch is mine,
Cursor must use prompt-guided/XML tools, embeddings must be 1024-dim,
never print secrets from smriti/.env.
```
