import { TelegramClient } from 'telegram';
import { StringSession } from 'telegram/sessions';
import type { UserAuthParams, BotAuthParams } from 'telegram/client/auth';
import { FloodWaitError } from 'telegram/errors';
import { TELEGRAM_API_ID, TELEGRAM_API_HASH } from '../utils/config';

type LoginOptions = UserAuthParams | BotAuthParams;

class MTProtoService {
  private static instance: MTProtoService | undefined;
  private client: TelegramClient | undefined;
  private isInitialized = false;
  private isConnected = false;
  private sessionString = '';
  private readonly apiId: number;
  private readonly apiHash: string;

  private constructor() {
    // Private constructor to enforce singleton
    // Load API credentials from config once
    if (!TELEGRAM_API_ID || !TELEGRAM_API_HASH) {
      throw new Error('TELEGRAM_API_ID and TELEGRAM_API_HASH must be configured');
    }
    this.apiId = TELEGRAM_API_ID;
    this.apiHash = TELEGRAM_API_HASH;
  }

  static getInstance(): MTProtoService {
    if (!MTProtoService.instance) {
      MTProtoService.instance = new MTProtoService();
    }
    return MTProtoService.instance;
  }

  /**
   * Initialize the MTProto client with API credentials
   */
  async initialize(): Promise<void> {
    if (this.isInitialized && this.client) {
      console.log('MTProto client already initialized');
      return;
    }

    const sessionString = this.loadSession() || '';

    try {
      const stringSession = new StringSession(sessionString);
      this.sessionString = sessionString;

      this.client = new TelegramClient(stringSession, this.apiId, this.apiHash, {
        connectionRetries: 5,
        requestRetries: 5,
        floodSleepThreshold: 60, // Auto-retry FLOOD_WAIT errors up to 60 seconds
      });

      this.isInitialized = true;
      console.log('MTProto client initialized successfully');
    } catch (error) {
      console.error('Failed to initialize MTProto client:', error);
      throw error;
    }
  }

  /**
   * Connect to Telegram servers
   */
  async connect(): Promise<void> {
    if (!this.client) {
      throw new Error('MTProto client not initialized. Call initialize() first.');
    }

    if (this.isConnected) {
      console.log('Already connected to Telegram');
      return;
    }

    try {
      await this.client.connect();
      this.isConnected = true;
      console.log('Connected to Telegram successfully');

      // Save session string if it changed
      const newSessionString = this.client.session.save() as string | undefined;
      if (newSessionString && newSessionString !== this.sessionString) {
        this.sessionString = newSessionString;
        this.saveSession(newSessionString);
        console.log('Session updated and saved');
      }
    } catch (error) {
      console.error('Failed to connect to Telegram:', error);
      throw error;
    }
  }

  /**
   * Start authentication/login process
   */
  async start(options: LoginOptions): Promise<void> {
    if (!this.client) {
      throw new Error('MTProto client not initialized. Call initialize() first.');
    }

    try {
      await this.client.start(options);

      // Save session after successful login
      const newSessionString = this.client.session.save() as string | undefined;
      if (newSessionString && newSessionString !== this.sessionString) {
        this.sessionString = newSessionString;
        this.saveSession(newSessionString);
        console.log('Authentication successful, session saved');
      }
    } catch (error) {
      console.error('Authentication failed:', error);
      throw error;
    }
  }

  /**
   * Sign in using QR code
   */
  async signInWithQrCode(
    qrCodeCallback: (qrCode: { token: Buffer; expires: number }) => void,
    passwordCallback?: (hint?: string) => Promise<string>,
    onError?: (err: Error) => Promise<boolean> | void
  ): Promise<unknown> {
    if (!this.client) {
      throw new Error('MTProto client not initialized. Call initialize() first.');
    }

    try {
      const user = await this.client.signInUserWithQrCode(
        {
          apiId: this.apiId,
          apiHash: this.apiHash,
        },
        {
          qrCode: async (qrCode) => {
            qrCodeCallback(qrCode);
          },
          password: passwordCallback,
          onError: async (err: Error): Promise<boolean> => {
            // If password callback is provided and we get SESSION_PASSWORD_NEEDED,
            // the password callback should handle it, but if onError is called first,
            // we need to let it through
            const errorMessage = err.message || '';
            if (errorMessage.includes('SESSION_PASSWORD_NEEDED') && passwordCallback) {
              // Don't stop - let the password callback handle it
              if (onError) {
                const result = await onError(err);
                return result ?? false;
              }
              return false;
            }
            
            if (onError) {
              const result = await onError(err);
              return result ?? false;
            }
            console.error('QR code auth error:', err);
            return false;
          },
        }
      );

      // Save session after successful login
      const newSessionString = this.client.session.save() as string | undefined;
      if (newSessionString && newSessionString !== this.sessionString) {
        this.sessionString = newSessionString;
        this.saveSession(newSessionString);
        console.log('QR code authentication successful, session saved');
      }

      return user;
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      
      // If it's a password needed error, the password callback should have been invoked
      // by the library. If we're here, it means either:
      // 1. The password callback wasn't provided
      // 2. The password callback failed
      // 3. The library threw before invoking the callback
      // In any case, we should let the caller handle it
      if (errorMessage.includes('SESSION_PASSWORD_NEEDED')) {
        console.log('SESSION_PASSWORD_NEEDED - password callback should handle this');
        // Don't log as error - this is expected when 2FA is enabled
        // The password callback should be invoked by the library
      } else {
        console.error('QR code authentication failed:', error);
      }
      throw error;
    }
  }

  /**
   * Get the Telegram client instance
   * @throws Error if client is not initialized
   */
  getClient(): TelegramClient {
    if (!this.client || !this.isInitialized) {
      throw new Error('MTProto client not initialized. Call initialize() first.');
    }
    return this.client;
  }

  /**
   * Check if the client is initialized
   */
  isReady(): boolean {
    return this.isInitialized && this.client !== undefined;
  }

  /**
   * Check if the client is connected
   */
  isClientConnected(): boolean {
    return this.isConnected && this.isReady();
  }

  /**
   * Get the current session string
   */
  getSessionString(): string {
    return this.sessionString;
  }

  /**
   * Check connection status and update user online status
   * This calls getMe() which also updates the user's online status on Telegram
   * Automatically initializes and connects if needed
   */
  async checkConnection(): Promise<boolean> {
    try {
      // Initialize if not already initialized
      if (!this.isInitialized || !this.client) {
        await this.initialize();
      }

      // Connect if not already connected
      if (!this.isConnected) {
        await this.connect();
      }

      // Check authorization
      const isAuthorized = await this.client!.checkAuthorization();
      if (!isAuthorized) {
        return false;
      }

      // Call getMe() to check connection and update online status with FLOOD_WAIT handling
      await this.handleFloodWait(async () => {
        await this.client!.getMe();
      });
      return true;
    } catch (error) {
      // Don't log FLOOD_WAIT as a warning - it's expected behavior
      if (error instanceof FloodWaitError) {
        console.debug(`Telegram connection check: FLOOD_WAIT ${error.seconds}s`);
      } else {
        console.warn('Telegram connection check failed:', error);
      }
      return false;
    }
  }

  /**
   * Disconnect from Telegram
   */
  async disconnect(): Promise<void> {
    if (this.client && this.isConnected) {
      try {
        await this.client.disconnect();
        this.isConnected = false;
        console.log('Disconnected from Telegram');
      } catch (error) {
        console.error('Error disconnecting from Telegram:', error);
        throw error;
      }
    }
  }

  /**
   * Send a message using the client with FLOOD_WAIT handling
   */
  async sendMessage(entity: string, message: string): Promise<void> {
    const client = this.getClient();
    if (!this.isClientConnected()) {
      await this.connect();
    }
    
    return this.handleFloodWait(async () => {
      await client.sendMessage(entity, { message });
    });
  }

  /**
   * Handle FLOOD_WAIT errors by waiting and retrying
   * @param operation The async operation to execute
   * @param maxRetries Maximum number of retry attempts (default: 3)
   * @param retryCount Current retry count (internal use)
   * @returns The result of the operation
   */
  private async handleFloodWait<T>(
    operation: () => Promise<T>,
    maxRetries = 3,
    retryCount = 0
  ): Promise<T> {
    try {
      return await operation();
    } catch (error) {
      // Check if it's a FLOOD_WAIT error
      if (error instanceof FloodWaitError) {
        const waitSeconds = error.seconds;
        
        // If wait time is too long (more than 5 minutes), throw error
        if (waitSeconds > 300) {
          throw new Error(`FLOOD_WAIT: Too long wait time (${waitSeconds}s). Please try again later.`);
        }

        // If we've exceeded max retries, throw error
        if (retryCount >= maxRetries) {
          throw new Error(`FLOOD_WAIT: Maximum retries exceeded. Wait ${waitSeconds}s before trying again.`);
        }

        console.warn(`FLOOD_WAIT: Waiting ${waitSeconds} seconds before retry (attempt ${retryCount + 1}/${maxRetries})`);
        
        // Wait for the specified time (convert to milliseconds)
        await new Promise((resolve) => setTimeout(resolve, waitSeconds * 1000));
        
        // Retry the operation
        return this.handleFloodWait(operation, maxRetries, retryCount + 1);
      }
      
      // If it's not a FLOOD_WAIT error, rethrow it
      throw error;
    }
  }

  /**
   * Execute an operation with FLOOD_WAIT error handling
   * This is a public utility method that can be used to wrap any Telegram API call
   * @param operation The async operation to execute
   * @param maxRetries Maximum number of retry attempts (default: 3)
   * @returns The result of the operation
   */
  async withFloodWaitHandling<T>(
    operation: () => Promise<T>,
    maxRetries = 3
  ): Promise<T> {
    return this.handleFloodWait(operation, maxRetries);
  }

  /**
   * Invoke a raw Telegram API method with FLOOD_WAIT handling
   */
  async invoke<T = unknown>(request: Parameters<TelegramClient['invoke']>[0]): Promise<T> {
    const client = this.getClient();
    if (!this.isClientConnected()) {
      await this.connect();
    }
    
    return this.handleFloodWait(async () => {
      return client.invoke(request) as Promise<T>;
    });
  }

  /**
   * Load session from localStorage
   */
  private loadSession(): string | null {
    try {
      return localStorage.getItem('telegram_session');
    } catch (error) {
      console.error('Failed to load session from localStorage:', error);
      return null;
    }
  }

  /**
   * Save session to localStorage
   */
  private saveSession(session: string): void {
    try {
      localStorage.setItem('telegram_session', session);
    } catch (error) {
      console.error('Failed to save session to localStorage:', error);
    }
  }
}

// Export singleton instance
export const mtprotoService = MTProtoService.getInstance();
export default mtprotoService;
