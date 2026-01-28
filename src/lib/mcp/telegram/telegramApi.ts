/**
 * Telegram API helpers for MCP tools
 * Uses mtprotoService + Redux telegram state (alphahuman)
 */

import { store } from '../../../store';
import { mtprotoService } from '../../../services/mtprotoService';
import {
  selectOrderedChats,
  selectChatMessages,
  selectCurrentUser,
} from '../../../store/telegramSelectors';
import type { TelegramChat, TelegramUser, TelegramMessage } from '../../../store/telegram/types';

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

function getTelegramState() {
  return store.getState().telegram;
}

/**
 * Get chat by ID or username
 */
export function getChatById(chatId: string | number): TelegramChat | undefined {
  const state = getTelegramState();
  const idStr = String(chatId);

  const chat = state.chats[idStr];
  if (chat) return chat;

  if (typeof chatId === 'string' && (chatId.startsWith('@') || /^[a-zA-Z0-9_]+$/.test(chatId))) {
    const username = chatId.startsWith('@') ? chatId : `@${chatId}`;
    return Object.values(state.chats).find(
      (c) => c.username && (c.username === username || c.username === username.slice(1)),
    );
  }

  return undefined;
}

/**
 * Get user by ID (current user only for now; no full user cache)
 */
export function getUserById(userId: string | number): TelegramUser | undefined {
  const state = getTelegramState();
  const current = state.currentUser;
  if (!current) return undefined;
  if (String(current.id) === String(userId)) return current;
  return undefined;
}

/**
 * Get messages from a chat (from store cache)
 * @param offset - numeric offset for pagination (default 0)
 */
export async function getMessages(
  chatId: string | number,
  limit = 20,
  offset = 0,
): Promise<TelegramMessage[] | undefined> {
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
 * Send a message to a chat
 */
export async function sendMessage(
  chatId: string | number,
  message: string,
  replyToMessageId?: number,
): Promise<{ id: string } | undefined> {
  const chat = getChatById(chatId);
  if (!chat) return undefined;

  const entity = chat.username ? `@${chat.username.replace('@', '')}` : chat.id;

  if (replyToMessageId !== undefined) {
    const client = mtprotoService.getClient();
    await mtprotoService.withFloodWaitHandling(async () => {
      await client.sendMessage(entity, {
        message,
        replyTo: replyToMessageId,
      });
    });
  } else {
    await mtprotoService.sendMessage(entity, message);
  }

  return { id: String(Date.now()) };
}

/**
 * Get list of chats (from store)
 */
export async function getChats(limit = 20): Promise<TelegramChat[]> {
  const state = store.getState();
  const ordered = selectOrderedChats(state);
  return ordered.slice(0, limit);
}

/**
 * Search chats by query (filter by title/username from store)
 */
export async function searchChats(query: string): Promise<TelegramChat[]> {
  const state = store.getState();
  const ordered = selectOrderedChats(state);
  const q = query.toLowerCase();
  return ordered.filter((c) => {
    const title = (c.title ?? '').toLowerCase();
    const un = (c.username ?? '').toLowerCase();
    return title.includes(q) || un.includes(q);
  });
}

/**
 * Get current user info
 */
export function getCurrentUser(): TelegramUser | undefined {
  const state = store.getState();
  return selectCurrentUser(state) ?? undefined;
}

/**
 * Format entity (chat or user) for display
 */
export function formatEntity(entity: TelegramChat | TelegramUser): FormattedEntity {
  if ('title' in entity) {
    const chat = entity as TelegramChat;
    const type = chat.type === 'channel' ? 'channel' : chat.type === 'supergroup' ? 'group' : chat.type;
    return {
      id: chat.id,
      name: chat.title ?? 'Unknown',
      type,
      username: chat.username,
    };
  }
  const user = entity as TelegramUser;
  const name = [user.firstName, user.lastName].filter(Boolean).join(' ') || 'Unknown';
  return {
    id: user.id,
    name,
    type: 'user',
    username: user.username,
    phone: user.phoneNumber,
  };
}

/**
 * Format message for display
 */
export function formatMessage(message: TelegramMessage): FormattedMessage {
  const result: FormattedMessage = {
    id: message.id,
    date: new Date(message.date * 1000).toISOString(),
    text: message.message ?? '',
  };
  if (message.fromId) result.from_id = message.fromId;
  if (message.media?.type) {
    result.has_media = true;
    result.media_type = message.media.type;
  }
  return result;
}
