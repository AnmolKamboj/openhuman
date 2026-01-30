/**
 * API: Send reaction
 */

import { mtprotoService } from "../services/mtprotoService";
import { getChatById } from "./helpers";
import type { ApiResult } from "./types";
import { Api } from "telegram";

export async function sendReaction(
  chatId: string | number,
  messageId: number,
  emoji: string
): Promise<ApiResult<void>> {
  const chat = getChatById(chatId);
  if (!chat) {
    throw new Error(`Chat not found: ${chatId}`);
  }

  const client = mtprotoService.getClient();
  const entity = chat.username ? `@${chat.username.replace("@", "")}` : chat.id;

  await mtprotoService.withFloodWaitHandling(async () => {
    const inputPeer = await client.getInputEntity(entity);
    await client.invoke(
      new Api.messages.SendReaction({
        peer: inputPeer,
        msgId: messageId,
        reaction: [new Api.ReactionEmoji({ emoticon: emoji })],
      })
    );
  });

  return { data: undefined as unknown as void, fromCache: false };
}
