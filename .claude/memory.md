# Project Memory

Quick reference for anyone starting with Claude on this project. Updated by the `memory-keeper` agent.

## Fixes & Gotchas

- **ServiceBlockingGate CORS errors** — The gate calls `openhumanServiceStatus()` and `openhumanAgentServerStatus()` at startup. These used `callCoreRpc()` which falls back to raw `fetch()` when socket isn't connected yet, causing CORS errors. Fix: route through `invoke('core_rpc_relay')` instead (Tauri IPC, no CORS).
- **Socket not connected at startup** — `SocketProvider` only connects when a Redux `auth.token` is set. At fresh launch (no token), socket is null, so any `callCoreRpc()` call falls back to `fetch()`. Always use `invoke('core_rpc_relay')` for local sidecar RPC calls.
- **`openhuman.agent_server_status` doesn't exist** — This RPC method is not registered in the core. The gate checks it but it always errors. The gate passes if either service is Running OR agent server is running OR core is reachable.
- **Cargo incremental builds can serve stale UI** — If the app shows old frontend after a Rust rebuild, run `cargo clean --manifest-path app/src-tauri/Cargo.toml` before rebuilding.
- **macOS deep links require .app bundle** — `yarn tauri dev` does NOT support deep links. Must use `yarn tauri build --debug --bundles app`.

## Strict Rules

- **No dynamic imports in `app/src/`** — Use static `import` at file top. Guard call sites with `try/catch` for Tauri/non-Tauri safety. See CLAUDE.md.
- **Service RPC calls must use Tauri IPC** — Never use `callCoreRpc()` for service operations. Use `invoke('core_rpc_relay', { request: { method, params } })`.
- **Always run checks before commit** — `yarn typecheck`, `yarn lint`, `yarn format:check`, `yarn build`. Husky hooks enforce this but run manually first.
- **Stage specific files** — Never `git add -A`. Always `git add <specific-files>`.

## Workflow

- **Agent order**: architectobot (plan) → user approval → codecrusher (implement) → architectobot (verify)
- **Always read CLAUDE.md first** before any issue work
- **Ask user when in doubt** — never assume scope or approach
- **PRs target upstream** — `tinyhumansai/openhuman` main branch, not fork

## Environment

- **Core sidecar port** — `7788` (default). Check with `lsof -i :7788`.
- **Stage sidecar** — `cd app && yarn core:stage` (required for core RPC).
- **Kill stuck processes** — `lsof -i :7788` then `kill <PID>`.
