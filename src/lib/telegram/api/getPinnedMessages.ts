/**
 * API: Get pinned messages (API-first with cache fallback)
 */

import { mtprotoService } from "../../../services/mtprotoService";
import { getChatById } from "./helpers";
import type { ApiResult } from "./types";
import type { ApiMessage } from "./apiResultTypes";
import { Api } from "telegram";
import bigInt from "big-integer";
import { narrow } from "./apiCastHelpers";

export interface PinnedMessage {
  id: number | string;
  date: string;
  text: string;
}

export async function getPinnedMessages(
  chatId: string | number
): Promise<ApiResult<PinnedMessage[]>> {
  const chat = getChatById(chatId);
  if (!chat) {
    throw new Error(`Chat not found: ${chatId}`);
  }

  const entity = chat.username ? `@${chat.username.replace("@", "")}` : chat.id;
  const client = mtprotoService.getClient();

  // Try API first
  try {
    const inputPeer = await client.getInputEntity(entity);
    const result = await client.invoke(
      new Api.messages.Search({
        peer: inputPeer,
        q: "",
        filter: new Api.InputMessagesFilterPinned(),
        minDate: 0,
        maxDate: 0,
        offsetId: 0,
        addOffset: 0,
        limit: 50,
        maxId: 0,
        minId: 0,
        hash: bigInt(0),
      })
    );

    if ("messages" in result && Array.isArray(result.messages)) {
      const pinnedMessages = narrow<ApiMessage[]>(result.messages).map(
        (msg) => ({
          id: msg.id ?? "?",
          date: msg.date ? new Date(msg.date * 1000).toISOString() : "unknown",
          text: msg.message ?? "[Media/No text]",
        })
      );
      return { data: pinnedMessages, fromCache: false };
    }
  } catch {
    // API failed, fall back to cache below
  }

  // Fallback: pinned status is not stored in Redux cache (TelegramMessage),
  // so we can't provide pinned messages from cache. Return empty result.
  return { data: [], fromCache: true };
}
