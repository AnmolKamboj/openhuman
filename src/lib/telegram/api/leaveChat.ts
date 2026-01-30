import { mtprotoService } from "../services/mtprotoService";
import { getChatById } from "./helpers";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import bigInt from "big-integer";
import { toInputChannel } from "./apiCastHelpers";

export async function leaveChat(
  chatId: string | number
): Promise<ApiResult<void>> {
  const chat = getChatById(chatId);
  if (!chat) throw new Error(`Chat not found: ${chatId}`);

  const client = mtprotoService.getClient();
  const entity = chat.username ? chat.username : chat.id;

  if (chat.type === "channel" || chat.type === "supergroup") {
    await mtprotoService.withFloodWaitHandling(async () => {
      const inputChannel = await client.getInputEntity(entity);
      await client.invoke(
        new Api.channels.LeaveChannel({
          channel: toInputChannel(inputChannel),
        })
      );
    });
  } else {
    await mtprotoService.withFloodWaitHandling(async () => {
      await client.invoke(
        new Api.messages.DeleteChatUser({
          chatId: bigInt(chat.id),
          userId: new Api.InputUserSelf(),
        })
      );
    });
  }

  return { data: undefined as unknown as void, fromCache: false };
}
