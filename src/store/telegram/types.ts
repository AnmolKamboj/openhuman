// Types for Telegram entities
export type TelegramConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'error';
export type TelegramAuthStatus = 'not_authenticated' | 'authenticating' | 'authenticated' | 'error';

export interface TelegramUser {
  id: string;
  firstName: string;
  lastName?: string;
  username?: string;
  phoneNumber?: string;
  isBot: boolean;
  isVerified?: boolean;
  isPremium?: boolean;
  accessHash?: string;
}

export interface TelegramChat {
  id: string;
  title?: string;
  type: 'private' | 'group' | 'supergroup' | 'channel';
  username?: string;
  accessHash?: string;
  unreadCount: number;
  lastMessage?: TelegramMessage;
  lastMessageDate?: number;
  isPinned: boolean;
  photo?: {
    smallFileId?: string;
    bigFileId?: string;
  };
  participantsCount?: number;
}

export interface TelegramMessage {
  id: string;
  chatId: string;
  threadId?: string;
  date: number;
  message: string;
  fromId?: string;
  fromName?: string;
  isOutgoing: boolean;
  isEdited: boolean;
  isForwarded: boolean;
  replyToMessageId?: string;
  media?: {
    type: string;
    [key: string]: unknown;
  };
  reactions?: Array<{
    emoticon: string;
    count: number;
  }>;
  views?: number;
}

export interface TelegramThread {
  id: string;
  chatId: string;
  title: string;
  messageCount: number;
  lastMessage?: TelegramMessage;
  lastMessageDate?: number;
  unreadCount: number;
  isPinned: boolean;
}

export interface TelegramState {
  // Connection state
  connectionStatus: TelegramConnectionStatus;
  connectionError: string | null;

  // Authentication state
  authStatus: TelegramAuthStatus;
  authError: string | null;
  isInitialized: boolean;
  phoneNumber: string | null;
  sessionString: string | null;

  // User data
  currentUser: TelegramUser | null;

  // Chats
  chats: Record<string, TelegramChat>;
  chatsOrder: string[]; // Ordered list of chat IDs
  selectedChatId: string | null;

  // Messages (organized by chatId)
  messages: Record<string, Record<string, TelegramMessage>>; // [chatId][messageId] = message
  messagesOrder: Record<string, string[]>; // [chatId] = [messageId, ...]

  // Threads (organized by chatId)
  threads: Record<string, Record<string, TelegramThread>>; // [chatId][threadId] = thread
  threadsOrder: Record<string, string[]>; // [chatId] = [threadId, ...]
  selectedThreadId: string | null;

  // Loading states
  isLoadingChats: boolean;
  isLoadingMessages: boolean;
  isLoadingThreads: boolean;

  // Pagination
  hasMoreChats: boolean;
  hasMoreMessages: Record<string, boolean>; // [chatId] = hasMore
  hasMoreThreads: Record<string, boolean>; // [chatId] = hasMore

  // Filters and search
  searchQuery: string | null;
  filteredChatIds: string[] | null;
}

export const initialState: TelegramState = {
  // Connection
  connectionStatus: 'disconnected',
  connectionError: null,

  // Authentication
  authStatus: 'not_authenticated',
  authError: null,
  isInitialized: false,
  phoneNumber: null,
  sessionString: null,

  // User
  currentUser: null,

  // Chats
  chats: {},
  chatsOrder: [],
  selectedChatId: null,

  // Messages
  messages: {},
  messagesOrder: {},

  // Threads
  threads: {},
  threadsOrder: {},
  selectedThreadId: null,

  // Loading
  isLoadingChats: false,
  isLoadingMessages: false,
  isLoadingThreads: false,

  // Pagination
  hasMoreChats: true,
  hasMoreMessages: {},
  hasMoreThreads: {},

  // Search
  searchQuery: null,
  filteredChatIds: null,
};
