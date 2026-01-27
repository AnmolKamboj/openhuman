import { createSelector } from '@reduxjs/toolkit';
import type { RootState } from './index';
import type { TelegramChat, TelegramMessage, TelegramThread } from './telegramSlice';

// Base selectors
export const selectTelegramState = (state: RootState) => state.telegram;

export const selectConnectionStatus = (state: RootState) => state.telegram.connectionStatus;
export const selectConnectionError = (state: RootState) => state.telegram.connectionError;
export const selectIsConnected = (state: RootState) =>
  state.telegram.connectionStatus === 'connected';

export const selectAuthStatus = (state: RootState) => state.telegram.authStatus;
export const selectAuthError = (state: RootState) => state.telegram.authError;
export const selectIsAuthenticated = (state: RootState) =>
  state.telegram.authStatus === 'authenticated';
export const selectIsInitialized = (state: RootState) => state.telegram.isInitialized;
export const selectPhoneNumber = (state: RootState) => state.telegram.phoneNumber;
export const selectCurrentUser = (state: RootState) => state.telegram.currentUser;

// Chat selectors
export const selectAllChats = (state: RootState) => state.telegram.chats;
export const selectChatsOrder = (state: RootState) => state.telegram.chatsOrder;
export const selectSelectedChatId = (state: RootState) => state.telegram.selectedChatId;
export const selectIsLoadingChats = (state: RootState) => state.telegram.isLoadingChats;

// Ordered chats selector
export const selectOrderedChats = createSelector(
  [selectAllChats, selectChatsOrder],
  (chats, order): TelegramChat[] => {
    return order.map((id) => chats[id]).filter(Boolean);
  }
);

// Selected chat selector
export const selectSelectedChat = createSelector(
  [selectAllChats, selectSelectedChatId],
  (chats, selectedId): TelegramChat | null => {
    return selectedId ? chats[selectedId] || null : null;
  }
);

// Filtered chats selector (for search)
export const selectFilteredChats = createSelector(
  [selectOrderedChats, (state: RootState) => state.telegram.filteredChatIds],
  (chats, filteredIds): TelegramChat[] => {
    if (!filteredIds) return chats;
    return chats.filter((chat) => filteredIds.includes(chat.id));
  }
);

// Message selectors
export const selectAllMessages = (state: RootState) => state.telegram.messages;
export const selectMessagesOrder = (state: RootState) => state.telegram.messagesOrder;
export const selectIsLoadingMessages = (state: RootState) => state.telegram.isLoadingMessages;

// Messages for a specific chat
export const selectChatMessages = createSelector(
  [
    selectAllMessages,
    selectMessagesOrder,
    (_: RootState, chatId: string) => chatId,
  ],
  (messages, messagesOrder, chatId): TelegramMessage[] => {
    const chatMessages = messages[chatId];
    const order = messagesOrder[chatId] || [];
    if (!chatMessages) return [];
    return order.map((id) => chatMessages[id]).filter(Boolean);
  }
);

// Latest message for a chat
export const selectChatLatestMessage = createSelector(
  [selectChatMessages],
  (messages): TelegramMessage | null => {
    return messages.length > 0 ? messages[messages.length - 1] : null;
  }
);

// Thread selectors
export const selectAllThreads = (state: RootState) => state.telegram.threads;
export const selectThreadsOrder = (state: RootState) => state.telegram.threadsOrder;
export const selectSelectedThreadId = (state: RootState) => state.telegram.selectedThreadId;
export const selectIsLoadingThreads = (state: RootState) => state.telegram.isLoadingThreads;

// Threads for a specific chat
export const selectChatThreads = createSelector(
  [
    selectAllThreads,
    selectThreadsOrder,
    (_: RootState, chatId: string) => chatId,
  ],
  (threads, threadsOrder, chatId): TelegramThread[] => {
    const chatThreads = threads[chatId];
    const order = threadsOrder[chatId] || [];
    if (!chatThreads) return [];
    return order.map((id) => chatThreads[id]).filter(Boolean);
  }
);

// Selected thread selector
export const selectSelectedThread = createSelector(
  [selectAllThreads, selectSelectedChatId, selectSelectedThreadId],
  (threads, chatId, threadId): TelegramThread | null => {
    if (!chatId || !threadId) return null;
    return threads[chatId]?.[threadId] || null;
  }
);

// Messages for selected thread
export const selectThreadMessages = createSelector(
  [
    selectChatMessages,
    selectSelectedChatId,
    selectSelectedThreadId,
  ],
  (messages, chatId, threadId): TelegramMessage[] => {
    if (!threadId) return [];
    return messages.filter((msg) => msg.threadId === threadId);
  }
);

// Search selectors
export const selectSearchQuery = (state: RootState) => state.telegram.searchQuery;
export const selectIsSearching = (state: RootState) => state.telegram.searchQuery !== null;

// Pagination selectors
export const selectHasMoreChats = (state: RootState) => state.telegram.hasMoreChats;
export const selectHasMoreMessages = (state: RootState) => state.telegram.hasMoreMessages;
export const selectHasMoreThreads = (state: RootState) => state.telegram.hasMoreThreads;

// Helper: Check if chat has more messages
export const selectChatHasMoreMessages = createSelector(
  [selectHasMoreMessages, (_: RootState, chatId: string) => chatId],
  (hasMore, chatId) => hasMore[chatId] ?? true
);

// Helper: Check if chat has more threads
export const selectChatHasMoreThreads = createSelector(
  [selectHasMoreThreads, (_: RootState, chatId: string) => chatId],
  (hasMore, chatId) => hasMore[chatId] ?? true
);

// Combined state selectors
export const selectTelegramReady = createSelector(
  [selectIsConnected, selectIsAuthenticated, selectIsInitialized],
  (isConnected, isAuthenticated, isInitialized) => {
    return isConnected && isAuthenticated && isInitialized;
  }
);

// Unread counts
export const selectTotalUnreadCount = createSelector(
  [selectOrderedChats],
  (chats) => {
    return chats.reduce((total, chat) => total + (chat.unreadCount || 0), 0);
  }
);

// Pinned chats
export const selectPinnedChats = createSelector(
  [selectOrderedChats],
  (chats): TelegramChat[] => {
    return chats.filter((chat) => chat.isPinned);
  }
);
