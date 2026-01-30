/**
 * State update functions for Telegram messages.
 *
 * Called by API read functions after fetching fresh data from Telegram
 * to keep the Redux store in sync.
 */

import { store } from "../../../../store";
import {
  addChatMessagesById,
  updateMessage,
} from "../../../../store/telegram";
import type { TelegramMessage } from "../../../../store/telegram/types";
import { getCurrentUserId } from "./helpers";

/**
 * Add fetched messages into the Redux normalized message store for a chat.
 */
export function updateMessagesInState(
  chatId: string,
  messages: TelegramMessage[],
): void {
  const userId = getCurrentUserId();
  if (!userId || messages.length === 0) return;
  store.dispatch(addChatMessagesById({ userId, chatId, messages }));
}

/**
 * Update reactions on a specific message in Redux state.
 */
export function updateMessageReactionsInState(
  chatId: string,
  messageId: string,
  reactions: Array<{ emoticon: string; count: number }>,
): void {
  const userId = getCurrentUserId();
  if (!userId) return;
  store.dispatch(
    updateMessage({ userId, chatId, messageId, updates: { reactions } }),
  );
}
