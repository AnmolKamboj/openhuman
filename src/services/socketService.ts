import { io, Socket } from "socket.io-client";
import { BACKEND_URL } from "../utils/config";
import { store } from "../store";
import {
  setStatusForUser,
  setSocketIdForUser,
  resetForUser,
} from "../store/socketSlice";

function getSocketUserId(): string {
  return store.getState().user.user?._id ?? "__pending__";
}

class SocketService {
  private socket: Socket | null = null;
  private token: string | null = null;

  /**
   * Connect to the socket server with authentication
   */
  connect(token: string): void {
    if (!token) return;

    // Don't connect if already connected with the same token
    if (this.socket?.connected && this.token === token) return;

    // Disconnect existing connection if token changed or socket exists
    if (this.socket) {
      if (this.token !== token) {
        this.disconnect();
      } else if (this.socket.connected) {
        return;
      } else if (!this.socket.disconnected) {
        // Socket is connecting, wait for it
        return;
      }
    }

    this.token = token;

    store.dispatch(
      setStatusForUser({ userId: getSocketUserId(), status: "connecting" }),
    );

    const backendUrl = BACKEND_URL;

    // Ensure we're not connecting to the wrong URL
    if (backendUrl.includes("localhost:1420") || backendUrl.includes(":1420")) {
      return;
    }

    // Create socket connection with auth token
    // Note: path must match backend server configuration
    // Backend expects token in socket.handshake.auth.token (NOT in query string)
    // Match the working test script configuration: start with polling, then upgrade
    // Socket.io sends auth in the handshake (POST request body for polling, not in GET headers)
    const socketOptions = {
      auth: { token },
      path: "/socket.io/",
      transports: ["websocket", "polling"], // Start with polling (more reliable), then upgrade to websocket
      reconnection: true,
      reconnectionDelay: 1000,
      reconnectionAttempts: 5,
      forceNew: true, // Force new connection to ensure auth is sent
      timeout: 2000, // Increase timeout for initial connection
      upgrade: true, // Allow upgrade from polling to websocket
      query: {}, // Explicitly prevent token from being added to query string
    };

    this.socket = io(backendUrl, socketOptions);

    // Connection event handlers
    this.socket.on("connect", () => {
      const socketId = this.socket?.id || null;
      const uid = getSocketUserId();
      console.log("connected", socketId, uid);
      store.dispatch(setStatusForUser({ userId: uid, status: "connected" }));
      store.dispatch(setSocketIdForUser({ userId: uid, socketId }));
    });

    this.socket.on("ready", () => {
      // Server ready - authentication successful
    });

    this.socket.on("error", () => {
      // Handle server errors
    });

    this.socket.on("disconnect", () => {
      const uid = getSocketUserId();
      store.dispatch(setStatusForUser({ userId: uid, status: "disconnected" }));
      store.dispatch(setSocketIdForUser({ userId: uid, socketId: null }));
    });

    this.socket.on("connect_error", () => {
      const uid = getSocketUserId();
      store.dispatch(setStatusForUser({ userId: uid, status: "disconnected" }));
    });

    this.socket.connect();
  }

  /**
   * Disconnect from the socket server
   */
  disconnect(): void {
    if (this.socket) {
      this.socket.disconnect();
      this.socket = null;
      this.token = null;
      store.dispatch(resetForUser({ userId: getSocketUserId() }));
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
