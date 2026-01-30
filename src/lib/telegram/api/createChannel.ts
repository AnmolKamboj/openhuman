import { mtprotoService } from "../../../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import type { ResultWithChats } from "./apiResultTypes";
import { narrow } from "./apiCastHelpers";

export async function createChannel(
  title: string,
  about?: string,
  megagroup?: boolean
): Promise<ApiResult<{ id: string; type: string }>> {
  const client = mtprotoService.getClient();

  const result = await mtprotoService.withFloodWaitHandling(async () => {
    return client.invoke(
      new Api.channels.CreateChannel({
        title,
        about: about ?? "",
        megagroup: megagroup ?? false,
        broadcast: !megagroup,
      })
    );
  });

  const channelId =
    narrow<ResultWithChats>(result)?.chats?.[0]?.id ?? "unknown";
  const type = megagroup ? "Supergroup" : "Channel";

  return { data: { id: String(channelId), type }, fromCache: false };
}
