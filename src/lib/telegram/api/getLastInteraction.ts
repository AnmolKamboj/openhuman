/**
 * API: Get last interaction — cache-only.
 */

import type { ApiResult } from "./types";
import { getChatById, getCachedMessages, formatMessage } from "./helpers";

export interface LastInteraction {
  chatTitle: string;
  from: string;
  date: string;
  text: string;
}

export async function getLastInteraction(
  chatId: string | number,
): Promise<ApiResult<LastInteraction | undefined>> {
  const chat = getChatById(chatId);
  if (!chat) return { data: undefined, fromCache: true };

  const messages = getCachedMessages(chatId, 1, 0);
  if (!messages || messages.length === 0) {
    return { data: undefined, fromCache: true };
  }

  const msg = messages[0];
  const f = formatMessage(msg);

  return {
    data: {
      chatTitle: chat.title ?? String(chatId),
      from: msg.fromName ?? msg.fromId ?? "Unknown",
      date: f.date,
      text: f.text || "[Media/No text]",
    },
    fromCache: true,
  };
}
