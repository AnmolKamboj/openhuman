/**
 * Shared helpers for the Telegram API layer.
 *
 * Cache lookups, entity formatting, and message formatting live here.
 * These are used by both api/ functions and (transitionally) by tool wrappers.
 */

import { store } from "../../../store";
import {
  selectOrderedChats,
  selectCurrentUser,
  selectTelegramUserState,
} from "../../../store/telegramSelectors";
import type {
  TelegramChat,
  TelegramUser,
  TelegramMessage,
} from "../../../store/telegram/types";

// ---------------------------------------------------------------------------
// Redux state helpers
// ---------------------------------------------------------------------------

export function getTelegramState() {
  return selectTelegramUserState(store.getState());
}

// ---------------------------------------------------------------------------
// Cache lookups
// ---------------------------------------------------------------------------

/**
 * Get chat by ID or username from Redux cache.
 */
export function getChatById(chatId: string | number): TelegramChat | undefined {
  const state = getTelegramState();
  const idStr = String(chatId);

  const chat = state.chats[idStr];
  if (chat) return chat;

  if (
    typeof chatId === "string" &&
    (chatId.startsWith("@") || /^[a-zA-Z0-9_]+$/.test(chatId))
  ) {
    const username = chatId.startsWith("@") ? chatId : `@${chatId}`;
    return Object.values(state.chats).find(
      (c) =>
        c.username &&
        (c.username === username || c.username === username.slice(1))
    );
  }

  return undefined;
}

/**
 * Get user by ID (current user only; no full user cache).
 */
export function getUserById(userId: string | number): TelegramUser | undefined {
  const state = getTelegramState();
  const current = state.currentUser;
  if (!current) return undefined;
  if (String(current.id) === String(userId)) return current;
  return undefined;
}

/**
 * Get current user from Redux cache.
 */
export function getCurrentUser(): TelegramUser | undefined {
  const state = store.getState();
  return selectCurrentUser(state) ?? undefined;
}

/**
 * Get ordered chats from Redux cache.
 */
export function getOrderedChats(limit = 20): TelegramChat[] {
  const state = store.getState();
  const ordered = selectOrderedChats(state);
  return ordered.slice(0, limit);
}

/**
 * Get cached messages for a chat from Redux store.
 */
export function getCachedMessages(
  chatId: string | number,
  limit = 20,
  offset = 0
): TelegramMessage[] | undefined {
  const chat = getChatById(chatId);
  if (!chat) return undefined;

  const state = getTelegramState();
  const order = state.messagesOrder[chat.id] ?? [];
  const byId = state.messages[chat.id] ?? {};
  const all = order.map((id) => byId[id]).filter(Boolean);
  const list = all.slice(offset, offset + limit);

  return list.length ? list : undefined;
}

/**
 * Search chats by query (filter by title/username from cache).
 */
export function searchChatsInCache(query: string): TelegramChat[] {
  const state = store.getState();
  const ordered = selectOrderedChats(state);
  const q = query.toLowerCase();
  return ordered.filter((c) => {
    const title = (c.title ?? "").toLowerCase();
    const un = (c.username ?? "").toLowerCase();
    return title.includes(q) || un.includes(q);
  });
}

// ---------------------------------------------------------------------------
// Formatting helpers
// ---------------------------------------------------------------------------

export interface FormattedEntity {
  id: string;
  name: string;
  type: string;
  username?: string;
  phone?: string;
}

export interface FormattedMessage {
  id: number | string;
  date: string;
  text: string;
  from_id?: string;
  has_media?: boolean;
  media_type?: string;
}

/**
 * Format entity (chat or user) for display.
 */
export function formatEntity(
  entity: TelegramChat | TelegramUser
): FormattedEntity {
  if ("title" in entity) {
    const chat = entity as TelegramChat;
    const type =
      chat.type === "channel"
        ? "channel"
        : chat.type === "supergroup"
        ? "group"
        : chat.type;
    return {
      id: chat.id,
      name: chat.title ?? "Unknown",
      type,
      username: chat.username,
    };
  }
  const user = entity as TelegramUser;
  const name =
    [user.firstName, user.lastName].filter(Boolean).join(" ") || "Unknown";
  return {
    id: user.id,
    name,
    type: "user",
    username: user.username,
    phone: user.phoneNumber,
  };
}

/**
 * Format message for display.
 */
export function formatMessage(message: TelegramMessage): FormattedMessage {
  const result: FormattedMessage = {
    id: message.id,
    date: new Date(message.date * 1000).toISOString(),
    text: message.message ?? "",
  };
  if (message.fromId) result.from_id = message.fromId;
  if (message.media?.type) {
    result.has_media = true;
    result.media_type = message.media.type;
  }
  return result;
}

// ---------------------------------------------------------------------------
// GramJS conversion helpers (used by API fallback functions)
// ---------------------------------------------------------------------------

/** Convert a raw GramJS message to our TelegramMessage format */
export function apiMessageToTelegramMessage(
  msg: Record<string, unknown>,
  chatId: string
): TelegramMessage {
  const fromId =
    msg.fromId &&
    typeof msg.fromId === "object" &&
    "userId" in (msg.fromId as object)
      ? String((msg.fromId as { userId: unknown }).userId)
      : undefined;

  const replyTo = msg.replyTo as { replyToMsgId?: number } | undefined;

  const media = msg.media as { className?: string } | undefined;
  let mediaInfo: TelegramMessage["media"] | undefined;
  if (media && media.className && media.className !== "MessageMediaEmpty") {
    mediaInfo = { type: media.className };
  }

  return {
    id: String(msg.id ?? ""),
    chatId,
    date: typeof msg.date === "number" ? msg.date : 0,
    message: typeof msg.message === "string" ? msg.message : "",
    fromId,
    isOutgoing: Boolean(msg.out),
    isEdited: msg.editDate != null,
    isForwarded: msg.fwdFrom != null,
    replyToMessageId:
      replyTo?.replyToMsgId != null ? String(replyTo.replyToMsgId) : undefined,
    media: mediaInfo,
  };
}

/** Convert a raw GramJS dialog + chat/user to our TelegramChat format */
export function apiDialogToTelegramChat(
  dialog: Record<string, unknown>,
  chatsById: Map<string, Record<string, unknown>>,
  usersById: Map<string, Record<string, unknown>>
): TelegramChat | undefined {
  const peer = dialog.peer as
    | {
        className?: string;
        userId?: unknown;
        chatId?: unknown;
        channelId?: unknown;
      }
    | undefined;
  if (!peer) return undefined;

  let id: string;
  let type: TelegramChat["type"];
  let raw: Record<string, unknown> | undefined;

  if (peer.className === "PeerUser" && peer.userId != null) {
    id = String(peer.userId);
    type = "private";
    raw = usersById.get(id);
  } else if (peer.className === "PeerChat" && peer.chatId != null) {
    id = String(peer.chatId);
    type = "group";
    raw = chatsById.get(id);
  } else if (peer.className === "PeerChannel" && peer.channelId != null) {
    id = String(peer.channelId);
    raw = chatsById.get(id);
    type = raw && Boolean(raw.megagroup) ? "supergroup" : "channel";
  } else {
    return undefined;
  }

  let title: string;
  let username: string | undefined;
  if (raw) {
    title =
      (raw.title as string) ??
      [raw.firstName, raw.lastName].filter(Boolean).join(" ") ??
      "Unknown";
    username = raw.username as string | undefined;
  } else {
    title = "Unknown";
  }

  return {
    id,
    title,
    type,
    username,
    unreadCount:
      typeof dialog.unreadCount === "number" ? dialog.unreadCount : 0,
    isPinned: Boolean(dialog.pinned),
  };
}
