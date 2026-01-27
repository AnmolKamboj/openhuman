// API Response wrapper
export interface ApiResponse<T> {
  success: boolean;
  data: T;
  error?: string;
  message?: string;
}

// API Error response
export interface ApiError {
  success: false;
  error: string;
  message?: string;
}

// User types based on backend ITgUser model
export interface UserSubscription {
  hasActiveSubscription: boolean;
  plan: 'FREE' | 'BASIC' | 'PRO';
  planExpiry?: string;
  stripeCustomerId?: string;
}

export interface UserUsage {
  dailyTokenLimit: number;
  remainingTokens: number;
  activeSessionCount: number;
  lastTokenResetAt?: string;
}

export interface UserReferral {
  inviteCode?: string | null;
  inviteCodeUsages: number;
  maxInviteCodeUsages?: number | null;
  inviteCodeUsedAt?: string;
  invitedBy?: string | null;
  pendingInviteCode?: string | null;
}

export interface UserSettings {
  dailySummariesEnabled: boolean;
  dailySummaryUtcTriggerHour?: number;
  dailySummaryChatIds: number[];
  autoCompleteEnabled: boolean;
  autoCompleteVisibility: 'always' | 'groups_only' | 'private_chats_only';
  autoCompleteWhitelistChatIds: number[];
  autoCompleteBlacklistChatIds: number[];
}

export interface User {
  id: string;
  telegramId: number;
  hasAccess: boolean;
  magicWord: string;
  referral: UserReferral;
  subscription: UserSubscription;
  usage: UserUsage;
  role: 'admin' | 'team' | 'user';
  settings: UserSettings;
  autoDeleteTelegramMessagesAfterDays: number;
  autoDeleteThreadsAfterDays: number;
  firstName?: string;
  lastName?: string;
  username?: string;
  languageCode?: string;
  waitlist?: string;
}

// API Endpoints
export type GetMeResponse = ApiResponse<User>;
