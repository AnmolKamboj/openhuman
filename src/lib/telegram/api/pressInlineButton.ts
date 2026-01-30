/**
 * API: Press inline button
 */

import { mtprotoService } from "../services/mtprotoService";
import { getChatById } from "./helpers";
import type { ApiResult } from "./types";
import type { BotCallbackAnswer } from "./apiResultTypes";
import { Api } from "telegram";
import { narrow } from "./apiCastHelpers";

export async function pressInlineButton(
  chatId: string | number,
  messageId: number,
  buttonData: string
): Promise<ApiResult<string>> {
  const chat = getChatById(chatId);
  if (!chat) {
    throw new Error(`Chat not found: ${chatId}`);
  }

  const client = mtprotoService.getClient();
  const entity = chat.username ? `@${chat.username.replace("@", "")}` : chat.id;

  const result = await mtprotoService.withFloodWaitHandling(async () => {
    const inputPeer = await client.getInputEntity(entity);
    return client.invoke(
      new Api.messages.GetBotCallbackAnswer({
        peer: inputPeer,
        msgId: messageId,
        data: Buffer.from(buttonData, "base64"),
      })
    );
  });

  const answer =
    narrow<BotCallbackAnswer>(result)?.message ??
    "Button pressed (no response message).";

  return { data: answer, fromCache: false };
}
