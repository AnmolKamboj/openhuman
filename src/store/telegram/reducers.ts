import { PayloadAction } from '@reduxjs/toolkit';
import type {
  TelegramState,
  TelegramConnectionStatus,
  TelegramAuthStatus,
  TelegramUser,
  TelegramChat,
  TelegramMessage,
  TelegramThread,
} from './types';

export const reducers = {
  // Connection actions
  setConnectionStatus: (state: TelegramState, action: PayloadAction<TelegramConnectionStatus>) => {
    state.connectionStatus = action.payload;
    if (action.payload !== 'error') {
      state.connectionError = null;
    }
  },
  setConnectionError: (state: TelegramState, action: PayloadAction<string | null>) => {
    state.connectionError = action.payload;
    if (action.payload) {
      state.connectionStatus = 'error';
    }
  },

  // Authentication actions
  setAuthStatus: (state: TelegramState, action: PayloadAction<TelegramAuthStatus>) => {
    state.authStatus = action.payload;
    if (action.payload !== 'error') {
      state.authError = null;
    }
  },
  setAuthError: (state: TelegramState, action: PayloadAction<string | null>) => {
    state.authError = action.payload;
    if (action.payload) {
      state.authStatus = 'error';
    }
  },
  setPhoneNumber: (state: TelegramState, action: PayloadAction<string | null>) => {
    state.phoneNumber = action.payload;
  },
  setSessionString: (state: TelegramState, action: PayloadAction<string | null>) => {
    state.sessionString = action.payload;
  },

  // User actions
  setCurrentUser: (state: TelegramState, action: PayloadAction<TelegramUser | null>) => {
    state.currentUser = action.payload;
  },

  // Chat actions
  setChats: (state: TelegramState, action: PayloadAction<Record<string, TelegramChat>>) => {
    state.chats = action.payload;
  },
  addChat: (state: TelegramState, action: PayloadAction<TelegramChat>) => {
    const chat = action.payload;
    state.chats[chat.id] = chat;
    if (!state.chatsOrder.includes(chat.id)) {
      state.chatsOrder.unshift(chat.id);
    }
  },
  updateChat: (state: TelegramState, action: PayloadAction<Partial<TelegramChat> & { id: string }>) => {
    const { id, ...updates } = action.payload;
    if (state.chats[id]) {
      state.chats[id] = { ...state.chats[id], ...updates };
    }
  },
  removeChat: (state: TelegramState, action: PayloadAction<string>) => {
    const chatId = action.payload;
    delete state.chats[chatId];
    state.chatsOrder = state.chatsOrder.filter((id) => id !== chatId);
    if (state.selectedChatId === chatId) {
      state.selectedChatId = null;
    }
  },
  setSelectedChat: (state: TelegramState, action: PayloadAction<string | null>) => {
    state.selectedChatId = action.payload;
    // Clear selected thread when changing chat
    if (action.payload !== state.selectedChatId) {
      state.selectedThreadId = null;
    }
  },
  setChatsOrder: (state: TelegramState, action: PayloadAction<string[]>) => {
    state.chatsOrder = action.payload;
  },

  // Message actions
  addMessage: (state: TelegramState, action: PayloadAction<TelegramMessage>) => {
    const message = action.payload;
    const { chatId, id } = message;

    if (!state.messages[chatId]) {
      state.messages[chatId] = {};
      state.messagesOrder[chatId] = [];
    }

    if (!state.messages[chatId][id]) {
      state.messages[chatId][id] = message;
      state.messagesOrder[chatId].push(id);
    }
  },
  addMessages: (state: TelegramState, action: PayloadAction<{ chatId: string; messages: TelegramMessage[] }>) => {
    const { chatId, messages } = action.payload;

    if (!state.messages[chatId]) {
      state.messages[chatId] = {};
      state.messagesOrder[chatId] = [];
    }

    messages.forEach((message) => {
      if (!state.messages[chatId][message.id]) {
        state.messages[chatId][message.id] = message;
        state.messagesOrder[chatId].push(message.id);
      }
    });
  },
  updateMessage: (
    state: TelegramState,
    action: PayloadAction<{ chatId: string; messageId: string; updates: Partial<TelegramMessage> }>
  ) => {
    const { chatId, messageId, updates } = action.payload;
    if (state.messages[chatId]?.[messageId]) {
      state.messages[chatId][messageId] = {
        ...state.messages[chatId][messageId],
        ...updates,
      };
    }
  },
  removeMessage: (state: TelegramState, action: PayloadAction<{ chatId: string; messageId: string }>) => {
    const { chatId, messageId } = action.payload;
    if (state.messages[chatId]?.[messageId]) {
      delete state.messages[chatId][messageId];
      state.messagesOrder[chatId] = state.messagesOrder[chatId].filter(
        (id) => id !== messageId
      );
    }
  },
  clearMessages: (state: TelegramState, action: PayloadAction<string>) => {
    const chatId = action.payload;
    delete state.messages[chatId];
    delete state.messagesOrder[chatId];
  },

  // Thread actions
  addThread: (state: TelegramState, action: PayloadAction<TelegramThread>) => {
    const thread = action.payload;
    const { chatId, id } = thread;

    if (!state.threads[chatId]) {
      state.threads[chatId] = {};
      state.threadsOrder[chatId] = [];
    }

    if (!state.threads[chatId][id]) {
      state.threads[chatId][id] = thread;
      state.threadsOrder[chatId].push(id);
    }
  },
  updateThread: (
    state: TelegramState,
    action: PayloadAction<{ chatId: string; threadId: string; updates: Partial<TelegramThread> }>
  ) => {
    const { chatId, threadId, updates } = action.payload;
    if (state.threads[chatId]?.[threadId]) {
      state.threads[chatId][threadId] = {
        ...state.threads[chatId][threadId],
        ...updates,
      };
    }
  },
  setSelectedThread: (state: TelegramState, action: PayloadAction<string | null>) => {
    state.selectedThreadId = action.payload;
  },

  // Search actions
  setSearchQuery: (state: TelegramState, action: PayloadAction<string | null>) => {
    state.searchQuery = action.payload;
  },
  setFilteredChatIds: (state: TelegramState, action: PayloadAction<string[] | null>) => {
    state.filteredChatIds = action.payload;
  },

  // Reset actions
  // Note: resetTelegram is handled in index.ts to return initialState
  resetChats: (state: TelegramState) => {
    state.chats = {};
    state.chatsOrder = [];
    state.selectedChatId = null;
  },
  resetMessages: (state: TelegramState) => {
    state.messages = {};
    state.messagesOrder = {};
  },
};
