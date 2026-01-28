import { ActionReducerMapBuilder } from '@reduxjs/toolkit';
import type { TelegramState } from './types';
import {
  initializeTelegram,
  connectTelegram,
  checkAuthStatus,
  fetchChats,
  fetchMessages,
} from './thunks';
import type { TelegramUser } from './types';

export const buildExtraReducers = (builder: ActionReducerMapBuilder<TelegramState>) => {
  // Initialize
  builder
    .addCase(initializeTelegram.pending, (state) => {
      state.isInitialized = false;
    })
    .addCase(initializeTelegram.fulfilled, (state, action) => {
      state.isInitialized = true;
      state.sessionString = action.payload.sessionString;
    })
    .addCase(initializeTelegram.rejected, (state, action) => {
      state.isInitialized = false;
      state.connectionError = action.payload as string;
    });

  // Connect
  builder
    .addCase(connectTelegram.pending, (state) => {
      state.connectionStatus = 'connecting';
      state.connectionError = null;
    })
    .addCase(connectTelegram.fulfilled, (state) => {
      state.connectionStatus = 'connected';
      state.connectionError = null;
    })
    .addCase(connectTelegram.rejected, (state, action) => {
      state.connectionStatus = 'error';
      state.connectionError = action.payload as string;
    });

  // Check auth
  builder
    .addCase(checkAuthStatus.pending, (state) => {
      state.authStatus = 'authenticating';
    })
    .addCase(checkAuthStatus.fulfilled, (state, action) => {
      if (action.payload) {
        state.authStatus = 'authenticated';
        // Convert Api.User to TelegramUser
        // Handle both TelegramUser (from cached state) and User (from API)
        const payload = action.payload as TelegramUser | { id: number | string; firstName?: string; lastName?: string; username?: string; bot?: boolean; accessHash?: bigint | string };
        if ('isBot' in payload) {
          // Already a TelegramUser
          state.currentUser = payload;
        } else {
          // Convert from API User format
          state.currentUser = {
            id: String(payload.id),
            firstName: payload.firstName || '',
            lastName: payload.lastName,
            username: payload.username,
            isBot: Boolean(payload.bot),
            accessHash: payload.accessHash?.toString(),
          };
        }
      } else {
        state.authStatus = 'not_authenticated';
        state.currentUser = null;
      }
    })
    .addCase(checkAuthStatus.rejected, (state, action) => {
      state.authStatus = 'error';
      state.authError = action.payload as string;
    });

  // Fetch chats
  builder
    .addCase(fetchChats.pending, (state) => {
      state.isLoadingChats = true;
    })
    .addCase(fetchChats.fulfilled, (state) => {
      state.isLoadingChats = false;
      // Convert dialogs to chats
      // This is a placeholder - adjust based on actual API response
      // action.payload should be an array of dialogs
    })
    .addCase(fetchChats.rejected, (state) => {
      state.isLoadingChats = false;
    });

  // Fetch messages
  builder
    .addCase(fetchMessages.pending, (state) => {
      state.isLoadingMessages = true;
    })
    .addCase(fetchMessages.fulfilled, (state) => {
      state.isLoadingMessages = false;
      // Messages will be added via addMessages action
    })
    .addCase(fetchMessages.rejected, (state) => {
      state.isLoadingMessages = false;
    });
};
