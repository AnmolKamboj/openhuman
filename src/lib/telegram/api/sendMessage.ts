/**
 * API: Send message — always API (write operation).
 */

import { mtprotoService } from "../services/mtprotoService";
import { getChatById } from "./helpers";
import type { ApiResult } from "./types";

export async function sendMessage(
  chatId: string | number,
  message: string,
  replyToMessageId?: number
): Promise<ApiResult<{ id: string } | undefined>> {
  const chat = getChatById(chatId);
  if (!chat) return { data: undefined, fromCache: false };

  const entity = chat.username ? `@${chat.username.replace("@", "")}` : chat.id;

  if (replyToMessageId !== undefined) {
    const client = mtprotoService.getClient();
    await mtprotoService.withFloodWaitHandling(async () => {
      await client.sendMessage(entity, {
        message,
        replyTo: replyToMessageId,
      });
    });
  } else {
    await mtprotoService.sendMessage(entity, message);
  }

  return { data: { id: String(Date.now()) }, fromCache: false };
}
