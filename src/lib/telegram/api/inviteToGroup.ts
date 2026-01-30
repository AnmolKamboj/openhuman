import { mtprotoService } from "../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import bigInt from "big-integer";
import { toInputChannel, toInputUser } from "./apiCastHelpers";
import { getChatById } from "./helpers";

export async function inviteToGroup(
  chatId: string | number,
  userIds: string[]
): Promise<ApiResult<void>> {
  if (userIds.length === 0) {
    throw new Error("user_ids must not be empty");
  }

  const chat = getChatById(chatId);
  if (!chat) {
    throw new Error(`Chat not found: ${chatId}`);
  }

  const client = mtprotoService.getClient();
  const entity = chat.username ? chat.username : chat.id;

  const users: Api.TypeInputUser[] = [];
  for (const uid of userIds) {
    const inputUser = await client.getInputEntity(String(uid));
    users.push(toInputUser(inputUser));
  }

  const inputPeer = await client.getInputEntity(entity);

  if (chat.type === "channel" || chat.type === "supergroup") {
    await mtprotoService.withFloodWaitHandling(async () => {
      await client.invoke(
        new Api.channels.InviteToChannel({
          channel: toInputChannel(inputPeer),
          users,
        })
      );
    });
  } else {
    for (const user of users) {
      await mtprotoService.withFloodWaitHandling(async () => {
        await client.invoke(
          new Api.messages.AddChatUser({
            chatId: bigInt(chat.id),
            userId: user,
            fwdLimit: 100,
          })
        );
      });
    }
  }

  return { data: undefined, fromCache: false };
}
