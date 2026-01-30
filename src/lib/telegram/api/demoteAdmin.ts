import { mtprotoService } from "../../../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import { toInputChannel, toInputUser } from "./apiCastHelpers";
import { getChatById } from "./helpers";

export async function demoteAdmin(
  chatId: string | number,
  userId: string | number
): Promise<ApiResult<void>> {
  const chat = getChatById(chatId);
  if (!chat) {
    throw new Error(`Chat not found: ${chatId}`);
  }

  if (chat.type !== "channel" && chat.type !== "supergroup") {
    throw new Error(
      "Admin demotion is only available for channels/supergroups."
    );
  }

  const client = mtprotoService.getClient();
  const entity = chat.username ? chat.username : chat.id;

  await mtprotoService.withFloodWaitHandling(async () => {
    const inputChannel = await client.getInputEntity(entity);
    const inputUser = await client.getInputEntity(userId);
    await client.invoke(
      new Api.channels.EditAdmin({
        channel: toInputChannel(inputChannel),
        userId: toInputUser(inputUser),
        adminRights: new Api.ChatAdminRights({}),
        rank: "",
      })
    );
  });

  return { data: undefined, fromCache: false };
}
