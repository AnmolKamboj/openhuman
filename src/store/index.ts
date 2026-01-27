import { configureStore } from "@reduxjs/toolkit";
import {
  persistStore,
  persistReducer,
  FLUSH,
  REHYDRATE,
  PAUSE,
  PERSIST,
  PURGE,
  REGISTER,
} from "redux-persist";
import storage from "redux-persist/lib/storage";
import authReducer from "./authSlice";
import socketReducer from "./socketSlice";
import userReducer from "./userSlice";
import telegramReducer from "./telegramSlice";
import { createLogger } from "redux-logger";
import { IS_DEV } from "../utils/config";

// Persist config for auth only
const authPersistConfig = {
  key: "auth",
  storage,
  // Persist token and onboarding status
  whitelist: ["token", "isOnboarded"],
};

// Persist config for telegram state
const telegramPersistConfig = {
  key: "telegram",
  storage,
  // Persist important state but not connection status (will reconnect on app start)
  whitelist: [
    "authStatus",
    "phoneNumber",
    "sessionString",
    "currentUser",
    "chats",
    "chatsOrder",
    "selectedChatId",
    "messages",
    "messagesOrder",
    "threads",
    "threadsOrder",
    "selectedThreadId",
    "hasMoreChats",
    "hasMoreMessages",
    "hasMoreThreads",
  ],
  // Don't persist connection status, errors, or loading states
  blacklist: [
    "connectionStatus",
    "connectionError",
    "authError",
    "isInitialized",
    "isLoadingChats",
    "isLoadingMessages",
    "isLoadingThreads",
    "searchQuery",
    "filteredChatIds",
  ],
};

const persistedAuthReducer = persistReducer(authPersistConfig, authReducer);
const persistedTelegramReducer = persistReducer(telegramPersistConfig, telegramReducer);

export const store = configureStore({
  reducer: {
    auth: persistedAuthReducer,
    socket: socketReducer,
    user: userReducer,
    telegram: persistedTelegramReducer,
  },
  middleware: (getDefaultMiddleware) => {
    const middleware = getDefaultMiddleware({
      serializableCheck: {
        ignoredActions: [FLUSH, REHYDRATE, PAUSE, PERSIST, PURGE, REGISTER],
      },
    });

    // Add redux-logger in development with collapsed groups
    if (IS_DEV) {
      return middleware.concat(
        createLogger({
          collapsed: true,
          duration: true,
          timestamp: true,
        }),
      );
    }
    return middleware;
  },
});

export const persistor = persistStore(store);

export type RootState = ReturnType<typeof store.getState>;
export type AppDispatch = typeof store.dispatch;
