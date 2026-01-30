import { mtprotoService } from "../../../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import type { ResultWithChats } from "./apiResultTypes";
import { narrow } from "./apiCastHelpers";

export async function joinChatByLink(
  link: string
): Promise<ApiResult<{ chatTitle: string }>> {
  // Extract hash from link
  let hash = link;
  const plusMatch = link.match(/t\.me\/\+(.+)/);
  const joinMatch = link.match(/t\.me\/joinchat\/(.+)/);
  if (plusMatch) hash = plusMatch[1];
  else if (joinMatch) hash = joinMatch[1];

  const client = mtprotoService.getClient();

  const result = await mtprotoService.withFloodWaitHandling(async () => {
    return client.invoke(new Api.messages.ImportChatInvite({ hash }));
  });

  const chatTitle =
    narrow<ResultWithChats>(result)?.chats?.[0]?.title ?? "unknown";

  return { data: { chatTitle }, fromCache: false };
}
