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
    if (!token) {
      console.warn('[Socket] Cannot connect: no token provided');
      return;
    }

    // Don't connect if already connected with the same token
    if (this.socket?.connected && this.token === token) {
      console.log('[Socket] Already connected with same token');
      return;
    }

    // Disconnect existing connection if token changed or socket exists
    if (this.socket) {
      if (this.token !== token) {
        console.log('[Socket] Token changed, disconnecting old connection');
        this.disconnect();
      } else if (this.socket.connected) {
        console.log('[Socket] Already connected');
        return;
      } else if (!this.socket.disconnected) {
        // Socket is connecting, wait for it
        console.log('[Socket] Connection in progress, waiting...');
        return;
      }
    }

    this.token = token;

    // Update status to connecting
    store.dispatch(setStatus('connecting'));

    console.log('[Socket] Connecting to:', BACKEND_URL);
    console.log('[Socket] Token present:', token ? 'yes' : 'no');
    console.log('[Socket] Token length:', token?.length || 0);

    // Create socket connection with auth token
    // Note: path must match backend server configuration
    // Backend expects token in socket.handshake.auth.token
    this.socket = io(BACKEND_URL, {
      auth: {
        token: token, // Explicitly pass token in auth object
      },
      path: '/socket.io/',
      transports: ['websocket', 'polling'],
      reconnection: true,
      reconnectionDelay: 1000,
      reconnectionAttempts: 5,
      forceNew: true, // Force new connection to ensure auth is sent
    });

    // Connection event handlers
    this.socket.on('connect', () => {
      const socketId = this.socket?.id || null;
      console.log('[Socket] Connected with ID:', socketId);
      store.dispatch(setStatus('connected'));
      store.dispatch(setSocketId(socketId));
    });

    this.socket.on('ready', () => {
      console.log('[Socket] Server ready - authentication successful');
    });

    this.socket.on('error', (error: { message?: string; status?: number }) => {
      console.error('[Socket] Server error:', error);
      if (error.status === 403) {
        console.error('[Socket] Authentication failed - plan limit or access denied');
      }
    });

    this.socket.on('disconnect', (reason) => {
      console.log('[Socket] Disconnected:', reason);
      store.dispatch(setStatus('disconnected'));
      store.dispatch(setSocketId(null));
    });

    this.socket.on('connect_error', (error) => {
      console.error('[Socket] Connection error:', error);
      console.error('[Socket] Error message:', error.message);
      console.error('[Socket] Error type:', error.type);
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
