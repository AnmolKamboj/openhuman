import { configureStore } from '@reduxjs/toolkit';
import { persistStore, persistReducer, FLUSH, REHYDRATE, PAUSE, PERSIST, PURGE, REGISTER } from 'redux-persist';
import storage from 'redux-persist/lib/storage';
import authReducer from './authSlice';
import socketReducer from './socketSlice';

// Persist config for auth only
const authPersistConfig = {
  key: 'auth',
  storage,
  // Only persist the token
  whitelist: ['token'],
};

// Custom storage that syncs with localStorage 'sessionToken' for backward compatibility
// redux-persist stores data as JSON string with the state object
const customStorage = {
  getItem: (key: string): Promise<string | null> => {
    return new Promise((resolve) => {
      // First check redux-persist storage (redux-persist adds 'persist:' prefix)
      const persistKey = `persist:${key}`;
      const persistData = localStorage.getItem(persistKey);
      
      if (persistData) {
        try {
          const parsed = JSON.parse(persistData);
          // redux-persist format: { _persist: {...}, token: "..." }
          if (parsed && typeof parsed === 'object') {
            resolve(persistData);
            return;
          }
        } catch {
          // Ignore parse errors
        }
      }
      
      // Fallback to legacy sessionToken for backward compatibility
      const legacyToken = localStorage.getItem('sessionToken');
      if (legacyToken) {
        // Create redux-persist compatible format
        const fallbackData = JSON.stringify({
          token: legacyToken,
          _persist: { version: -1, rehydrated: true },
        });
        localStorage.setItem(persistKey, fallbackData);
        resolve(fallbackData);
        return;
      }
      
      resolve(null);
    });
  },
  setItem: (key: string, value: string): Promise<void> => {
    return new Promise((resolve) => {
      const persistKey = `persist:${key}`;
      localStorage.setItem(persistKey, value);
      // Also sync to sessionToken for backward compatibility
      try {
        const parsed = JSON.parse(value);
        if (parsed && typeof parsed === 'object' && parsed.token) {
          localStorage.setItem('sessionToken', parsed.token);
        }
      } catch {
        // Ignore parse errors
      }
      resolve();
    });
  },
  removeItem: (key: string): Promise<void> => {
    return new Promise((resolve) => {
      const persistKey = `persist:${key}`;
      localStorage.removeItem(persistKey);
      localStorage.removeItem('sessionToken');
      resolve();
    });
  },
};

const authPersistConfigWithCustomStorage = {
  ...authPersistConfig,
  storage: customStorage,
};

const persistedAuthReducer = persistReducer(authPersistConfigWithCustomStorage, authReducer);

// Get logger only in dev mode
let loggerMiddleware: unknown = undefined;
if (import.meta.env.DEV) {
  try {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const createLogger = require('redux-logger');
    loggerMiddleware = createLogger.createLogger();
  } catch {
    // Logger not available, continue without it
  }
}

export const store = configureStore({
  reducer: {
    auth: persistedAuthReducer,
    socket: socketReducer,
  },
  middleware: (getDefaultMiddleware) => {
    const middleware = getDefaultMiddleware({
      serializableCheck: {
        ignoredActions: [FLUSH, REHYDRATE, PAUSE, PERSIST, PURGE, REGISTER],
      },
    });
    
    // Add redux-logger in development
    if (loggerMiddleware) {
      return middleware.concat(loggerMiddleware);
    }
    
    return middleware;
  },
});

export const persistor = persistStore(store);

export type RootState = ReturnType<typeof store.getState>;
export type AppDispatch = typeof store.dispatch;
