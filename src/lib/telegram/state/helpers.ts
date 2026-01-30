/**
 * Shared helpers for the Telegram state update layer.
 */

import { store } from "../../../store";

/**
 * Get the current app user ID from Redux (used as the key in telegram.byUser).
 * Returns empty string if no user is logged in.
 */
export function getCurrentUserId(): string {
  return store.getState().user.user?._id ?? "";
}

/**
 * Resolve the user ID to use for state operations.
 * If an explicit userId is provided, use it; otherwise fall back to getCurrentUserId().
 */
export function resolveUserId(userId?: string): string {
  return userId ?? getCurrentUserId();
}
