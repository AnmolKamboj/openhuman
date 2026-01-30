/**
 * Get Media Info API - Get media info from a cached message
 *
 * Reads from cached messages only, does not make API calls.
 */

import { getChatById, getCachedMessages } from "./helpers";
import type { ApiResult } from "./types";

export interface MediaInfo {
  type: string;
  [key: string]: unknown;
}

export async function getMediaInfo(
  chatId: string | number,
  messageId: number,
): Promise<ApiResult<MediaInfo>> {
  const chat = getChatById(chatId);
  if (!chat) {
    throw new Error("Chat not found: " + chatId);
  }

  const messages = getCachedMessages(chatId, 200, 0);
  if (!messages || messages.length === 0) {
    throw new Error("No messages found in cache.");
  }

  const msg = messages.find((m) => String(m.id) === String(messageId));
  if (!msg) {
    throw new Error("Message " + messageId + " not found in cache.");
  }

  if (!msg.media) {
    return {
      data: { type: "none" },
      fromCache: true,
    };
  }

  const info: MediaInfo = {
    ...msg.media,
    type: msg.media.type ?? "unknown",
  };

  return { data: info, fromCache: true };
}
