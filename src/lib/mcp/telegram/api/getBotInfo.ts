import { mtprotoService } from "../../../../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import type { FullUserResult } from "./apiResultTypes";
import { toInputUser, narrow } from "./apiCastHelpers";
import { updateUsersFromApiUsers } from "../state";

export interface BotInfo {
  name: string;
  username: string;
  id: string;
  isBot: boolean;
  about?: string;
  botDescription?: string;
  commands?: Array<{ command: string; description: string }>;
}

export async function getBotInfo(
  botId: string | number,
): Promise<ApiResult<BotInfo>> {
  const client = mtprotoService.getClient();

  const result = await mtprotoService.withFloodWaitHandling(async () => {
    const inputUser = await client.getInputEntity(botId);
    return client.invoke(
      new Api.users.GetFullUser({ id: toInputUser(inputUser) }),
    );
  });

  const fullUser = narrow<FullUserResult>(result)?.fullUser;
  const user = narrow<FullUserResult>(result)?.users?.[0];

  if (!user) {
    throw new Error("Bot not found: " + botId);
  }

  // Update Redux state with fetched bot user
  updateUsersFromApiUsers([user]);

  const name =
    [user.firstName, user.lastName].filter(Boolean).join(" ") || "Unknown";

  return {
    data: {
      name,
      username: user.username ?? "N/A",
      id: String(user.id),
      isBot: user.bot ?? false,
      about: fullUser?.about,
      botDescription: fullUser?.botInfo?.description,
      commands: fullUser?.botInfo?.commands,
    },
    fromCache: false,
  };
}
