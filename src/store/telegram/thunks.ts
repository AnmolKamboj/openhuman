import { createAsyncThunk } from '@reduxjs/toolkit';
import { mtprotoService } from '../../services/mtprotoService';
import type { TelegramUser } from './types';

// Global flag to prevent concurrent checkAuthStatus calls
let isCheckingAuth = false;
let lastCheckTime = 0;
const MIN_CHECK_INTERVAL = 5000; // 5 seconds minimum between checks

export const initializeTelegram = createAsyncThunk(
  'telegram/initialize',
  async (_, { rejectWithValue }) => {
    try {
      await mtprotoService.initialize();
      const sessionString = mtprotoService.getSessionString();
      return { sessionString };
    } catch (error) {
      return rejectWithValue(
        error instanceof Error ? error.message : 'Failed to initialize Telegram client'
      );
    }
  }
);

export const connectTelegram = createAsyncThunk(
  'telegram/connect',
  async (_, { rejectWithValue }) => {
    try {
      await mtprotoService.connect();
      return true;
    } catch (error) {
      return rejectWithValue(
        error instanceof Error ? error.message : 'Failed to connect to Telegram'
      );
    }
  }
);

export const checkAuthStatus = createAsyncThunk(
  'telegram/checkAuthStatus',
  async (_, { rejectWithValue, getState }) => {
    // Prevent concurrent calls
    const now = Date.now();
    if (isCheckingAuth && now - lastCheckTime < MIN_CHECK_INTERVAL) {
      // Return current state instead of making another call
      const state = getState() as { telegram: { authStatus: string; currentUser: TelegramUser | null } };
      return state.telegram.currentUser || null;
    }

    isCheckingAuth = true;
    lastCheckTime = now;

    try {
      const client = mtprotoService.getClient();
      
      // First check if we're authorized without making API calls
      const isAuthorized = await client.checkAuthorization();
      
      if (!isAuthorized) {
        isCheckingAuth = false;
        return null;
      }
      
      // Only call getMe() if we're authorized, with FLOOD_WAIT handling
      try {
        const me = await mtprotoService.withFloodWaitHandling(async () => {
          return client.getMe();
        });
        isCheckingAuth = false;
        return me;
      } catch (error) {
        // If getMe() fails, we're not actually authorized
        // This can happen if the session is invalid
        console.warn('getMe() failed, user not authenticated:', error);
        isCheckingAuth = false;
        return null;
      }
    } catch (error) {
      isCheckingAuth = false;
      // If checkAuthorization itself fails, we're definitely not authenticated
      // Don't treat this as an error, just return null
      if (error instanceof Error && error.message.includes('AUTH_KEY_UNREGISTERED')) {
        return null;
      }
      return rejectWithValue(
        error instanceof Error ? error.message : 'Failed to check auth status'
      );
    }
  }
);

export const fetchChats = createAsyncThunk(
  'telegram/fetchChats',
  async (_, { rejectWithValue }) => {
    try {
      const client = mtprotoService.getClient();
      const dialogs = await mtprotoService.withFloodWaitHandling(async () => {
        return client.getDialogs({ limit: 100 });
      });
      return dialogs;
    } catch (error) {
      return rejectWithValue(
        error instanceof Error ? error.message : 'Failed to fetch chats'
      );
    }
  }
);

export const fetchMessages = createAsyncThunk(
  'telegram/fetchMessages',
  async (
    { chatId, limit = 50, offsetId }: { chatId: string; limit?: number; offsetId?: number },
    { rejectWithValue }
  ) => {
    try {
      const client = mtprotoService.getClient();
      // Implementation depends on GramJS API
      // This is a placeholder - adjust based on actual API
      const messages = await mtprotoService.withFloodWaitHandling(async () => {
        return client.getMessages(chatId, { limit, offsetId });
      });
      return { chatId, messages };
    } catch (error) {
      return rejectWithValue(
        error instanceof Error ? error.message : 'Failed to fetch messages'
      );
    }
  }
);
