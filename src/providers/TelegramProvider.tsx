import {
  useEffect,
  useRef,
  createContext,
  useContext,
  ReactNode,
} from "react";
import { useAppSelector, useAppDispatch } from "../store/hooks";
import {
  selectIsAuthenticated,
  selectIsInitialized,
  selectConnectionStatus,
  selectSessionString,
  selectTelegramCurrentUserId,
} from "../store/telegramSelectors";
import { initializeTelegram, connectTelegram } from "../store/telegram";
import { mtprotoService } from "../services/mtprotoService";

interface TelegramContextType {
  isAuthenticated: boolean;
  isInitialized: boolean;
  connectionStatus: "disconnected" | "connecting" | "connected" | "error";
  checkConnection: () => Promise<boolean>;
}

const TelegramContext = createContext<TelegramContextType | undefined>(
  undefined,
);

export const useTelegram = () => {
  const context = useContext(TelegramContext);
  if (!context) {
    throw new Error("useTelegram must be used within TelegramProvider");
  }
  return context;
};

interface TelegramProviderProps {
  children: ReactNode;
}

/**
 * TelegramProvider manages the Telegram MTProto connection
 * - Initializes when app-authenticated (JWT), or has Telegram session / authenticated
 * - Starts init+connect in parallel with login (token) so connect modal is ready sooner
 * - Connects automatically
 * - Provides Telegram context to children
 */
const TelegramProvider = ({ children }: TelegramProviderProps) => {
  const dispatch = useAppDispatch();
  const token = useAppSelector((state) => state.auth.token);
  const userId = useAppSelector(selectTelegramCurrentUserId);
  const isAuthenticated = useAppSelector(selectIsAuthenticated);
  const isInitialized = useAppSelector(selectIsInitialized);
  const connectionStatus = useAppSelector(selectConnectionStatus);
  const sessionString = useAppSelector(selectSessionString);
  const initializedRef = useRef(false);
  const connectedRef = useRef(false);
  const setupInProgressRef = useRef(false);
  const setupCompleteRef = useRef(false);

  const hasSession = !!sessionString;

  const shouldSetupTelegram = !!token && !!userId;

  // Initialize Telegram when app-authenticated (token), or have session / Telegram auth
  useEffect(() => {
    if (!shouldSetupTelegram) {
      initializedRef.current = false;
      connectedRef.current = false;
      setupInProgressRef.current = false;
      setupCompleteRef.current = false;
      return;
    }

    // If setup is already complete and everything is connected, don't run again
    if (
      setupCompleteRef.current &&
      isInitialized &&
      connectionStatus === "connected"
    ) {
      return;
    }

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
          await dispatch(initializeTelegram(userId)).unwrap();
          setupInProgressRef.current = false;
          return;
        }

        if (
          isInitialized &&
          connectionStatus !== "connected" &&
          !connectedRef.current
        ) {
          connectedRef.current = true;
          await dispatch(connectTelegram(userId)).unwrap();
          setupInProgressRef.current = false;
          return; // Exit early, will re-run when connectionStatus updates
        }

        // Setup complete - mark as done
        setupInProgressRef.current = false;
        setupCompleteRef.current = true;
      } catch (error) {
        console.error("Failed to setup Telegram:", error);
        initializedRef.current = false;
        connectedRef.current = false;
        setupInProgressRef.current = false;
        setupCompleteRef.current = false;
      }
    };

    setupTelegram();
  }, [
    shouldSetupTelegram,
    isInitialized,
    connectionStatus,
    dispatch,
    userId,
  ]);

  // Check connection status once when Telegram reports as connected
  useEffect(() => {
    if (
      !shouldSetupTelegram ||
      !isInitialized ||
      connectionStatus !== "connected"
    ) {
      return;
    }

    const uid = userId;
    const checkConnection = async () => {
      try {
        await mtprotoService.checkConnection(uid);
      } catch (error) {
        console.warn("Telegram connection check failed:", error);
      }
    };

    checkConnection();
  }, [shouldSetupTelegram, isInitialized, connectionStatus, userId]);

  const checkConnection = async (): Promise<boolean> => {
    try {
      return await mtprotoService.checkConnection(userId || undefined);
    } catch (error) {
      console.warn("Connection check failed:", error);
      return false;
    }
  };

  const value: TelegramContextType = {
    isAuthenticated: isAuthenticated || hasSession,
    isInitialized,
    connectionStatus,
    checkConnection,
  };

  return (
    <TelegramContext.Provider value={value}>
      {children}
    </TelegramContext.Provider>
  );
};

export default TelegramProvider;
