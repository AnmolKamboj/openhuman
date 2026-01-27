import { configureStore, Middleware } from '@reduxjs/toolkit';
import { persistStore, persistReducer, FLUSH, REHYDRATE, PAUSE, PERSIST, PURGE, REGISTER } from 'redux-persist';
import storage from 'redux-persist/lib/storage';
import authReducer from './authSlice';
import socketReducer from './socketSlice';
import userReducer from './userSlice';

// Persist config for auth only
const authPersistConfig = {
  key: 'auth',
  storage,
  // Persist token and onboarding status
  whitelist: ['token', 'isOnboarded'],
};

const persistedAuthReducer = persistReducer(authPersistConfig, authReducer);

// Get logger only in dev mode
let loggerMiddleware: Middleware | undefined = undefined;
if (import.meta.env.DEV) {
  try {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const createLogger = require('redux-logger');
    loggerMiddleware = createLogger.createLogger() as Middleware;
  } catch {
    // Logger not available, continue without it
  }
}

export const store = configureStore({
  reducer: {
    auth: persistedAuthReducer,
    socket: socketReducer,
    user: userReducer,
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
