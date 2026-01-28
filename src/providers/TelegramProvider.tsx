import { useEffect, useRef, useMemo, createContext, useContext, ReactNode } from 'react';
import { useAppSelector, useAppDispatch } from '../store/hooks';
import { selectIsAuthenticated, selectIsInitialized, selectConnectionStatus } from '../store/telegramSelectors';
import { initializeTelegram, connectTelegram } from '../store/telegramSlice';
import { mtprotoService } from '../services/mtprotoService';

// Helper to check if there's a saved session in localStorage
const hasSavedSession = (): boolean => {
  try {
    return !!localStorage.getItem('telegram_session');
  } catch {
    return false;
  }
};

interface TelegramContextType {
  isAuthenticated: boolean;
  isInitialized: boolean;
  connectionStatus: 'disconnected' | 'connecting' | 'connected' | 'error';
  checkConnection: () => Promise<boolean>;
}

const TelegramContext = createContext<TelegramContextType | undefined>(undefined);

export const useTelegram = () => {
  const context = useContext(TelegramContext);
  if (!context) {
    throw new Error('useTelegram must be used within TelegramProvider');
  }
  return context;
};

interface TelegramProviderProps {
  children: ReactNode;
}

/**
 * TelegramProvider manages the Telegram MTProto connection
 * - Initializes when authenticated
 * - Connects automatically
 * - Provides Telegram context to children
 */
const TelegramProvider = ({ children }: TelegramProviderProps) => {
  const dispatch = useAppDispatch();
  const isAuthenticated = useAppSelector(selectIsAuthenticated);
  const isInitialized = useAppSelector(selectIsInitialized);
  const connectionStatus = useAppSelector(selectConnectionStatus);
  const sessionString = useAppSelector((state) => state.telegram.sessionString);
  const initializedRef = useRef(false);
  const connectedRef = useRef(false);
  const setupInProgressRef = useRef(false);
  const setupCompleteRef = useRef(false);

  // Memoize hasSession to prevent unnecessary recalculations
  const hasSession = useMemo(() => {
    return !!sessionString || hasSavedSession();
  }, [sessionString]);

  // Initialize Telegram when authenticated or when we have a saved session
  useEffect(() => {
    // Reset refs if we're not authenticated and have no session
    if (!isAuthenticated && !hasSession) {
      initializedRef.current = false;
      connectedRef.current = false;
      setupInProgressRef.current = false;
      setupCompleteRef.current = false;
      return;
    }

    // If setup is already complete and everything is connected, don't run again
    if (setupCompleteRef.current && isInitialized && connectionStatus === 'connected') {
      return;
    }

    // Prevent multiple simultaneous setup attempts
    if (setupInProgressRef.current) {
      return;
    }

    const setupTelegram = async () => {
      // Mark setup as in progress
      setupInProgressRef.current = true;

      try {
        // Initialize if not already initialized
        if (!isInitialized && !initializedRef.current) {
          initializedRef.current = true;
          await dispatch(initializeTelegram()).unwrap();
          setupInProgressRef.current = false;
          return; // Exit early, will re-run when isInitialized updates
        }

        // Connect if not already connected
        if (isInitialized && connectionStatus !== 'connected' && !connectedRef.current) {
          connectedRef.current = true;
          await dispatch(connectTelegram()).unwrap();
          setupInProgressRef.current = false;
          return; // Exit early, will re-run when connectionStatus updates
        }

        // Setup complete - mark as done
        setupInProgressRef.current = false;
        setupCompleteRef.current = true;
      } catch (error) {
        console.error('Failed to setup Telegram:', error);
        initializedRef.current = false;
        connectedRef.current = false;
        setupInProgressRef.current = false;
        setupCompleteRef.current = false;
      }
    };

    setupTelegram();
  }, [isAuthenticated, hasSession, isInitialized, connectionStatus, dispatch]);

  // Check connection status periodically to keep user online
  useEffect(() => {
    if ((!isAuthenticated && !hasSession) || !isInitialized || connectionStatus !== 'connected') {
      return;
    }

    const checkConnection = async () => {
      try {
        await mtprotoService.checkConnection();
      } catch (error) {
        console.warn('Telegram connection check failed:', error);
      }
    };

    // Check immediately, then every 20 seconds
    checkConnection();
    const interval = setInterval(checkConnection, 20000);

    return () => clearInterval(interval);
  }, [isAuthenticated, hasSession, isInitialized, connectionStatus]);

  const checkConnection = async (): Promise<boolean> => {
    try {
      return await mtprotoService.checkConnection();
    } catch (error) {
      console.warn('Connection check failed:', error);
      return false;
    }
  };

  const value: TelegramContextType = {
    isAuthenticated: isAuthenticated || hasSession,
    isInitialized,
    connectionStatus,
    checkConnection,
  };

  return <TelegramContext.Provider value={value}>{children}</TelegramContext.Provider>;
};

export default TelegramProvider;
