/**
 * API: Get chats — cache-first with API fallback.
 */

import type { TelegramChat } from "../../../store/telegram/types";
import { mtprotoService } from "../services/mtprotoService";
import { Api } from "telegram";
import bigInt from "big-integer";
import { enforceRateLimit } from "../../mcp/rateLimiter";
import { getOrderedChats, apiDialogToTelegramChat } from "./helpers";
import type { ApiResult } from "./types";
import { updateChatsInState, updateUsersFromApiUsers } from "../state";
import type { ApiUser } from "./apiResultTypes";

export async function getChats(limit = 20): Promise<ApiResult<TelegramChat[]>> {
  // 1. Try cache
  const cached = getOrderedChats(limit);
  if (cached.length > 0) return { data: cached, fromCache: true };

  // 2. Rate limit before API call
  try {
    await enforceRateLimit("__api_fallback_chats");
  } catch {
    return { data: [], fromCache: false };
  }

  // 3. Fetch from Telegram API
  try {
    const client = mtprotoService.getClient();

    const result = await mtprotoService.withFloodWaitHandling(async () => {
      return client.invoke(
        new Api.messages.GetDialogs({
          offsetDate: 0,
          offsetId: 0,
          offsetPeer: new Api.InputPeerEmpty(),
          limit,
          hash: bigInt(0),
        })
      );
    });

    if (!("dialogs" in result) || !Array.isArray(result.dialogs)) {
      return { data: [], fromCache: false };
    }

    // Index chats and users by ID
    const chatsById = new Map<string, Record<string, unknown>>();
    const usersById = new Map<string, Record<string, unknown>>();
    if ("chats" in result && Array.isArray(result.chats)) {
      for (const c of result.chats as unknown as Record<string, unknown>[]) {
        if (c.id != null) chatsById.set(String(c.id), c);
      }
    }
    if ("users" in result && Array.isArray(result.users)) {
      for (const u of result.users as unknown as Record<string, unknown>[]) {
        if (u.id != null) usersById.set(String(u.id), u);
      }
    }

    const data = (result.dialogs as unknown as Record<string, unknown>[])
      .map((d) => apiDialogToTelegramChat(d, chatsById, usersById))
      .filter((c): c is TelegramChat => c !== undefined)
      .slice(0, limit);

    // Update Redux state with fetched chats and users
    updateChatsInState(data);
    if ("users" in result && Array.isArray(result.users)) {
      updateUsersFromApiUsers(result.users as unknown as ApiUser[]);
    }

    return { data, fromCache: false };
  } catch {
    return { data: [], fromCache: false };
  }
}
