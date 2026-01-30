/**
 * State update functions for Telegram chats.
 *
 * Called by API read functions after fetching fresh data from Telegram
 * to keep the Redux store in sync.
 */

import { store } from "../../../store";
import {
  addChats,
  addChat,
  updateChat,
  replaceChats,
} from "../../../store/telegram";
import type { TelegramChat } from "../../../store/telegram/types";
import { resolveUserId } from "./helpers";

/**
 * Add or merge fetched chats into Redux state.
 */
export function updateChatsInState(
  chats: TelegramChat[],
  userId?: string
): void {
  const uid = resolveUserId(userId);
  if (!uid || chats.length === 0) return;

  const chatsMap: Record<string, TelegramChat> = {};
  const order: string[] = [];
  for (const chat of chats) {
    chatsMap[chat.id] = chat;
    order.push(chat.id);
  }
  store.dispatch(addChats({ userId: uid, chats: chatsMap, appendOrder: order }));
}

/**
 * Add or update a single chat in Redux state.
 */
export function updateChatInState(
  chat: TelegramChat,
  userId?: string
): void {
  const uid = resolveUserId(userId);
  if (!uid) return;
  store.dispatch(addChat({ userId: uid, chat }));
}

/** Map a generic type string to TelegramChat["type"] */
function mapChatType(type: string): TelegramChat["type"] {
  switch (type) {
    case "channel":
      return "channel";
    case "supergroup":
      return "supergroup";
    case "group":
    case "chat":
      return "group";
    case "user":
    case "bot":
    case "private":
      return "private";
    default:
      return "group";
  }
}

/**
 * Update state from a single chat-like result (e.g., from resolveUsername).
 */
export function updateChatFromResult(result: {
  id: string;
  name: string;
  type: string;
  username?: string;
}): void {
  updateChatInState({
    id: result.id,
    title: result.name,
    type: mapChatType(result.type),
    username: result.username,
    unreadCount: 0,
    isPinned: false,
  });
}

/**
 * Update state from multiple chat-like results (e.g., from searchPublicChats).
 */
export function updateChatsFromResults(
  results: Array<{
    id: string;
    name: string;
    type: string;
    username?: string;
  }>
): void {
  const uid = resolveUserId();
  if (!uid || results.length === 0) return;

  const chatsMap: Record<string, TelegramChat> = {};
  const order: string[] = [];
  for (const r of results) {
    const chat: TelegramChat = {
      id: r.id,
      title: r.name,
      type: mapChatType(r.type),
      username: r.username,
      unreadCount: 0,
      isPinned: false,
    };
    chatsMap[r.id] = chat;
    order.push(r.id);
  }
  store.dispatch(addChats({ userId: uid, chats: chatsMap, appendOrder: order }));
}

/**
 * Partially update fields on an existing chat in Redux state.
 */
export function updateChatFieldsInState(
  chatId: string,
  updates: Partial<TelegramChat>,
  userId?: string
): void {
  const uid = resolveUserId(userId);
  if (!uid) return;
  store.dispatch(updateChat({ userId: uid, id: chatId, updates }));
}

/**
 * Replace all chats in Redux state (used for the first batch of dialogs).
 */
export function replaceAllChatsInState(
  chatsMap: Record<string, TelegramChat>,
  order: string[],
  userId?: string
): void {
  const uid = resolveUserId(userId);
  if (!uid) return;
  store.dispatch(
    replaceChats({ userId: uid, chats: chatsMap, chatsOrder: order })
  );
}

/**
 * Add a batch of chats (used for subsequent dialog batches).
 */
export function addChatsWithUsersInState(
  chatsMap: Record<string, TelegramChat>,
  order: string[],
  userId?: string
): void {
  const uid = resolveUserId(userId);
  if (!uid) return;
  store.dispatch(
    addChats({ userId: uid, chats: chatsMap, appendOrder: order })
  );
}
