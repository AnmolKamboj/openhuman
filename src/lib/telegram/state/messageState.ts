/**
 * State update functions for Telegram messages.
 *
 * Called by API read functions after fetching fresh data from Telegram
 * to keep the Redux store in sync.
 */

import { store } from "../../../store";
import {
  addChatMessagesById,
  updateMessage,
  addMessage,
  deleteChatMessages,
} from "../../../store/telegram";
import type { TelegramMessage } from "../../../store/telegram/types";
import { resolveUserId } from "./helpers";

/**
 * Add fetched messages into the Redux normalized message store for a chat.
 */
export function updateMessagesInState(
  chatId: string,
  messages: TelegramMessage[],
  userId?: string
): void {
  const uid = resolveUserId(userId);
  if (!uid || messages.length === 0) return;
  store.dispatch(addChatMessagesById({ userId: uid, chatId, messages }));
}

/**
 * Update reactions on a specific message in Redux state.
 */
export function updateMessageReactionsInState(
  chatId: string,
  messageId: string,
  reactions: Array<{ emoticon: string; count: number }>,
  userId?: string
): void {
  const uid = resolveUserId(userId);
  if (!uid) return;
  store.dispatch(
    updateMessage({ userId: uid, chatId, messageId, updates: { reactions } })
  );
}

/**
 * Add a single message to Redux state.
 */
export function addMessageToState(
  message: TelegramMessage,
  userId?: string
): void {
  const uid = resolveUserId(userId);
  if (!uid) return;
  store.dispatch(addMessage({ userId: uid, message }));
}

/**
 * Partially update fields on an existing message in Redux state.
 */
export function updateMessageFieldsInState(
  chatId: string,
  messageId: string,
  updates: Partial<TelegramMessage>,
  userId?: string
): void {
  const uid = resolveUserId(userId);
  if (!uid) return;
  store.dispatch(
    updateMessage({ userId: uid, chatId, messageId, updates })
  );
}

/**
 * Delete specific messages from a known chat in Redux state.
 */
export function deleteMessagesFromState(
  chatId: string,
  messageIds: string[],
  userId?: string
): void {
  const uid = resolveUserId(userId);
  if (!uid || messageIds.length === 0) return;
  store.dispatch(deleteChatMessages({ userId: uid, chatId, messageIds }));
}

/**
 * Search all chats in state to find which ones contain the given message IDs,
 * then delete them. Used for UpdateDeleteMessages which doesn't include chatId.
 */
export function findAndDeleteMessagesFromState(
  messageIds: string[],
  userId?: string
): void {
  const uid = resolveUserId(userId);
  if (!uid || messageIds.length === 0) return;

  const state = store.getState();
  const userState = state.telegram.byUser[uid];
  if (!userState) return;

  for (const chatId of Object.keys(userState.messages)) {
    const chatMsgs = userState.messages[chatId];
    const matching = messageIds.filter((id) => chatMsgs[id]);
    if (matching.length > 0) {
      store.dispatch(
        deleteChatMessages({ userId: uid, chatId, messageIds: matching })
      );
    }
  }
}
