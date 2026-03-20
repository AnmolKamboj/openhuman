# Repository Guidelines

## Project Structure & Module Organization
- `src/`: React + TypeScript frontend (pages, components, hooks, store, services, lib).
- `src-tauri/`: Rust backend and Tauri desktop integration.
- `test/`: Vitest config and E2E/WebDriverIO setup (`test/e2e`, `test/wdio.conf.ts`).
- `skills/`: Skill runtime/build artifacts and manifests.
- `scripts/`: build/test automation (E2E helpers, tool generation).
- `docs/`: architecture and product documentation.
- `public/` and `src/assets/`: static assets.

## Build, Test, and Development Commands
- `yarn dev`: run Vite web dev server.
- `yarn dev:app` (or `yarn tauri:dev`): run desktop app with Tauri + TDLib bootstrap.
- `yarn build`: type-check and build web bundle.
- `yarn build:app`: build app bundle plus skills/tools generation.
- `yarn compile`: TypeScript check only (`tsc --noEmit`).
- `yarn test` / `yarn test:coverage`: run unit tests and coverage.
- `yarn test:rust`: run Rust tests in `src-tauri`.
- `yarn test:e2e`: run E2E suite (login/auth/payment/telegram/notion/gmail).
- `yarn lint` / `yarn lint:fix`, `yarn format:check` / `yarn format`: quality and formatting checks.

## Coding Style & Naming Conventions
- TypeScript-first; keep all app code under `src/`.
- Prettier is authoritative: 2 spaces, semicolons, single quotes, max width 100.
- Imports are auto-sorted via `@trivago/prettier-plugin-sort-imports`.
- ESLint (flat config) enforces React hooks rules, no duplicate imports, `prefer-const`, and TS-aware unused var checks.
- Naming: React components `PascalCase.tsx`; hooks `useX.ts`; slices/services/helpers `camelCase.ts`; tests `*.test.ts(x)`.

## Testing Guidelines
- Framework: Vitest + Testing Library (`jsdom`), with MSW utilities in `src/test/`.
- Unit tests live beside code (`__tests__`) or as `*.test.ts(x)` in `src/`.
- Coverage thresholds (global minimum): lines 15%, statements 15%, functions 15%, branches 12%.
- Run `yarn test:coverage` before opening non-trivial PRs.

## Commit & Pull Request Guidelines
- Prefer concise conventional-style subjects: `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`.
- Keep commits scoped and descriptive (e.g., `fix: handle socket reconnect on auth expiry`).
- If contributing upstream, branch from `develop` and open PRs against `develop`.
- PRs should include: purpose, key changes, test evidence (commands run), linked issue (`Fixes #123`), and screenshots/videos for UI changes.
- Hooks exist but are lightweight; run lint, format, compile, and relevant tests manually before push.
