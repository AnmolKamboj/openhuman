/**
 * API: Unpin message
 */

import { mtprotoService } from "../services/mtprotoService";
import { getChatById } from "./helpers";
import type { ApiResult } from "./types";
import { Api } from "telegram";

export async function unpinMessage(
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
    const inputPeer = await client.getInputEntity(entity);
    await client.invoke(
      new Api.messages.UpdatePinnedMessage({
        peer: inputPeer,
        id: messageId,
        unpin: true,
      })
    );
  });

  return { data: undefined as unknown as void, fromCache: false };
}
