import { createSlice } from "@reduxjs/toolkit";
import type { TelegramRootState } from "./types";
import { reducers } from "./reducers";
import { buildExtraReducers } from "./extraReducers";

const telegramInitialState: TelegramRootState = { byUser: {} };

const telegramSlice = createSlice({
  name: "telegram",
  initialState: telegramInitialState,
  reducers: {
    ...reducers,
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
  resetTelegramForUser,
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
} from "./thunks";

// Re-export types
export type {
  TelegramConnectionStatus,
  TelegramAuthStatus,
  TelegramUser,
  TelegramChat,
  TelegramMessage,
  TelegramThread,
  TelegramState,
  TelegramRootState,
} from "./types";
export { initialState } from "./types";

export default telegramSlice.reducer;
