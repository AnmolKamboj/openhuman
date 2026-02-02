# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Summary

Cross-platform crypto community communication platform built with **Tauri v2** (React 19 + Rust). Targets desktop (Windows, macOS) and mobile (Android, iOS). Features deep Telegram integration via MTProto, real-time Socket.io communication, and an MCP (Model Context Protocol) tool system for AI-driven Telegram interactions.

## App Theme & Design System

**Design Philosophy**: Premium, sophisticated crypto platform with calm, trustworthy aesthetic.

### Color Palette

- **Primary**: Ocean blue (`#4A83DD`) optimized for dark backgrounds
- **Sage**: Success green (`#4DC46F`) for growth indicators
- **Amber**: Warning (`#E8A838`) for attention states
- **Coral**: Error (`#F56565`) soft professional red
- **Canvas**: Background layers (`#FAFAF9` to `#D4D4D1`) with subtle warmth
- **Market Colors**: Bullish green, bearish red, Bitcoin orange, Ethereum purple

### Typography

- **Primary**: Inter (premium font stack)
- **Display**: Cabinet Grotesk for headings
- **Mono**: JetBrains Mono for code
- **Scale**: Sophisticated sizing with negative letter spacing for elegance

### Component System

- **Shadows**: Glow effects, subtle to float depth levels
- **Animations**: Fade-in, slide-in, scale-in with cubic-bezier easing
- **Border Radius**: Smooth system from `xs` (0.25rem) to `5xl` (2rem)
- **Spacing**: Extended scale including custom values (4.5, 13, 15, etc.)

### Current UI State

- Uses HashRouter (not BrowserRouter) as seen in `App.tsx:1`
- 153 TypeScript files total in src/
- Sophisticated Tailwind config with custom color system and animations

## Commands

```bash
# Frontend dev server only (port 1420)
yarn dev

# Desktop dev with hot-reload (starts Vite + Tauri)
yarn tauri dev

# Production build (TypeScript compile + Vite build + Tauri bundle)
yarn tauri build

# Debug build with .app bundle (required for deep link testing on macOS)
# On macOS, alphahuman:// only works when running the .app, not `tauri dev`
yarn tauri build --debug --bundles app
yarn macos:dev

# Android
yarn tauri android dev
yarn tauri android build

# iOS
yarn tauri ios dev
yarn tauri ios build

# Rust checks
cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml
```

No test framework is currently configured. **ESLint and Prettier are configured** with Husky pre-commit/pre-push hooks for code quality enforcement.

## Architecture

### Provider Chain (App.tsx)

The app wraps in this order: `Redux Provider` → `PersistGate` → `SocketProvider` → `TelegramProvider` → `HashRouter` → `AppRoutes`. **Note**: Now uses HashRouter instead of BrowserRouter. This ordering matters because Socket.io and Telegram providers depend on Redux auth state.

### State Management (Redux Toolkit + Persist)

State lives in `src/store/` using Redux Toolkit slices:

- **authSlice** — JWT token, onboarding completion flag (persisted)
- **userSlice** — user profile
- **socketSlice** — connection status, socket ID
- **telegramSlice** — connection/auth status, chats, messages, threads (selectively persisted; loading/error states excluded)
- **aiSlice** — AI system state, memory management, session tracking
- **skillsSlice** — skills catalog, setup status, management state
- **teamSlice** — team management, member invites, permissions

Redux Persist stores auth and telegram state (storage backend is configurable; default uses localStorage). The telegram slice has a complex nested structure in `src/store/telegram/` with separate files for types, reducers, extraReducers, and thunks.

### LocalStorage

- **Do not use `localStorage` (or `sessionStorage`) for app state or feature logic.** Use Redux (and Redux Persist where needed) instead.
- **Remove any existing `localStorage` usage** when touching related code. User-scoped data (auth, onboarding, Telegram session, socket state) lives in Redux, keyed by user id where applicable. Telegram session is in `telegram.byUser[userId].sessionString`, not localStorage.
- **Exceptions**: Redux-persist may use a localStorage-backed storage adapter by default; that is the persistence layer, not app logic. Any other remaining usage (e.g. deep-link `deepLinkHandled` flag) should be migrated to Redux or similar when that code is modified.
- **General rule**: Avoid adding new `localStorage` or `sessionStorage` usage; prefer Redux and remove existing usage when you work on affected areas.

### Service Layer (Singletons)

- **mtprotoService** (`src/services/mtprotoService.ts`) — Telegram MTProto client via `telegram` npm package. Session stored in Redux (`telegram.byUser[userId].sessionString`), not localStorage. Auto-retries FLOOD_WAIT up to 60s.
- **socketService** (`src/services/socketService.ts`) — Socket.io client. Auth token passed in socket `auth` object (not query string). Transports: polling first, then WebSocket.
- **apiClient** (`src/services/apiClient.ts`) — HTTP client for REST backend.

### MCP System (`src/lib/mcp/`)

Model Context Protocol implementation for AI tool execution over Socket.io:

- `transport.ts` — Socket.io JSON-RPC 2.0 transport with 30s timeout
- `telegram/server.ts` — TelegramMCPServer manages 99 tool definitions
- `telegram/tools/` — Individual tool files (one per Telegram API operation)
- Tools use `big-integer` library for Telegram's large integer IDs

### Routing (`src/AppRoutes.tsx`)

```
/           → Welcome (public)
/login      → Login (public)
/onboarding → Onboarding (protected, requires auth, not yet onboarded)
/home       → Home (protected, requires auth + onboarded)
*           → DefaultRedirect (routes based on auth state)
```

`PublicRoute` redirects authenticated users away. `ProtectedRoute` enforces auth and optionally onboarding status.

### Deep Link Auth Flow

Web-to-desktop handoff using `alphahuman://` URL scheme:

1. User authenticates in browser
2. Browser redirects to `alphahuman://auth?token=<loginToken>`
3. Tauri catches the deep link, Rust `exchange_token` command calls backend via `reqwest` (bypasses CORS)
4. Backend returns `sessionToken` + user object
5. App stores session in Redux, navigates to onboarding/home

Key file: `src/utils/desktopDeepLinkListener.ts` (lazy-loaded in `main.tsx`). Uses a `deepLinkHandled` flag to prevent infinite reload loops. Deep links do NOT work in `tauri dev` on macOS — must use built `.app` bundle.

### Rust Backend (`src-tauri/src/lib.rs`)

Minimal — two Tauri commands:

- `greet` — demo command
- `exchange_token` — CORS-free HTTP POST to backend for token exchange

Deep link plugin registered at setup. `register_all()` called only on Windows/Linux (panics on macOS).

## Environment Variables

Set in `.env` (Vite exposes `VITE_*` prefixed vars):

| Variable                     | Purpose                                            |
| ---------------------------- | -------------------------------------------------- |
| `VITE_BACKEND_URL`           | Backend API URL (default: `http://localhost:5005`) |
| `VITE_TELEGRAM_API_ID`       | Telegram MTProto API ID                            |
| `VITE_TELEGRAM_API_HASH`     | Telegram MTProto API hash                          |
| `VITE_TELEGRAM_BOT_USERNAME` | Telegram bot username                              |
| `VITE_TELEGRAM_BOT_ID`       | Telegram bot numeric ID                            |
| `VITE_SENTRY_DSN`            | Sentry DSN for error reporting (optional)          |
| `VITE_DEBUG`                 | Debug mode flag                                    |

Production defaults are in `src/utils/config.ts`.

## Recent Changes (Last 24 Hours)

Key updates from recent commits:

### Major Additions

- **ESLint & Prettier Integration** (`5896966`): Complete code quality toolchain
  - ES module syntax for ESLint configuration with enhanced TypeScript support
  - Husky pre-commit/pre-push hooks for automatic formatting and linting
  - Type-only imports standardization across codebase
  - Consolidated import statements and improved code organization
  - GitHub workflows updated with Prettier and ESLint checks
- **Advanced Skills System** (`10ec1b3`): Comprehensive skill management platform
  - Dynamic skills loading from local directory via Rust integration
  - SkillSetupModal with conditional rendering (wizard vs management panel)
  - Background GitHub sync for skills catalog updates
  - Skills table with setup status indicators and management controls
  - Enhanced skill metadata with setup hooks and descriptions
- **Team Management Features** (`10ec1b3`): Multi-user collaboration system
  - TeamPanel, TeamMembersPanel, and TeamInvitesPanel components
  - Redux state management for teams, members, and invites
  - Team API integration with CRUD operations
  - Settings modal routing for team management paths
  - Role-based permissions and invitation system
- **AI System Enhancements**: Advanced memory and session management
  - Hybrid search with encryption for AI memory
  - Constitution-based AI behavior with GitHub integration
  - Entity graph migration to Neo4j backend
  - Session capture and transcript management
  - Memory chunking and context formatting
- **Enhanced CI/CD Pipeline** (`b1d7bce`): Production-ready deployment
  - XGH_TOKEN authentication for alphahumanxyz/alphahuman releases
  - Python sidecar setup and caching for cross-platform builds
  - Tauri configuration updates (com.alphahuman.app identifier)
  - GitHub Pages deployment with optimized workflows
  - Version tagging and environment variable management
- **Device Detection & Download System** (`9d74721`, `b5bccd2`): Enhanced multi-architecture download support
  - Optimized asset parsing using Maps for unique architecture links per platform
  - Enhanced DownloadScreen.tsx with architecture-specific download options
  - Improved device detection for Windows, macOS, Linux, and Android platforms
  - Added preference logic for more specific filenames in asset parsing
  - Support for multiple architectures (x64, aarch64) with intelligent sorting
- **Version Bump**: Project updated to v0.20.0 (`891517c`)

### Design System Updates

- **Settings Modal UI**: Clean 520px white modal contrasting with glass morphism theme
- **Animations**: 200ms entry animations, 250ms panel transitions, chevron hover effects
- **Lottie Animations**: Integrated into onboarding flow (`334673e`)
- **Connection Components**: Added Telegram and Gmail connection indicators
- **Routing**: Switched to HashRouter for better desktop app compatibility
- **Theme**: Implemented sophisticated color system with premium crypto aesthetic

### Component Structure

- **200+ TypeScript files** across `src/` directory with comprehensive tooling
- **AI System Architecture** (`src/lib/ai/`): Advanced artificial intelligence platform
  - Memory management with encryption, chunking, and hybrid search
  - Constitution-based behavior with GitHub integration
  - Entity graph with Neo4j backend integration
  - Session capture, transcript management, and tool compression
  - Provider system with OpenAI integration and custom providers
- **Skills Management System**: Dynamic skill platform with Rust integration
  - SkillsGrid.tsx - Skills catalog with setup status and management
  - SkillSetupModal.tsx - Conditional wizard/management panel rendering
  - SkillProvider.tsx - GitHub sync and local directory integration
  - Skills submodule integration with background updates
- **Team Collaboration Features**: Multi-user workspace management
  - TeamPanel.tsx - Team overview with member management
  - TeamMembersPanel.tsx - Member roles and permissions
  - TeamInvitesPanel.tsx - Invitation system with role assignment
  - Team API integration with Redux state management
- **Settings Modal System**: Comprehensive configuration interface
  - SettingsModal.tsx - Main container with URL routing
  - SettingsLayout.tsx - Modal wrapper with createPortal
  - Enhanced panels: Billing, Team, Connections, Privacy, Profile
  - Hooks: useSettingsNavigation.ts, useSettingsAnimation.ts
- **Download System**: Enhanced multi-platform distribution
  - DownloadScreen.tsx - Platform detection with architecture support
  - deviceDetection.ts - Comprehensive device/architecture utilities
  - GitHub API integration for real-time release assets
- **Code Quality Infrastructure**: ESLint, Prettier, and Husky integration
  - Pre-commit/pre-push hooks with TypeScript compilation checks
  - Standardized type-only imports and consolidated statements
  - GitHub workflow integration with automated quality checks

## Git Workflow

- **PR target branch**: All pull requests should target the `develop` branch, not `main`.

## Key Patterns

- **Code Quality**: ESLint and Prettier enforce code standards with Husky hooks. Use type-only imports (`import type`) and consolidate imports from same modules.
- **No localStorage**: Avoid `localStorage` and `sessionStorage`; use Redux (and persist) for app state. Remove any direct usage when working on affected code.
- **AI System Integration**: Use `src/lib/ai/` for memory management, constitution loading, entity queries, and session capture. AI providers abstracted through interface pattern.
- **Skills Management**: Skills loaded dynamically from local directory via Rust. Use `SkillProvider` for GitHub sync and `SkillsGrid` for management interface.
- **Team Collaboration**: Team features in `src/components/settings/panels/Team*`. Use Redux `teamSlice` for state management and `teamApi` for backend operations.
- **Device Detection**: Use `deviceDetection.ts` utilities for platform/architecture detection. Support multiple architectures per platform (x64, aarch64) with intelligent preference logic.
- **GitHub Integration**: Fetch release assets via GitHub API (`fetchLatestRelease()`) and parse by architecture (`parseReleaseAssetsByArchitecture()`). Use Maps for efficient unique architecture tracking.
- **Download System**: Platform-specific file type support (.exe/.msi for Windows, .dmg for macOS, .AppImage/.deb/.rpm for Linux, .apk for Android) with fallback links.
- **Modal System**: Settings modal uses `createPortal` pattern with URL-based routing. Clean white design (not glass morphism) for system settings. Navigate with `/settings` paths for different panels.
- **Component Reuse**: Connection management reuses `connectOptions` array and components from onboarding flow. Maintains consistent UX patterns across features.
- **Redux Integration**: Multiple slices (auth, user, telegram, ai, skills, team) with Redux Persist. Use typed hooks and selectors. State functions accept optional `userId` param.
- **Node polyfills**: Vite config (`vite.config.ts`) polyfills `buffer`, `process`, `util`, `os`, `crypto`, `stream` for the `telegram` package which requires Node APIs.
- **Telegram IDs**: Use `big-integer` library, not native JS numbers (Telegram IDs exceed `Number.MAX_SAFE_INTEGER`).
- **MCP tool files**: Each tool in `src/lib/mcp/telegram/tools/` exports a handler conforming to `TelegramMCPToolHandler` interface. Tool names are typed in `src/lib/mcp/telegram/types.ts`.
- **Tauri IPC**: Frontend calls Rust via `invoke()` from `@tauri-apps/api/core`. Rust commands are registered in `generate_handler![]` macro.
- **CORS workaround**: External HTTP requests from the WebView hit CORS. Use Rust `reqwest` via Tauri commands instead of browser `fetch()`.
- **Hash Routing**: Uses HashRouter for desktop app compatibility and deep link handling.
- **Integration Libraries**: Each integration (Telegram, future Gmail, etc.) lives under `src/lib/<integration>/` with its own `state/`, `services/`, `api/` subdirectories. Domain-specific services belong in the integration folder, not in `src/services/` (which holds only cross-cutting services like socketService, apiClient).
- **Unit Tests**: All unit tests live in `__tests__/` folders co-located with the code they test. Use Jest with TypeScript support.

## Platform Gotchas

- **macOS deep links**: Require `.app` bundle (not `tauri dev`). Clear WebKit caches when debugging stale content: `rm -rf ~/Library/WebKit/com.alphahuman.app ~/Library/Caches/com.alphahuman.app`
- **Cargo caching**: May serve stale frontend assets on incremental builds. Run `cargo clean --manifest-path src-tauri/Cargo.toml` if the app shows outdated UI.
- **`window.__TAURI__`**: Not available at module load time. Use dynamic `import()` and try/catch for Tauri plugin calls.
