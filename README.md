<h1 align="center">AlphaHuman</h1>

<p align="center">
  <strong>Your most productive co-worker</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/status-early%20beta-orange" alt="Early Beta" />
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux%20%7C%20Android%20%7C%20iOS-blue" alt="Platforms" />
  <a href="https://github.com/alphahumanxyz/alphahuman/releases/latest"><img src="https://img.shields.io/github/v/release/alphahumanxyz/alphahuman?label=latest" alt="Latest Release" /></a>
</p>

![The Tet](./docs/the-tet.png)

<p align="center" style="font-style: italic">
  "The Tet. What a brilliant machine" Morgan Freeman as he <a href="https://youtu.be/SveLVpqy_Rc?si=y83aZNokPiUjILN0&t=60">reminisces about the Tet</a> in the movie Oblivion
</p>

## What is AlphaHuman?

AlphaHuman is a personal AI assistant that helps you manage high-volume communication without reading everything yourself. It connects to your messaging platforms and productivity tools, understands conversations in context, and produces clear, actionable outputs you can use immediately.

AlphaHuman is not a chatbot, browser extension, or cloud-only service. It is a native application that runs on your device, connects to your tools, and works only when you ask it to. Think of it as a second brain that sits across your communication and productivity stack.

## What It Does

- **Summarize conversations** -- Understand what happened without reading everything
- **Surface signals** -- Decisions, action items, risks, and sentiment shifts extracted automatically
- **Generate responses** -- Context-aware reply suggestions informed by conversation history
- **Create workflows** -- Turn unstructured commitments into trackable actions
- **Export intelligence** -- Push summaries, action items, and structured data to Notion, Google Sheets, and connected tools
- **Run automations** -- Via a sandboxed skills engine that extends the platform without app updates

## Download

> **Early Beta** -- AlphaHuman is under active development. Expect rough edges.

### macOS

| Chip                        | Download                                                                                                        |
| --------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Apple Silicon (M1/M2/M3/M4) | [`.dmg` (aarch64)](https://github.com/alphahumanxyz/alphahuman/releases/latest/download/AlphaHuman_aarch64.dmg) |
| Intel                       | [`.dmg` (x64)](https://github.com/alphahumanxyz/alphahuman/releases/latest/download/AlphaHuman_x64.dmg)         |

### Windows

| Architecture | Download                                                                                                |
| ------------ | ------------------------------------------------------------------------------------------------------- |
| x64          | [`.msi`](https://github.com/alphahumanxyz/alphahuman/releases/latest/download/AlphaHuman_x64_en-US.msi) |

### Linux

| Format          | Download                                                                                                      |
| --------------- | ------------------------------------------------------------------------------------------------------------- |
| Debian / Ubuntu | [`.deb` (amd64)](https://github.com/alphahumanxyz/alphahuman/releases/latest/download/AlphaHuman_amd64.deb)   |
| Fedora / RHEL   | [`.rpm` (x86_64)](https://github.com/alphahumanxyz/alphahuman/releases/latest/download/AlphaHuman_x86_64.rpm) |
| Universal       | [`.AppImage`](https://github.com/alphahumanxyz/alphahuman/releases/latest/download/AlphaHuman_amd64.AppImage) |

### Mobile

- **Android** and **iOS** -- Coming soon

Browse all releases: [github.com/alphahumanxyz/alphahuman/releases](https://github.com/alphahumanxyz/alphahuman/releases)

## Skills

AlphaHuman uses a pluggable **skills** architecture. Each skill connects to an external service, syncs relevant data locally, and exposes tools that you (or the AI) can use. Skills run in a sandboxed environment with their own database, storage, and permissions.

| Skill           | Status      | Description                                                   |
| --------------- | ----------- | ------------------------------------------------------------- |
| Telegram        | Available   | Chats, messages, contacts, search, admin tools, AI summaries  |
| Notion          | Available   | Pages, databases, blocks, users, comments, search, local sync |
| Gmail           | In Progress | Email management, labels, search, send/receive with OAuth2    |
| Google Calendar | In Progress | Calendars, events, scheduling with OAuth2                     |
| Google Drive    | In Progress | Files, Sheets, Docs with OAuth2                               |
| Slack           | In Progress | Messages, channels, real-time events                          |
| Web3 Wallets    | Planned     | EVM wallet management, balance checks, network monitoring     |

## Privacy-First

- **Zero retention** -- Message content is processed to produce output, then discarded
- **OS-level credential storage** -- Desktop platforms use native keychains (macOS Keychain, Windows Credential Manager)
- **No training on your data** -- Your data is never used for model improvement
- **Request-only processing** -- Nothing happens without an explicit user action; no background scanning
- **Sandboxed skills** -- Each skill runs in isolation with memory and resource limits

## Getting Started

1. **Download** the installer for your platform from the [releases page](https://github.com/alphahumanxyz/alphahuman/releases/latest)
2. **Install** the app (drag to Applications on macOS, or use your package manager on Linux)
3. **Connect a source** -- follow the in-app onboarding to link Telegram, Notion, Gmail, or other services
4. **Run your first request** -- ask the AI to summarize what you missed, extract action items, or surface key decisions

## Links

- [Architecture Overview](./ARCHITECTURE.md) -- How AlphaHuman is built
- [Changelog](./CHANGELOG.md) -- Release history
- [Website](https://alphahuman.xyz) -- Learn more

---

<p align="center">
  Made with love by a bunch of Web3 nerds
</p>

<p align="center">
  <sub>AlphaHuman is in early beta. Features may change, break, or disappear. Use at your own risk.</sub>
</p>
