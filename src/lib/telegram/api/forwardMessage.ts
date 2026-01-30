/**
 * API: Forward message
 */

import { mtprotoService } from "../../../services/mtprotoService";
import { getChatById } from "./helpers";
import type { ApiResult } from "./types";

export async function forwardMessage(
  fromChatId: string | number,
  toChatId: string | number,
  messageId: number
): Promise<ApiResult<void>> {
  const fromChat = getChatById(fromChatId);
  if (!fromChat) {
    throw new Error(`Source chat not found: ${fromChatId}`);
  }

  const toChat = getChatById(toChatId);
  if (!toChat) {
    throw new Error(`Target chat not found: ${toChatId}`);
  }

  const fromEntity = fromChat.username
    ? `@${fromChat.username.replace("@", "")}`
    : fromChat.id;
  const toEntity = toChat.username
    ? `@${toChat.username.replace("@", "")}`
    : toChat.id;
  const client = mtprotoService.getClient();

  await mtprotoService.withFloodWaitHandling(async () => {
    await client.forwardMessages(toEntity, {
      messages: [messageId],
      fromPeer: fromEntity,
    });
  });

  return { data: undefined as unknown as void, fromCache: false };
}
