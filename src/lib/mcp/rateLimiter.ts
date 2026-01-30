/**
 * MCP Rate Limiter
 *
 * Three-tier rate limiting system for MCP tool execution:
 * 1. Per-request counter — caps tool calls within a single agent request
 * 2. Per-minute sliding window — prevents sustained high-frequency usage
 * 3. Inter-call delay — enforces minimum gap between API calls
 *
 * Read-only tools that only access cached Redux state bypass rate limits.
 * Mutation tools (send, delete, create, etc.) incur heavier delays.
 */

import { mcpLog, mcpWarn } from "./logger";

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

export const RATE_LIMIT_CONFIG = {
  /** Minimum delay (ms) between ANY tool calls that hit the Telegram API */
  MIN_CALL_DELAY_MS: 500,
  /** Extra delay (ms) for mutation/write operations */
  HEAVY_CALL_DELAY_MS: 1000,
  /** Maximum tool calls allowed within a 60-second sliding window */
  MAX_CALLS_PER_MINUTE: 30,
  /** Maximum tool calls allowed within a single MCP request */
  MAX_CALLS_PER_REQUEST: 20,
} as const;

// ---------------------------------------------------------------------------
// Tool classification
// ---------------------------------------------------------------------------

/**
 * Read-only tools that only query cached Redux state — zero Telegram API calls.
 * These are exempt from rate limiting entirely.
 */
const READ_ONLY_TOOLS = new Set<string>([
  "get_chats",
  "list_chats",
  "get_chat",
  "get_messages",
  "list_messages",
  "get_me",
  "get_drafts",
  "get_message_context",
  "get_pinned_messages",
  "get_history",
  "get_user_status",
  "get_participants",
  "get_admins",
  "get_blocked_users",
  "list_contacts",
  "search_contacts",
  "get_contact_ids",
  "get_contact_chats",
  "get_direct_chat_by_contact",
  "list_inline_buttons",
  "list_topics",
  "get_message_reactions",
]);

/**
 * Heavy (mutation) tools that modify state on Telegram servers.
 * These incur HEAVY_CALL_DELAY_MS instead of MIN_CALL_DELAY_MS.
 */
const HEAVY_TOOLS = new Set<string>([
  "send_message",
  "reply_to_message",
  "forward_message",
  "delete_message",
  "create_group",
  "create_channel",
  "invite_to_group",
  "ban_user",
  "unban_user",
  "promote_admin",
  "demote_admin",
  "archive_chat",
  "unarchive_chat",
  "leave_chat",
  "import_contacts",
  "export_contacts",
  "set_privacy_settings",
  "set_bot_commands",
  "set_profile_photo",
  "delete_profile_photo",
  "edit_chat_photo",
  "delete_chat_photo",
  "create_poll",
]);

// ---------------------------------------------------------------------------
// Rate limiter state
// ---------------------------------------------------------------------------

/** Timestamp of the last API-bound tool call */
let lastCallTime = 0;

/** Per-request call counter — reset via resetRequestCallCount() */
let callsInCurrentRequest = 0;

/** Sliding window of timestamps for per-minute tracking */
const callHistory: number[] = [];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Returns true if the given tool name reads only from local cache and should
 * bypass all rate limiting.
 */
export function isReadOnlyTool(toolName: string): boolean {
  return READ_ONLY_TOOLS.has(toolName);
}

/**
 * Returns true if the given tool performs a mutation/write operation and should
 * incur a heavier inter-call delay.
 */
export function isHeavyTool(toolName: string): boolean {
  return HEAVY_TOOLS.has(toolName);
}

/**
 * Reset the per-request call counter. Call this at the start of each new
 * MCP request (agent turn) to allow a fresh budget of tool calls.
 */
export function resetRequestCallCount(): void {
  callsInCurrentRequest = 0;
}

/**
 * Enforce rate limits before executing a tool.
 *
 * - Read-only tools skip all limits.
 * - For API-bound tools:
 *   1. Check per-request budget → throw if exceeded
 *   2. Check per-minute sliding window → sleep until budget available
 *   3. Enforce inter-call delay (heavier for mutation tools)
 *
 * Call this BEFORE executing the tool handler. It may asynchronously wait
 * if delays are needed, or throw if hard limits are exceeded.
 */
export async function enforceRateLimit(toolName: string): Promise<void> {
  // Read-only tools are always allowed instantly
  if (isReadOnlyTool(toolName)) {
    return;
  }

  // --- Per-request cap ---
  callsInCurrentRequest += 1;
  if (callsInCurrentRequest > RATE_LIMIT_CONFIG.MAX_CALLS_PER_REQUEST) {
    throw new Error(
      `Rate limit: exceeded ${RATE_LIMIT_CONFIG.MAX_CALLS_PER_REQUEST} tool calls per request. ` +
        `Try breaking your task into smaller steps.`,
    );
  }

  // --- Per-minute sliding window ---
  const now = Date.now();
  purgeOldEntries(now);

  if (callHistory.length >= RATE_LIMIT_CONFIG.MAX_CALLS_PER_MINUTE) {
    // Wait until the oldest entry expires from the window
    const oldestTimestamp = callHistory[0];
    const waitMs = oldestTimestamp + 60_000 - now + 50; // +50ms buffer
    mcpWarn(
      `Rate limit: per-minute cap reached (${RATE_LIMIT_CONFIG.MAX_CALLS_PER_MINUTE}/min). ` +
        `Waiting ${waitMs}ms for '${toolName}'.`,
    );
    await sleep(waitMs);
    purgeOldEntries(Date.now());
  }

  // --- Inter-call delay ---
  const requiredDelay = isHeavyTool(toolName)
    ? RATE_LIMIT_CONFIG.HEAVY_CALL_DELAY_MS
    : RATE_LIMIT_CONFIG.MIN_CALL_DELAY_MS;

  const elapsed = Date.now() - lastCallTime;
  if (elapsed < requiredDelay) {
    const waitMs = requiredDelay - elapsed;
    mcpLog(`Rate limit: inter-call delay ${waitMs}ms for '${toolName}'`);
    await sleep(waitMs);
  }

  // Record this call
  lastCallTime = Date.now();
  callHistory.push(lastCallTime);
}

/**
 * Get current rate limit status for diagnostics / debugging.
 */
export function getRateLimitStatus(): {
  callsThisRequest: number;
  callsThisMinute: number;
  lastCallAgoMs: number;
} {
  purgeOldEntries(Date.now());
  return {
    callsThisRequest: callsInCurrentRequest,
    callsThisMinute: callHistory.length,
    lastCallAgoMs: lastCallTime > 0 ? Date.now() - lastCallTime : -1,
  };
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

function purgeOldEntries(now: number): void {
  const cutoff = now - 60_000;
  while (callHistory.length > 0 && callHistory[0] < cutoff) {
    callHistory.shift();
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, Math.max(0, ms)));
}
