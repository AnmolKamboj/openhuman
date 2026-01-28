import { useEffect } from 'react';
import { useAppSelector } from '../store/hooks';
import { socketService } from '../services/socketService';
import {
  initTelegramMCPServer,
  getTelegramMCPServer,
  updateTelegramMCPServerSocket,
} from '../lib/mcp/telegram';

/**
 * MCPProvider initializes and updates MCP servers when the socket connects.
 * Place inside SocketProvider so the socket is available.
 */
const MCPProvider = ({ children }: { children: React.ReactNode }) => {
  const socketStatus = useAppSelector((state) => state.socket.status);

  useEffect(() => {
    if (socketStatus !== 'connected') return;

    const socket = socketService.getSocket();
    const server = getTelegramMCPServer();

    if (server) {
      updateTelegramMCPServerSocket(socket);
    } else {
      initTelegramMCPServer(socket);
    }
  }, [socketStatus]);

  return <>{children}</>;
};

export default MCPProvider;
