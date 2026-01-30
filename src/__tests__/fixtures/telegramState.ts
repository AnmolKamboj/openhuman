/**
 * Factory builders for Telegram entity fixtures.
 */

import type {
  TelegramUser,
  TelegramChat,
  TelegramMessage,
  TelegramState,
} from "../../store/telegram/types";
import { initialState } from "../../store/telegram/types";

let idCounter = 1000;

export function createTelegramUser(
  overrides: Partial<TelegramUser> = {},
): TelegramUser {
  const id = String(idCounter++);
  return {
    id,
    firstName: "Test",
    lastName: "User",
    username: `user_${id}`,
    isBot: false,
    ...overrides,
  };
}

export function createTelegramChat(
  overrides: Partial<TelegramChat> = {},
): TelegramChat {
  const id = String(idCounter++);
  return {
    id,
    title: `Chat ${id}`,
    type: "private",
    unreadCount: 0,
    isPinned: false,
    ...overrides,
  };
}

export function createTelegramMessage(
  overrides: Partial<TelegramMessage> = {},
): TelegramMessage {
  const id = String(idCounter++);
  return {
    id,
    chatId: "1",
    date: Math.floor(Date.now() / 1000),
    message: `Message ${id}`,
    isOutgoing: false,
    isEdited: false,
    isForwarded: false,
    ...overrides,
  };
}

export function createTelegramState(
  overrides: Partial<TelegramState> = {},
): TelegramState {
  return {
    ...initialState,
    ...overrides,
  };
}
