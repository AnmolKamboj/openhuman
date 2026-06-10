// ---------------------------------------------------------------------------
// Settings search registry
//
// A single, flat, declarative manifest of every navigable settings destination.
// It is the source of truth for the global Settings search bar (Phase 1 —
// "shallow": pages / sections / dev tools, not the individual controls inside
// each panel).
//
// Each entry reuses the SAME i18n keys the existing menus render, so search
// results stay translated and in sync with the menu labels without inventing
// new copy. `keywords` are English-only match aids (synonyms a user might type)
// layered ON TOP of the already-localised title/description — they widen recall
// for English without affecting localised matching.
//
// `devOnly` entries are only surfaced when developer mode is on (they live
// under Settings → Developer & Diagnostics). Routes are resolved via
// `navigateToSettings(route)` — they map 1:1 to the <Route> table in
// `app/src/pages/Settings.tsx`.
//
// This registry is intentionally ADDITIVE: it mirrors the inline menu arrays
// rather than replacing them, so existing navigation cannot regress. A future
// follow-up can consolidate the menus to render from this registry.
// ---------------------------------------------------------------------------

export interface SettingsSearchEntry {
  /** Stable unique id — used as the React key and test id. */
  id: string;
  /** i18n key for the result title (reused from the existing menu item). */
  titleKey: string;
  /** i18n key for the result description (optional). */
  descriptionKey?: string;
  /** Settings route passed to `navigateToSettings(route)`. */
  route: string;
  /** i18n key for the section badge shown next to the result. */
  sectionKey: string;
  /** Extra English match terms (synonyms). Not shown in the UI. */
  keywords?: string[];
  /** When true, only surfaced if developer mode is enabled. */
  devOnly?: boolean;
}

// Section badge i18n keys (reused from the existing section headers).
const SECTION = {
  account: 'settings.groups.account',
  assistant: 'settings.groups.assistant',
  privacy: 'settings.privacySecurity.privacy',
  notifications: 'settings.groups.notifications',
  about: 'settings.about',
  features: 'pages.settings.featuresSection.title',
  ai: 'pages.settings.aiSection.title',
  agents: 'settings.agentsSection.title',
  composio: 'pages.settings.composioSection.title',
  crypto: 'settings.cryptoSection.title',
  settings: 'nav.settings',
  developer: 'settings.developerDiagnostics',
} as const;

/**
 * Every searchable settings destination. Deduped by route — where a route is
 * reachable from more than one menu, the most user-facing entry is kept and the
 * developer-menu duplicate is dropped.
 */
export const SETTINGS_SEARCH_ENTRIES: SettingsSearchEntry[] = [
  // --- Account ---
  {
    id: 'account',
    titleKey: 'pages.settings.accountSection.title',
    descriptionKey: 'pages.settings.accountSection.description',
    route: 'account',
    sectionKey: SECTION.account,
    keywords: ['profile', 'sign out', 'logout'],
  },
  {
    id: 'appearance',
    titleKey: 'settings.appearance.title',
    descriptionKey: 'settings.appearance.menuDesc',
    route: 'appearance',
    sectionKey: SECTION.account,
    keywords: ['theme', 'dark', 'light', 'mode', 'color', 'colour'],
  },
  {
    id: 'devices',
    titleKey: 'settings.account.devices',
    descriptionKey: 'settings.account.devicesDesc',
    route: 'devices',
    sectionKey: SECTION.account,
    keywords: ['mobile', 'phone', 'ios', 'android', 'pair'],
  },
  {
    id: 'data-sync',
    titleKey: 'settings.dataSync.title',
    descriptionKey: 'settings.dataSync.menuDesc',
    route: 'memory-sync',
    sectionKey: SECTION.account,
    keywords: ['sync', 'backup', 'data', 'memory'],
  },
  {
    id: 'team',
    titleKey: 'pages.settings.account.team',
    descriptionKey: 'pages.settings.account.teamDesc',
    route: 'team',
    sectionKey: SECTION.account,
    keywords: ['members', 'invites', 'organization', 'organisation', 'workspace'],
  },
  {
    id: 'security',
    titleKey: 'pages.settings.account.security',
    descriptionKey: 'pages.settings.account.securityDesc',
    route: 'security',
    sectionKey: SECTION.account,
    keywords: ['keychain', 'secret', 'password', 'encryption', 'credentials'],
  },
  {
    id: 'migration',
    titleKey: 'pages.settings.account.migration',
    descriptionKey: 'pages.settings.account.migrationDesc',
    route: 'migration',
    sectionKey: SECTION.account,
    keywords: ['import', 'export', 'transfer', 'data'],
  },

  // --- Assistant ---
  {
    id: 'persona',
    titleKey: 'settings.assistant.personality',
    descriptionKey: 'settings.assistant.personalityDesc',
    route: 'persona',
    sectionKey: SECTION.assistant,
    keywords: ['personality', 'tone', 'character', 'persona'],
  },
  {
    id: 'mascot',
    titleKey: 'settings.assistant.faceMascot',
    descriptionKey: 'settings.assistant.faceMascotDesc',
    route: 'mascot',
    sectionKey: SECTION.assistant,
    keywords: ['face', 'avatar', 'tiny', 'character'],
  },

  // --- Privacy ---
  {
    id: 'privacy',
    titleKey: 'settings.privacySecurity.privacy',
    descriptionKey: 'settings.privacySecurity.privacyDesc',
    route: 'privacy',
    sectionKey: SECTION.privacy,
    keywords: ['telemetry', 'tracking', 'analytics', 'data'],
  },

  // --- Notifications ---
  {
    id: 'notifications-hub',
    titleKey: 'settings.notifications.menuTitle',
    descriptionKey: 'settings.notifications.menuDesc',
    route: 'notifications-hub',
    sectionKey: SECTION.notifications,
    keywords: ['alerts', 'push', 'routing'],
  },
  {
    id: 'notification-settings',
    titleKey: 'pages.settings.features.notifications',
    descriptionKey: 'pages.settings.features.notificationsDesc',
    route: 'notifications',
    sectionKey: SECTION.notifications,
    keywords: ['alerts', 'push', 'preferences', 'routing'],
  },

  // --- Features ---
  {
    id: 'features',
    titleKey: 'pages.settings.featuresSection.title',
    descriptionKey: 'pages.settings.featuresSection.description',
    route: 'features',
    sectionKey: SECTION.settings,
  },
  {
    id: 'screen-intelligence',
    titleKey: 'pages.settings.features.screenAwareness',
    descriptionKey: 'pages.settings.features.screenAwarenessDesc',
    route: 'screen-intelligence',
    sectionKey: SECTION.features,
    keywords: ['screen', 'awareness', 'vision', 'capture'],
  },
  {
    id: 'tools',
    titleKey: 'pages.settings.features.tools',
    descriptionKey: 'pages.settings.features.toolsDesc',
    route: 'tools',
    sectionKey: SECTION.features,
    keywords: ['tools', 'capabilities', 'functions'],
  },
  {
    id: 'companion',
    titleKey: 'pages.settings.features.desktopCompanion',
    descriptionKey: 'pages.settings.features.desktopCompanionDesc',
    route: 'companion',
    sectionKey: SECTION.features,
    keywords: ['desktop', 'overlay', 'companion'],
  },

  // --- AI ---
  {
    id: 'ai',
    titleKey: 'pages.settings.aiSection.title',
    descriptionKey: 'pages.settings.aiSection.description',
    route: 'ai',
    sectionKey: SECTION.settings,
    keywords: ['ai', 'models', 'inference'],
  },
  {
    id: 'llm',
    titleKey: 'pages.settings.ai.llm',
    descriptionKey: 'pages.settings.ai.llmDesc',
    route: 'llm',
    sectionKey: SECTION.ai,
    keywords: ['model', 'anthropic', 'openai', 'claude', 'provider', 'api key'],
  },
  {
    id: 'embeddings',
    titleKey: 'pages.settings.ai.embeddings',
    descriptionKey: 'pages.settings.ai.embeddingsDesc',
    route: 'embeddings',
    sectionKey: SECTION.ai,
    keywords: ['vector', 'embedding', 'search'],
  },
  {
    id: 'voice',
    titleKey: 'pages.settings.ai.voice',
    descriptionKey: 'pages.settings.ai.voiceDesc',
    route: 'voice',
    sectionKey: SECTION.ai,
    keywords: ['tts', 'stt', 'speech', 'dictation', 'audio'],
  },
  {
    id: 'heartbeat',
    titleKey: 'settings.heartbeat.title',
    descriptionKey: 'settings.heartbeat.desc',
    route: 'heartbeat',
    sectionKey: SECTION.ai,
  },
  {
    id: 'ledger-usage',
    titleKey: 'settings.ledgerUsage.title',
    descriptionKey: 'settings.ledgerUsage.desc',
    route: 'ledger-usage',
    sectionKey: SECTION.ai,
    keywords: ['usage', 'tokens', 'ledger', 'cost'],
  },
  {
    id: 'cost-dashboard',
    titleKey: 'settings.costDashboard.title',
    descriptionKey: 'settings.costDashboard.desc',
    route: 'cost-dashboard',
    sectionKey: SECTION.ai,
    keywords: ['cost', 'spend', 'usage', 'billing'],
  },

  // --- Agents ---
  {
    id: 'agents-section',
    titleKey: 'settings.agentsSection.title',
    descriptionKey: 'settings.agentsSection.description',
    route: 'agents-settings',
    sectionKey: SECTION.settings,
  },
  {
    id: 'agents',
    titleKey: 'settings.agents.title',
    descriptionKey: 'settings.agents.subtitle',
    route: 'agents',
    sectionKey: SECTION.agents,
    keywords: ['agent', 'profiles'],
  },
  {
    id: 'autonomy',
    titleKey: 'settings.developerMenu.autonomy.title',
    descriptionKey: 'settings.developerMenu.autonomy.desc',
    route: 'autonomy',
    sectionKey: SECTION.agents,
    keywords: ['autonomy', 'autonomous'],
  },
  {
    id: 'agent-access',
    titleKey: 'settings.agentAccess.title',
    descriptionKey: 'settings.agentAccess.menuDesc',
    route: 'agent-access',
    sectionKey: SECTION.agents,
    keywords: ['access', 'permissions', 'tier', 'security policy'],
  },
  {
    id: 'activity-level',
    titleKey: 'activityLevel.title',
    descriptionKey: 'activityLevel.description',
    route: 'activity-level',
    sectionKey: SECTION.agents,
    keywords: ['background', 'activity', 'subconscious'],
  },
  {
    id: 'sandbox-settings',
    titleKey: 'settings.sandbox.title',
    descriptionKey: 'settings.sandbox.menuDesc',
    route: 'sandbox-settings',
    sectionKey: SECTION.agents,
    keywords: ['sandbox', 'jail', 'isolation', 'docker'],
  },

  // --- Composio / Integrations ---
  {
    id: 'composio-section',
    titleKey: 'pages.settings.composioSection.title',
    descriptionKey: 'pages.settings.composioSection.description',
    route: 'composio',
    sectionKey: SECTION.settings,
  },
  {
    id: 'task-sources',
    titleKey: 'settings.taskSources.title',
    descriptionKey: 'settings.taskSources.subtitle',
    route: 'task-sources',
    sectionKey: SECTION.composio,
    keywords: ['tasks', 'sources', 'inbox'],
  },
  {
    id: 'composio-routing',
    titleKey: 'settings.developerMenu.composioRouting.title',
    descriptionKey: 'settings.developerMenu.composioRouting.desc',
    route: 'composio-routing',
    sectionKey: SECTION.composio,
    keywords: ['composio', 'routing', 'integrations'],
  },
  {
    id: 'webhooks-triggers',
    titleKey: 'settings.developerMenu.composeioTriggers.title',
    descriptionKey: 'settings.developerMenu.composeioTriggers.desc',
    route: 'webhooks-triggers',
    sectionKey: SECTION.composio,
    keywords: ['webhooks', 'triggers', 'composio'],
  },

  // --- Crypto / Wallet ---
  {
    id: 'crypto-section',
    titleKey: 'settings.cryptoSection.title',
    descriptionKey: 'settings.cryptoSection.description',
    route: 'crypto',
    sectionKey: SECTION.settings,
    keywords: ['crypto', 'wallet'],
  },
  {
    id: 'recovery-phrase',
    titleKey: 'pages.settings.account.recoveryPhrase',
    descriptionKey: 'pages.settings.account.recoveryPhraseDesc',
    route: 'recovery-phrase',
    sectionKey: SECTION.crypto,
    keywords: ['mnemonic', 'seed', 'backup', 'recovery', 'wallet'],
  },
  {
    id: 'wallet-balances',
    titleKey: 'pages.settings.account.walletBalances',
    descriptionKey: 'pages.settings.account.walletBalancesDesc',
    route: 'wallet-balances',
    sectionKey: SECTION.crypto,
    keywords: ['wallet', 'balance', 'tokens', 'crypto'],
  },

  // --- About ---
  {
    id: 'about',
    titleKey: 'settings.about',
    descriptionKey: 'settings.aboutDesc',
    route: 'about',
    sectionKey: SECTION.about,
    keywords: ['version', 'build', 'update', 'developer mode'],
  },

  // --- Developer & Diagnostics (dev mode only) ---
  {
    id: 'developer-options',
    titleKey: 'settings.developerDiagnostics',
    descriptionKey: 'settings.developerDiagnosticsDesc',
    route: 'developer-options',
    sectionKey: SECTION.developer,
    keywords: ['developer', 'diagnostics', 'debug'],
    devOnly: true,
  },
  {
    id: 'intelligence',
    titleKey: 'settings.developerMenu.intelligence.title',
    descriptionKey: 'settings.developerMenu.intelligence.desc',
    route: 'intelligence',
    sectionKey: SECTION.developer,
    devOnly: true,
  },
  {
    id: 'memory-data',
    titleKey: 'devOptions.memoryInspection',
    descriptionKey: 'devOptions.memoryInspectionDesc',
    route: 'memory-data',
    sectionKey: SECTION.developer,
    keywords: ['memory', 'inspect'],
    devOnly: true,
  },
  {
    id: 'memory-debug',
    titleKey: 'devOptions.debugPanels',
    descriptionKey: 'devOptions.debugPanelsDesc',
    route: 'memory-debug',
    sectionKey: SECTION.developer,
    devOnly: true,
  },
  {
    id: 'analysis-views',
    titleKey: 'settings.analysisViews.title',
    descriptionKey: 'settings.analysisViews.menuDesc',
    route: 'analysis-views',
    sectionKey: SECTION.developer,
    devOnly: true,
  },
  {
    id: 'tool-policy-diagnostics',
    titleKey: 'devOptions.diagnostics',
    descriptionKey: 'devOptions.toolPolicyDiagnosticsDesc',
    route: 'tool-policy-diagnostics',
    sectionKey: SECTION.developer,
    devOnly: true,
  },
  {
    id: 'approval-history',
    titleKey: 'settings.approvalHistory.title',
    descriptionKey: 'settings.approvalHistory.subtitle',
    route: 'approval-history',
    sectionKey: SECTION.developer,
    keywords: ['approval', 'history'],
    devOnly: true,
  },
  {
    id: 'permissions',
    titleKey: 'settings.assistant.permissions',
    descriptionKey: 'settings.assistant.permissionsDesc',
    route: 'permissions',
    sectionKey: SECTION.developer,
    devOnly: true,
  },
  {
    id: 'model-health',
    titleKey: 'settings.modelHealth.title',
    descriptionKey: 'settings.modelHealth.desc',
    route: 'model-health',
    sectionKey: SECTION.developer,
    devOnly: true,
  },
  {
    id: 'search-engine',
    titleKey: 'settings.search.title',
    descriptionKey: 'settings.search.menuDesc',
    route: 'search',
    sectionKey: SECTION.developer,
    keywords: ['search', 'web', 'brave', 'parallel'],
    devOnly: true,
  },
  {
    id: 'agent-chat',
    titleKey: 'settings.developerMenu.agentChat.title',
    descriptionKey: 'settings.developerMenu.agentChat.desc',
    route: 'agent-chat',
    sectionKey: SECTION.developer,
    devOnly: true,
  },
  {
    id: 'local-model-debug',
    titleKey: 'settings.developerMenu.localModelDebug.title',
    descriptionKey: 'settings.developerMenu.localModelDebug.desc',
    route: 'local-model-debug',
    sectionKey: SECTION.developer,
    devOnly: true,
  },
  {
    id: 'cron-jobs',
    titleKey: 'settings.developerMenu.cronJobs.title',
    descriptionKey: 'settings.developerMenu.cronJobs.desc',
    route: 'cron-jobs',
    sectionKey: SECTION.developer,
    keywords: ['cron', 'schedule', 'jobs'],
    devOnly: true,
  },
  {
    id: 'webhooks-debug',
    titleKey: 'settings.developerMenu.webhooks.title',
    descriptionKey: 'settings.developerMenu.webhooks.desc',
    route: 'webhooks-debug',
    sectionKey: SECTION.developer,
    devOnly: true,
  },
  {
    id: 'mcp-server',
    titleKey: 'settings.developerMenu.mcpServer.title',
    descriptionKey: 'settings.developerMenu.mcpServer.desc',
    route: 'mcp-server',
    sectionKey: SECTION.developer,
    keywords: ['mcp', 'server'],
    devOnly: true,
  },
  {
    id: 'dev-workflow',
    titleKey: 'settings.developerMenu.devWorkflow.title',
    descriptionKey: 'settings.developerMenu.devWorkflow.desc',
    route: 'dev-workflow',
    sectionKey: SECTION.developer,
    devOnly: true,
  },
  {
    id: 'screen-awareness-debug',
    titleKey: 'settings.developerMenu.screenAwareness.title',
    descriptionKey: 'settings.developerMenu.screenAwareness.desc',
    route: 'screen-awareness-debug',
    sectionKey: SECTION.developer,
    devOnly: true,
  },
  {
    id: 'autocomplete',
    titleKey: 'settings.developerMenu.autocomplete.title',
    descriptionKey: 'settings.developerMenu.autocomplete.desc',
    route: 'autocomplete',
    sectionKey: SECTION.developer,
    devOnly: true,
  },
  {
    id: 'voice-debug',
    titleKey: 'settings.developerMenu.voiceDebug.title',
    descriptionKey: 'settings.developerMenu.voiceDebug.desc',
    route: 'voice-debug',
    sectionKey: SECTION.developer,
    devOnly: true,
  },
  {
    id: 'event-log',
    titleKey: 'settings.developerMenu.eventLog.title',
    descriptionKey: 'settings.developerMenu.eventLog.desc',
    route: 'event-log',
    sectionKey: SECTION.developer,
    keywords: ['events', 'log'],
    devOnly: true,
  },
];
