import { mtprotoService } from "../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import type { ResultWithChats } from "./apiResultTypes";
import { narrow } from "./apiCastHelpers";

export async function importChatInvite(
  hash: string
): Promise<ApiResult<{ chatTitle: string }>> {
  const client = mtprotoService.getClient();

  const result = await mtprotoService.withFloodWaitHandling(async () => {
    return client.invoke(new Api.messages.ImportChatInvite({ hash }));
  });

  const chatTitle =
    narrow<ResultWithChats>(result)?.chats?.[0]?.title ?? "unknown";

  return { data: { chatTitle }, fromCache: false };
}
