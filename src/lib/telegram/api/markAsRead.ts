/**
 * API: Mark as read
 */

import { mtprotoService } from "../services/mtprotoService";
import { getChatById } from "./helpers";
import type { ApiResult } from "./types";

export async function markAsRead(
  chatId: string | number
): Promise<ApiResult<void>> {
  const chat = getChatById(chatId);
  if (!chat) {
    throw new Error(`Chat not found: ${chatId}`);
  }

  const entity = chat.username ? `@${chat.username.replace("@", "")}` : chat.id;
  const client = mtprotoService.getClient();

  await mtprotoService.withFloodWaitHandling(async () => {
    await client.markAsRead(entity);
  });

  return { data: undefined as unknown as void, fromCache: false };
}
