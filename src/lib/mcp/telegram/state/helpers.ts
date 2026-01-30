/**
 * Shared helpers for the Telegram state update layer.
 */

import { store } from "../../../../store";

/**
 * Get the current app user ID from Redux (used as the key in telegram.byUser).
 * Returns empty string if no user is logged in.
 */
export function getCurrentUserId(): string {
  return store.getState().user.user?._id ?? "";
}
