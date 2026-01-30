/**
 * API: Get messages — cache-first with API fallback.
 */

import type { TelegramMessage } from "../../../store/telegram/types";
import { mtprotoService } from "../../../services/mtprotoService";
import { Api } from "telegram";
import bigInt from "big-integer";
import { enforceRateLimit } from "../../mcp/rateLimiter";
import {
  getChatById,
  getCachedMessages,
  apiMessageToTelegramMessage,
} from "./helpers";
import type { ApiResult } from "./types";
import { updateMessagesInState, updateUsersFromApiUsers } from "../state";
import type { ApiUser } from "./apiResultTypes";

export async function getMessages(
  chatId: string | number,
  limit = 20,
  offset = 0
): Promise<ApiResult<TelegramMessage[]>> {
  // 1. Try cache
  const cached = getCachedMessages(chatId, limit, offset);
  if (cached && cached.length > 0) {
    return { data: cached, fromCache: true };
  }

  // 2. Resolve chat for API call
  const chat = getChatById(chatId);
  if (!chat) return { data: [], fromCache: false };

  // 3. Rate limit before API call
  try {
    await enforceRateLimit("__api_fallback_messages");
  } catch {
    return { data: [], fromCache: false };
  }

  // 4. Fetch from Telegram API
  try {
    const client = mtprotoService.getClient();
    const entity = chat.username ? chat.username : chat.id;
    const inputPeer = await client.getInputEntity(entity);

    const result = await mtprotoService.withFloodWaitHandling(async () => {
      return client.invoke(
        new Api.messages.GetHistory({
          peer: inputPeer,
          offsetId: 0,
          offsetDate: 0,
          addOffset: offset,
          limit,
          maxId: 0,
          minId: 0,
          hash: bigInt(0),
        })
      );
    });

    if ("messages" in result && Array.isArray(result.messages)) {
      const messages = (result.messages as unknown as Record<string, unknown>[])
        .filter((m) => m.className !== "MessageEmpty")
        .map((m) => apiMessageToTelegramMessage(m, chat.id));

      // Enrich fromName from users list
      if ("users" in result && Array.isArray(result.users)) {
        const usersById = new Map<string, string>();
        for (const u of result.users as Array<{
          id?: unknown;
          firstName?: string;
          lastName?: string;
        }>) {
          if (u.id != null) {
            const name =
              [u.firstName, u.lastName].filter(Boolean).join(" ") || "Unknown";
            usersById.set(String(u.id), name);
          }
        }
        for (const msg of messages) {
          if (msg.fromId && !msg.fromName) {
            msg.fromName = usersById.get(msg.fromId);
          }
        }
      }

      // Update Redux state with fetched messages and users
      if (messages.length > 0) {
        updateMessagesInState(chat.id, messages);
      }
      if ("users" in result && Array.isArray(result.users)) {
        updateUsersFromApiUsers(result.users as unknown as ApiUser[]);
      }

      return {
        data: messages.length > 0 ? messages : [],
        fromCache: false,
      };
    }
  } catch {
    // API failed — return empty
  }

  return { data: [], fromCache: false };
}
