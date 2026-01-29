import { ActionReducerMapBuilder } from "@reduxjs/toolkit";
import type { TelegramRootState, TelegramState } from "./types";
import { initialState } from "./types";
import {
  initializeTelegram,
  connectTelegram,
  checkAuthStatus,
  fetchChats,
  fetchMessages,
} from "./thunks";
import type { TelegramUser } from "./types";

function ensureUser(
  state: TelegramRootState,
  userId: string,
): TelegramState {
  if (!state.byUser[userId]) {
    state.byUser[userId] = { ...initialState };
  }
  return state.byUser[userId];
}

function userIdFromMeta(action: {
  meta: { arg?: string | { userId: string } };
}): string {
  const arg = action.meta?.arg;
  if (typeof arg === "string") return arg;
  if (arg && typeof arg === "object" && typeof arg.userId === "string") {
    return arg.userId;
  }
  throw new Error("telegram thunk requires userId");
}

export const buildExtraReducers = (
  builder: ActionReducerMapBuilder<TelegramRootState>,
) => {
  builder
    .addCase(initializeTelegram.pending, (state, action) => {
      const uid = userIdFromMeta(action);
      const u = ensureUser(state, uid);
      u.isInitialized = false;
    })
    .addCase(initializeTelegram.fulfilled, (state, action) => {
      const uid = userIdFromMeta(action);
      const u = ensureUser(state, uid);
      u.isInitialized = true;
      u.sessionString = action.payload.sessionString;
    })
    .addCase(initializeTelegram.rejected, (state, action) => {
      const uid =
        typeof action.meta?.arg === "string" ? action.meta.arg : undefined;
      if (!uid) return;
      const u = ensureUser(state, uid);
      u.isInitialized = false;
      u.connectionError = (action.payload as string) ?? null;
    });

  builder
    .addCase(connectTelegram.pending, (state, action) => {
      const uid = userIdFromMeta(action);
      const u = ensureUser(state, uid);
      u.connectionStatus = "connecting";
      u.connectionError = null;
    })
    .addCase(connectTelegram.fulfilled, (state, action) => {
      const uid = userIdFromMeta(action);
      const u = ensureUser(state, uid);
      u.connectionStatus = "connected";
      u.connectionError = null;
    })
    .addCase(connectTelegram.rejected, (state, action) => {
      const uid =
        typeof action.meta?.arg === "string" ? action.meta.arg : undefined;
      if (!uid) return;
      const u = ensureUser(state, uid);
      u.connectionStatus = "error";
      u.connectionError = (action.payload as string) ?? null;
    });

  builder
    .addCase(checkAuthStatus.pending, (state, action) => {
      const uid = userIdFromMeta(action);
      const u = ensureUser(state, uid);
      u.authStatus = "authenticating";
    })
    .addCase(checkAuthStatus.fulfilled, (state, action) => {
      const uid = userIdFromMeta(action);
      const u = ensureUser(state, uid);
      if (action.payload) {
        u.authStatus = "authenticated";
        const payload = action.payload as
          | TelegramUser
          | {
              id: number | string;
              firstName?: string;
              lastName?: string;
              username?: string;
              bot?: boolean;
              accessHash?: bigint | string;
            };
        if ("isBot" in payload) {
          u.currentUser = payload;
        } else {
          u.currentUser = {
            id: String(payload.id),
            firstName: payload.firstName || "",
            lastName: payload.lastName,
            username: payload.username,
            isBot: Boolean(payload.bot),
            accessHash: payload.accessHash?.toString(),
          };
        }
      } else {
        u.authStatus = "not_authenticated";
        u.currentUser = null;
      }
    })
    .addCase(checkAuthStatus.rejected, (state, action) => {
      const uid =
        typeof action.meta?.arg === "string" ? action.meta.arg : undefined;
      if (!uid) return;
      const u = ensureUser(state, uid);
      u.authStatus = "error";
      u.authError = (action.payload as string) ?? null;
    });

  builder
    .addCase(fetchChats.pending, (state, action) => {
      const uid = userIdFromMeta(action);
      ensureUser(state, uid).isLoadingChats = true;
    })
    .addCase(fetchChats.fulfilled, (state, action) => {
      const uid = userIdFromMeta(action);
      ensureUser(state, uid).isLoadingChats = false;
    })
    .addCase(fetchChats.rejected, (state, action) => {
      const uid =
        typeof action.meta?.arg === "string" ? action.meta.arg : undefined;
      if (!uid) return;
      ensureUser(state, uid).isLoadingChats = false;
    });

  builder
    .addCase(fetchMessages.pending, (state, action) => {
      const uid = userIdFromMeta(action);
      ensureUser(state, uid).isLoadingMessages = true;
    })
    .addCase(fetchMessages.fulfilled, (state, action) => {
      const uid = userIdFromMeta(action);
      ensureUser(state, uid).isLoadingMessages = false;
    })
    .addCase(fetchMessages.rejected, (state, action) => {
      const arg = action.meta?.arg;
      const uid =
        typeof arg === "string"
          ? arg
          : arg && typeof arg === "object" && typeof arg.userId === "string"
            ? arg.userId
            : undefined;
      if (!uid) return;
      ensureUser(state, uid).isLoadingMessages = false;
    });
};
