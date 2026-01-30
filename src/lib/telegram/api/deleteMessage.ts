/**
 * API: Delete message
 */

import { mtprotoService } from "../../../services/mtprotoService";
import { getChatById } from "./helpers";
import type { ApiResult } from "./types";

export async function deleteMessage(
  chatId: string | number,
  messageId: number
): Promise<ApiResult<void>> {
  const chat = getChatById(chatId);
  if (!chat) {
    throw new Error(`Chat not found: ${chatId}`);
  }

  const entity = chat.username ? `@${chat.username.replace("@", "")}` : chat.id;
  const client = mtprotoService.getClient();

  await mtprotoService.withFloodWaitHandling(async () => {
    await client.deleteMessages(entity, [messageId], { revoke: true });
  });

  return { data: undefined as unknown as void, fromCache: false };
}
