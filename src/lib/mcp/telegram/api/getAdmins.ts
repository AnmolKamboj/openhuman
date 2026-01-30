import { mtprotoService } from "../../../../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import bigInt from "big-integer";
import type { ApiUser } from "./apiResultTypes";
import { toInputChannel, narrow } from "./apiCastHelpers";
import { getChatById } from "./helpers";
import { updateUsersFromApiUsers } from "../state";

export interface Admin {
  id: string;
  name: string;
  username?: string;
}

export async function getAdmins(chatId: string | number): Promise<ApiResult<Admin[]>> {
  const chat = getChatById(chatId);
  if (!chat) {
    throw new Error(`Chat not found: ${chatId}`);
  }

  const client = mtprotoService.getClient();
  const entity = chat.username ? chat.username : chat.id;

  let admins: ApiUser[] = [];

  if (chat.type === "channel" || chat.type === "supergroup") {
    const result = await mtprotoService.withFloodWaitHandling(async () => {
      const inputChannel = await client.getInputEntity(entity);
      return client.invoke(
        new Api.channels.GetParticipants({
          channel: toInputChannel(inputChannel),
          filter: new Api.ChannelParticipantsAdmins(),
          offset: 0,
          limit: 100,
          hash: bigInt(0),
        }),
      );
    });
    if (result && "users" in result && Array.isArray(result.users)) {
      admins = narrow<ApiUser[]>(result.users);
    }
  } else {
    const result = await mtprotoService.withFloodWaitHandling(async () => {
      return client.invoke(
        new Api.messages.GetFullChat({ chatId: bigInt(chat.id) }),
      );
    });
    if (result && "users" in result && Array.isArray(result.users)) {
      admins = narrow<ApiUser[]>(result.users);
    }
  }

  // Update Redux state with fetched users
  updateUsersFromApiUsers(admins);

  const data: Admin[] = admins.map((u: ApiUser) => ({
    id: String(u.id),
    name: [u.firstName, u.lastName].filter(Boolean).join(" ") || "Unknown",
    username: u.username,
  }));

  return { data, fromCache: false };
}
