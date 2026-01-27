import { io, Socket } from 'socket.io-client';
import { BACKEND_URL } from '../utils/config';
import { store } from '../store';
import { setStatus, setSocketId, reset } from '../store/socketSlice';

class SocketService {
  private socket: Socket | null = null;
  private token: string | null = null;

  /**
   * Connect to the socket server with authentication
   */
  connect(token: string): void {
    // Don't connect if already connected with the same token
    if (this.socket?.connected && this.token === token) {
      return;
    }

    // Disconnect existing connection if token changed
    if (this.socket && this.token !== token) {
      this.disconnect();
    }

    // Don't connect if socket exists and is not disconnected (already connecting or connected)
    if (this.socket && !this.socket.disconnected) {
      return;
    }

    this.token = token;

    // Update status to connecting
    store.dispatch(setStatus('connecting'));

    // Create socket connection with auth token
    this.socket = io(BACKEND_URL, {
      auth: {
        token,
      },
      transports: ['websocket', 'polling'],
      reconnection: true,
      reconnectionDelay: 1000,
      reconnectionAttempts: 5,
    });

    // Connection event handlers
    this.socket.on('connect', () => {
      const socketId = this.socket?.id || null;
      store.dispatch(setStatus('connected'));
      store.dispatch(setSocketId(socketId));
    });

    this.socket.on('disconnect', () => {
      store.dispatch(setStatus('disconnected'));
      store.dispatch(setSocketId(null));
    });

    this.socket.on('connect_error', (error) => {
      console.error('[Socket] Connection error:', error);
      store.dispatch(setStatus('disconnected'));
    });
  }

  /**
   * Disconnect from the socket server
   */
  disconnect(): void {
    if (this.socket) {
      this.socket.disconnect();
      this.socket = null;
      this.token = null;
      store.dispatch(reset());
    }
  }

  /**
   * Get the current socket instance
   */
  getSocket(): Socket | null {
    return this.socket;
  }

  /**
   * Check if socket is connected
   */
  isConnected(): boolean {
    return this.socket?.connected || false;
  }

  /**
   * Emit an event to the server
   */
  emit(event: string, data?: unknown): void {
    if (this.socket?.connected) {
      this.socket.emit(event, data);
    } else {
      console.warn(`[Socket] Cannot emit '${event}': socket not connected`);
    }
  }

  /**
   * Listen to an event from the server
   */
  on(event: string, callback: (...args: unknown[]) => void): void {
    if (this.socket) {
      this.socket.on(event, callback);
    }
  }

  /**
   * Remove an event listener
   */
  off(event: string, callback?: (...args: unknown[]) => void): void {
    if (this.socket) {
      if (callback) {
        this.socket.off(event, callback);
      } else {
        this.socket.off(event);
      }
    }
  }

  /**
   * Listen to an event once
   */
  once(event: string, callback: (...args: unknown[]) => void): void {
    if (this.socket) {
      this.socket.once(event, callback);
    }
  }
}

export const socketService = new SocketService();
