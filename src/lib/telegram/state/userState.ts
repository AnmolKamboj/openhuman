/**
 * State update functions for Telegram users.
 *
 * Called by API read functions after fetching fresh data from Telegram
 * to keep the Redux store in sync.
 */

import { store } from "../../../store";
import { setCurrentUser, addUsers, setUsers } from "../../../store/telegram";
import type { TelegramUser } from "../../../store/telegram/types";
import type { ApiUser } from "../api/apiResultTypes";
import { resolveUserId } from "./helpers";

/**
 * Update the current Telegram user in Redux state.
 */
export function updateCurrentUserInState(
  user: TelegramUser,
  userId?: string
): void {
  const uid = resolveUserId(userId);
  if (!uid) return;
  store.dispatch(setCurrentUser({ userId: uid, user }));
}

/**
 * Add TelegramUser objects into the Redux users map.
 */
export function updateUsersInState(
  users: TelegramUser[],
  userId?: string
): void {
  const uid = resolveUserId(userId);
  if (!uid || users.length === 0) return;

  const usersMap: Record<string, TelegramUser> = {};
  for (const user of users) {
    usersMap[user.id] = user;
  }
  store.dispatch(addUsers({ userId: uid, users: usersMap }));
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

/**
 * Replace all users in Redux state (used for the first batch of dialogs).
 */
export function replaceAllUsersInState(
  usersMap: Record<string, TelegramUser>,
  userId?: string
): void {
  const uid = resolveUserId(userId);
  if (!uid) return;
  store.dispatch(setUsers({ userId: uid, users: usersMap }));
}
