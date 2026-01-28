import { createSlice } from '@reduxjs/toolkit';
import { initialState } from './types';
import { reducers } from './reducers';
import { buildExtraReducers } from './extraReducers';

const telegramSlice = createSlice({
  name: 'telegram',
  initialState,
  reducers: {
    ...reducers,
    resetTelegram: () => initialState,
  },
  extraReducers: buildExtraReducers,
});

export const {
  setConnectionStatus,
  setConnectionError,
  setAuthStatus,
  setAuthError,
  setPhoneNumber,
  setSessionString,
  setCurrentUser,
  setChats,
  addChat,
  updateChat,
  removeChat,
  setSelectedChat,
  setChatsOrder,
  addMessage,
  addMessages,
  updateMessage,
  removeMessage,
  clearMessages,
  addThread,
  updateThread,
  setSelectedThread,
  setSearchQuery,
  setFilteredChatIds,
  resetTelegram,
  resetChats,
  resetMessages,
} = telegramSlice.actions;

// Re-export thunks
export {
  initializeTelegram,
  connectTelegram,
  checkAuthStatus,
  fetchChats,
  fetchMessages,
} from './thunks';

// Re-export types
export type {
  TelegramConnectionStatus,
  TelegramAuthStatus,
  TelegramUser,
  TelegramChat,
  TelegramMessage,
  TelegramThread,
  TelegramState,
} from './types';

export default telegramSlice.reducer;
