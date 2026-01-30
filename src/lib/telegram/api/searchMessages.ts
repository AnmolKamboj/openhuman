/**
 * API: Search messages (API-first with cache fallback)
 */

import { mtprotoService } from "../services/mtprotoService";
import { getChatById, formatMessage, getCachedMessages } from "./helpers";
import type { ApiResult } from "./types";
import type { ApiMessage } from "./apiResultTypes";
import { Api } from "telegram";
import bigInt from "big-integer";
import { narrow } from "./apiCastHelpers";

export interface SearchedMessage {
  id: number | string;
  date: string;
  text: string;
  from?: string;
}

export async function searchMessages(
  chatId: string | number,
  query: string,
  limit: number
): Promise<ApiResult<SearchedMessage[]>> {
  const chat = getChatById(chatId);
  if (!chat) {
    throw new Error(`Chat not found: ${chatId}`);
  }

  // Try API first
  try {
    const client = mtprotoService.getClient();
    const entity = chat.username
      ? `@${chat.username.replace("@", "")}`
      : chat.id;
    const inputPeer = await client.getInputEntity(entity);

    const result = await mtprotoService.withFloodWaitHandling(async () => {
      return client.invoke(
        new Api.messages.Search({
          peer: inputPeer,
          q: query,
          filter: new Api.InputMessagesFilterEmpty(),
          minDate: 0,
          maxDate: 0,
          offsetId: 0,
          addOffset: 0,
          limit,
          maxId: 0,
          minId: 0,
          hash: bigInt(0),
        })
      );
    });

    if ("messages" in result && Array.isArray(result.messages)) {
      const messages = narrow<ApiMessage[]>(result.messages);
      const searchedMessages = messages.map((msg) => ({
        id: msg.id ?? "?",
        date: msg.date ? new Date(msg.date * 1000).toISOString() : "unknown",
        text: msg.message ?? "[Media/No text]",
      }));
      return { data: searchedMessages, fromCache: false };
    }
  } catch {
    // API failed, fall back to cache below
  }

  // Fallback: search cached messages
  const cachedMessages = getCachedMessages(chatId, limit * 3, 0);

  if (cachedMessages) {
    const q = query.toLowerCase();
    const filtered = cachedMessages
      .filter((msg) => msg.message.toLowerCase().includes(q))
      .slice(0, limit);

    const searchedMessages = filtered.map((msg) => {
      const f = formatMessage(msg);
      return {
        id: f.id,
        date: f.date,
        text: f.text || "[Media]",
        from: msg.fromName ?? msg.fromId ?? "Unknown",
      };
    });
    return { data: searchedMessages, fromCache: true };
  }

  return { data: [], fromCache: true };
}
