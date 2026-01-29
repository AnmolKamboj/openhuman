import { useEffect, useRef } from "react";
import { useAppSelector } from "../store/hooks";
import { store } from "../store";
import { selectSocketStatus } from "../store/socketSelectors";
import { socketService } from "../services/socketService";
import {
  initTelegramMCPServer,
  getTelegramMCPServer,
  updateTelegramMCPServerSocket,
  cleanupTelegramMCPServer,
} from "../lib/mcp/telegram";

/**
 * SocketProvider manages the socket connection based on JWT token
 * - Connects when token is set
 * - Disconnects when token is unset
 */
const SocketProvider = ({ children }: { children: React.ReactNode }) => {
  const token = useAppSelector((state) => state.auth.token);
  const socketStatus = useAppSelector(selectSocketStatus);
  const previousTokenRef = useRef<string | null>(null);

  useEffect(() => {
    const previousToken = previousTokenRef.current;

    // Token was set - connect
    if (token && token !== previousToken) {
      socketService.connect(token);
      previousTokenRef.current = token;
    }

    // Token was unset - disconnect
    if (!token && previousToken) {
      socketService.disconnect();
      cleanupTelegramMCPServer();
      previousTokenRef.current = null;
    }
  }, [token]);

  // Handle MCP initialization when socket connects
  useEffect(() => {
    if (socketStatus === "connected") {
      const socket = socketService.getSocket();
      const server = getTelegramMCPServer();

      if (server) {
        updateTelegramMCPServerSocket(socket);
      } else {
        initTelegramMCPServer(socket);
      }
    } else if (socketStatus === "disconnected") {
      cleanupTelegramMCPServer();
    }
  }, [socketStatus]);

  // Cleanup on unmount only
  // Note: This should only run when the entire app unmounts, not on re-renders
  useEffect(() => {
    return () => {
      // Only disconnect on actual unmount (e.g., app closing)
      // Don't disconnect on re-renders or route changes
      // Check if token still exists - if it does, don't disconnect (might be a re-render)
      const currentToken = store.getState().auth.token;
      if (!currentToken) {
        socketService.disconnect();
      }
    };
  }, []); // Empty deps - only run cleanup on unmount

  return <>{children}</>;
};

export default SocketProvider;
