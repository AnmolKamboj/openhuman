---
icon: screwdriver
---

# Skills & Integrations

### Skills & Integrations

OpenHuman's capabilities are organized into three categories, all accessible from the Skills tab in the bottom navigation.

#### Built-in Skills

Core desktop capabilities that run locally on your device.

**Screen Intelligence:** Captures your screen, summarizes what is on it, and feeds useful context into memory. Configurable capture frequency (default: 1 frame per second), per-app allowlist and denylist controls, and privacy-first local processing. Configure in Settings > Automation & Channels > Screen Intelligence.

**Text Auto-Complete (Inline Autocomplete):** Suggests inline completions while you type in any application. Powered by your local AI model and your memory context. Accept suggestions with Tab. Configurable debounce timing, max characters, style presets, and per-app disable list. Configure in Settings > Automation & Channels > Inline Autocomplete.

**Voice Speech To Text:** Uses the microphone for dictation and voice-driven chat with local speech recognition. All processing happens on-device.

#### Channel Integrations

Messaging platforms that OpenHuman can send and receive messages through.

**Telegram:** Full messaging integration with 80+ capabilities including sending, replying, forwarding, editing, and deleting messages, managing groups and channels, handling contacts, admin actions, and more. See the Telegram Capabilities Reference for the complete list. Two connection modes: connect via OpenHuman (one-click, encrypted) or provide your own credentials.

**Discord:** Send and receive messages via Discord. Configuration required.

Both channels show connection status and can be configured by clicking the Configure button in the Skills tab.

#### 3rd Party Skills

External data sources that OpenHuman ingests and reasons over.

**Gmail:** Connects via Google API for comprehensive email integration. Once connected, OpenHuman syncs your email on a configurable interval (default: every 15 minutes). Shows local storage used and file count. To connect: go to Skills tab > 3rd Party Skills > Gmail > Configure.

**Notion:** Workspace integration with 25 tools for pages, databases, and documents. Syncs on a configurable interval (default: every 20 minutes). Shows local storage used and file count.

To connect: go to Skills tab > 3rd Party Skills > Notion > Configure.

**Important:** Gmail and Notion are available now as 3rd Party Skills. The Connections page in **Settings > Account & Security** shows deeper native integrations (Google, Notion, Web3 Wallet, Crypto Trading Exchanges) that are coming soon. To connect Gmail or Notion today, use the Skills tab, not the Connections page.

#### Sync Intervals

Each connected 3rd party skill syncs on a configurable schedule. You can adjust intervals in **Settings > Automation & Channels > Cron Jobs**.

| Skill  | Default Sync Interval |
| ------ | --------------------- |
| Gmail  | Every 15 minutes      |
| Notion | Every 20 minutes      |

More frequent syncing keeps data fresher but uses more inference budget.

#### Coming Soon

Connections for Google (calendar, contacts), Notion (deeper native integration), Web3 Wallet, and Crypto Trading Exchanges are in development. These will appear in Settings > Account & Security > Connections when available.
