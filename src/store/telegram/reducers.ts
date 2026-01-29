import { PayloadAction } from "@reduxjs/toolkit";
import type {
  TelegramRootState,
  TelegramState,
  TelegramConnectionStatus,
  TelegramAuthStatus,
  TelegramUser,
  TelegramChat,
  TelegramMessage,
  TelegramThread,
} from "./types";
import { initialState } from "./types";

function ensureUser(
  state: TelegramRootState,
  userId: string,
): TelegramState {
  if (!state.byUser[userId]) {
    state.byUser[userId] = { ...initialState };
  }
  return state.byUser[userId];
}

export const reducers = {
  setConnectionStatus: (
    state: TelegramRootState,
    action: PayloadAction<{ userId: string; status: TelegramConnectionStatus }>,
  ) => {
    const u = ensureUser(state, action.payload.userId);
    u.connectionStatus = action.payload.status;
    if (action.payload.status !== "error") u.connectionError = null;
  },
  setConnectionError: (
    state: TelegramRootState,
    action: PayloadAction<{ userId: string; error: string | null }>,
  ) => {
    const u = ensureUser(state, action.payload.userId);
    u.connectionError = action.payload.error;
    if (action.payload.error) u.connectionStatus = "error";
  },
  setAuthStatus: (
    state: TelegramRootState,
    action: PayloadAction<{ userId: string; status: TelegramAuthStatus }>,
  ) => {
    const u = ensureUser(state, action.payload.userId);
    u.authStatus = action.payload.status;
    if (action.payload.status !== "error") u.authError = null;
  },
  setAuthError: (
    state: TelegramRootState,
    action: PayloadAction<{ userId: string; error: string | null }>,
  ) => {
    const u = ensureUser(state, action.payload.userId);
    u.authError = action.payload.error;
    if (action.payload.error) u.authStatus = "error";
  },
  setPhoneNumber: (
    state: TelegramRootState,
    action: PayloadAction<{ userId: string; phoneNumber: string | null }>,
  ) => {
    ensureUser(state, action.payload.userId).phoneNumber =
      action.payload.phoneNumber;
  },
  setSessionString: (
    state: TelegramRootState,
    action: PayloadAction<{ userId: string; sessionString: string | null }>,
  ) => {
    ensureUser(state, action.payload.userId).sessionString =
      action.payload.sessionString;
  },
  setCurrentUser: (
    state: TelegramRootState,
    action: PayloadAction<{ userId: string; user: TelegramUser | null }>,
  ) => {
    ensureUser(state, action.payload.userId).currentUser = action.payload.user;
  },
  setChats: (
    state: TelegramRootState,
    action: PayloadAction<{
      userId: string;
      chats: Record<string, TelegramChat>;
    }>,
  ) => {
    ensureUser(state, action.payload.userId).chats = action.payload.chats;
  },
  addChat: (
    state: TelegramRootState,
    action: PayloadAction<{ userId: string; chat: TelegramChat }>,
  ) => {
    const u = ensureUser(state, action.payload.userId);
    const chat = action.payload.chat;
    u.chats[chat.id] = chat;
    if (!u.chatsOrder.includes(chat.id)) u.chatsOrder.unshift(chat.id);
  },
  updateChat: (
    state: TelegramRootState,
    action: PayloadAction<{
      userId: string;
      id: string;
      updates: Partial<TelegramChat>;
    }>,
  ) => {
    const u = ensureUser(state, action.payload.userId);
    const { id, updates } = action.payload;
    if (u.chats[id]) u.chats[id] = { ...u.chats[id], ...updates };
  },
  removeChat: (
    state: TelegramRootState,
    action: PayloadAction<{ userId: string; chatId: string }>,
  ) => {
    const u = ensureUser(state, action.payload.userId);
    const chatId = action.payload.chatId;
    delete u.chats[chatId];
    u.chatsOrder = u.chatsOrder.filter((id) => id !== chatId);
    if (u.selectedChatId === chatId) u.selectedChatId = null;
  },
  setSelectedChat: (
    state: TelegramRootState,
    action: PayloadAction<{ userId: string; chatId: string | null }>,
  ) => {
    const u = ensureUser(state, action.payload.userId);
    const prev = u.selectedChatId;
    u.selectedChatId = action.payload.chatId;
    if (action.payload.chatId !== prev) u.selectedThreadId = null;
  },
  setChatsOrder: (
    state: TelegramRootState,
    action: PayloadAction<{ userId: string; order: string[] }>,
  ) => {
    ensureUser(state, action.payload.userId).chatsOrder = action.payload.order;
  },
  addMessage: (
    state: TelegramRootState,
    action: PayloadAction<{ userId: string; message: TelegramMessage }>,
  ) => {
    const u = ensureUser(state, action.payload.userId);
    const { chatId, id } = action.payload.message;
    if (!u.messages[chatId]) {
      u.messages[chatId] = {};
      u.messagesOrder[chatId] = [];
    }
    if (!u.messages[chatId][id]) {
      u.messages[chatId][id] = action.payload.message;
      u.messagesOrder[chatId].push(id);
    }
  },
  addMessages: (
    state: TelegramRootState,
    action: PayloadAction<{
      userId: string;
      chatId: string;
      messages: TelegramMessage[];
    }>,
  ) => {
    const u = ensureUser(state, action.payload.userId);
    const { chatId, messages } = action.payload;
    if (!u.messages[chatId]) {
      u.messages[chatId] = {};
      u.messagesOrder[chatId] = [];
    }
    messages.forEach((m) => {
      if (!u.messages[chatId][m.id]) {
        u.messages[chatId][m.id] = m;
        u.messagesOrder[chatId].push(m.id);
      }
    });
  },
  updateMessage: (
    state: TelegramRootState,
    action: PayloadAction<{
      userId: string;
      chatId: string;
      messageId: string;
      updates: Partial<TelegramMessage>;
    }>,
  ) => {
    const u = ensureUser(state, action.payload.userId);
    const { chatId, messageId, updates } = action.payload;
    if (u.messages[chatId]?.[messageId]) {
      u.messages[chatId][messageId] = {
        ...u.messages[chatId][messageId],
        ...updates,
      };
    }
  },
  removeMessage: (
    state: TelegramRootState,
    action: PayloadAction<{
      userId: string;
      chatId: string;
      messageId: string;
    }>,
  ) => {
    const u = ensureUser(state, action.payload.userId);
    const { chatId, messageId } = action.payload;
    if (u.messages[chatId]?.[messageId]) {
      delete u.messages[chatId][messageId];
      u.messagesOrder[chatId] = u.messagesOrder[chatId].filter(
        (id) => id !== messageId,
      );
    }
  },
  clearMessages: (
    state: TelegramRootState,
    action: PayloadAction<{ userId: string; chatId: string }>,
  ) => {
    const u = ensureUser(state, action.payload.userId);
    delete u.messages[action.payload.chatId];
    delete u.messagesOrder[action.payload.chatId];
  },
  addThread: (
    state: TelegramRootState,
    action: PayloadAction<{ userId: string; thread: TelegramThread }>,
  ) => {
    const u = ensureUser(state, action.payload.userId);
    const { chatId, id } = action.payload.thread;
    if (!u.threads[chatId]) {
      u.threads[chatId] = {};
      u.threadsOrder[chatId] = [];
    }
    if (!u.threads[chatId][id]) {
      u.threads[chatId][id] = action.payload.thread;
      u.threadsOrder[chatId].push(id);
    }
  },
  updateThread: (
    state: TelegramRootState,
    action: PayloadAction<{
      userId: string;
      chatId: string;
      threadId: string;
      updates: Partial<TelegramThread>;
    }>,
  ) => {
    const u = ensureUser(state, action.payload.userId);
    const { chatId, threadId, updates } = action.payload;
    if (u.threads[chatId]?.[threadId]) {
      u.threads[chatId][threadId] = {
        ...u.threads[chatId][threadId],
        ...updates,
      };
    }
  },
  setSelectedThread: (
    state: TelegramRootState,
    action: PayloadAction<{ userId: string; threadId: string | null }>,
  ) => {
    ensureUser(state, action.payload.userId).selectedThreadId =
      action.payload.threadId;
  },
  setSearchQuery: (
    state: TelegramRootState,
    action: PayloadAction<{ userId: string; query: string | null }>,
  ) => {
    ensureUser(state, action.payload.userId).searchQuery = action.payload.query;
  },
  setFilteredChatIds: (
    state: TelegramRootState,
    action: PayloadAction<{ userId: string; chatIds: string[] | null }>,
  ) => {
    ensureUser(state, action.payload.userId).filteredChatIds =
      action.payload.chatIds;
  },
  resetChats: (
    state: TelegramRootState,
    action: PayloadAction<{ userId: string }>,
  ) => {
    const u = ensureUser(state, action.payload.userId);
    u.chats = {};
    u.chatsOrder = [];
    u.selectedChatId = null;
  },
  resetMessages: (
    state: TelegramRootState,
    action: PayloadAction<{ userId: string }>,
  ) => {
    const u = ensureUser(state, action.payload.userId);
    u.messages = {};
    u.messagesOrder = {};
  },
  resetTelegramForUser: (
    state: TelegramRootState,
    action: PayloadAction<{ userId: string }>,
  ) => {
    state.byUser[action.payload.userId] = { ...initialState };
  },
};
