import { mtprotoService } from "../../../services/mtprotoService";
import { getChatById } from "./helpers";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import bigInt from "big-integer";
import { toInputChannel } from "./apiCastHelpers";

export async function deleteChatPhoto(
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
        new Api.channels.EditPhoto({
          channel: toInputChannel(inputChannel),
          photo: new Api.InputChatPhotoEmpty(),
        })
      );
    });
  } else {
    await mtprotoService.withFloodWaitHandling(async () => {
      await client.invoke(
        new Api.messages.EditChatPhoto({
          chatId: bigInt(chat.id),
          photo: new Api.InputChatPhotoEmpty(),
        })
      );
    });
  }

  return { data: undefined as unknown as void, fromCache: false };
}
