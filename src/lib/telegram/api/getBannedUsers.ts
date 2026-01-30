import { mtprotoService } from "../../../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import bigInt from "big-integer";
import type { ApiUser } from "./apiResultTypes";
import { toInputChannel } from "./apiCastHelpers";
import { getChatById } from "./helpers";
import { updateUsersFromApiUsers } from "../state";

export interface BannedUser {
  id: string;
  name: string;
  username?: string;
}

export async function getBannedUsers(
  chatId: string | number,
  limit: number = 50
): Promise<ApiResult<BannedUser[]>> {
  const chat = getChatById(chatId);
  if (!chat) {
    throw new Error(`Chat not found: ${chatId}`);
  }

  if (chat.type !== "channel" && chat.type !== "supergroup") {
    throw new Error(
      "Banned users list is only available for channels/supergroups."
    );
  }

  const client = mtprotoService.getClient();
  const entity = chat.username ? chat.username : chat.id;

  const result = await mtprotoService.withFloodWaitHandling(async () => {
    const inputChannel = await client.getInputEntity(entity);
    return client.invoke(
      new Api.channels.GetParticipants({
        channel: toInputChannel(inputChannel),
        filter: new Api.ChannelParticipantsKicked({ q: "" }),
        offset: 0,
        limit,
        hash: bigInt(0),
      })
    );
  });

  if (
    !result ||
    !("users" in result) ||
    !Array.isArray(result.users) ||
    result.users.length === 0
  ) {
    return { data: [], fromCache: false };
  }

  // Update Redux state with fetched users
  updateUsersFromApiUsers(result.users as ApiUser[]);

  const data: BannedUser[] = result.users.map((u: ApiUser) => ({
    id: String(u.id),
    name: [u.firstName, u.lastName].filter(Boolean).join(" ") || "Unknown",
    username: u.username,
  }));

  return { data, fromCache: false };
}
