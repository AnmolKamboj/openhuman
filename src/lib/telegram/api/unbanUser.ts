import { mtprotoService } from "../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import { toInputChannel, toInputPeer } from "./apiCastHelpers";
import { getChatById } from "./helpers";

export async function unbanUser(
  chatId: string | number,
  userId: string | number
): Promise<ApiResult<void>> {
  const chat = getChatById(chatId);
  if (!chat) {
    throw new Error(`Chat not found: ${chatId}`);
  }

  if (chat.type !== "channel" && chat.type !== "supergroup") {
    throw new Error("Unban is only available for channels/supergroups.");
  }

  const client = mtprotoService.getClient();
  const entity = chat.username ? chat.username : chat.id;

  await mtprotoService.withFloodWaitHandling(async () => {
    const inputChannel = await client.getInputEntity(entity);
    const inputUser = await client.getInputEntity(userId);
    await client.invoke(
      new Api.channels.EditBanned({
        channel: toInputChannel(inputChannel),
        participant: toInputPeer(inputUser),
        bannedRights: new Api.ChatBannedRights({ untilDate: 0 }),
      })
    );
  });

  return { data: undefined, fromCache: false };
}
