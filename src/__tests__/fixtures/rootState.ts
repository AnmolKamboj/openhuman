/**
 * Factory builder for a mock RootState matching the Redux store shape.
 */

import type { TelegramState } from "../../store/telegram/types";
import { initialState as telegramInitialState } from "../../store/telegram/types";

/**
 * Minimal RootState shape matching the real store (without persist wrappers).
 * Only includes the fields tests actually need.
 */
export interface MockRootState {
  auth: {
    token: string | null;
    isOnboardedByUser: Record<string, boolean>;
  };
  socket: {
    byUser: Record<
      string,
      { status: "connected" | "disconnected" | "connecting"; socketId: string | null }
    >;
  };
  user: {
    user: { _id: string; telegramId: number; [key: string]: unknown } | null;
    isLoading: boolean;
    error: string | null;
  };
  telegram: {
    byUser: Record<string, TelegramState>;
  };
}

const DEFAULT_USER_ID = "user_123";

export function createMockRootState(
  telegramOverrides: Partial<TelegramState> = {},
  userId = DEFAULT_USER_ID,
): MockRootState {
  return {
    auth: {
      token: "mock-jwt-token",
      isOnboardedByUser: { [userId]: true },
    },
    socket: {
      byUser: {
        [userId]: { status: "connected", socketId: "sock_1" },
      },
    },
    user: {
      user: {
        _id: userId,
        telegramId: 12345,
      },
      isLoading: false,
      error: null,
    },
    telegram: {
      byUser: {
        [userId]: {
          ...telegramInitialState,
          ...telegramOverrides,
        },
      },
    },
  };
}
