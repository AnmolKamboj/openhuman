/**
 * State update functions for Telegram users.
 *
 * Called by API read functions after fetching fresh data from Telegram
 * to keep the Redux store in sync.
 */

import { store } from "../../../../store";
import { setCurrentUser, addUsers } from "../../../../store/telegram";
import type { TelegramUser } from "../../../../store/telegram/types";
import type { ApiUser } from "../api/apiResultTypes";
import { getCurrentUserId } from "./helpers";

/**
 * Update the current Telegram user in Redux state.
 */
export function updateCurrentUserInState(user: TelegramUser): void {
  const userId = getCurrentUserId();
  if (!userId) return;
  store.dispatch(setCurrentUser({ userId, user }));
}

/**
 * Add TelegramUser objects into the Redux users map.
 */
export function updateUsersInState(users: TelegramUser[]): void {
  const userId = getCurrentUserId();
  if (!userId || users.length === 0) return;

  const usersMap: Record<string, TelegramUser> = {};
  for (const user of users) {
    usersMap[user.id] = user;
  }
  store.dispatch(addUsers({ userId, users: usersMap }));
}

/**
 * Convert raw Telegram API user objects to TelegramUser and update Redux state.
 * Preserves firstName/lastName split from the raw API response.
 */
export function updateUsersFromApiUsers(apiUsers: ApiUser[]): void {
  if (apiUsers.length === 0) return;

  const users: TelegramUser[] = apiUsers.map((u) => ({
    id: String(u.id),
    firstName: u.firstName ?? "",
    lastName: u.lastName,
    username: u.username,
    phoneNumber: u.phone,
    isBot: u.bot ?? false,
  }));
  updateUsersInState(users);
}
