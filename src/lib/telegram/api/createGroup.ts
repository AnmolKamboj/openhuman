import { mtprotoService } from "../../../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import type { ResultWithChats } from "./apiResultTypes";
import { toInputUser, narrow } from "./apiCastHelpers";

export async function createGroup(
  title: string,
  userIds: string[]
): Promise<ApiResult<{ id: string }>> {
  if (userIds.length === 0) {
    throw new Error("user_ids must not be empty");
  }

  const client = mtprotoService.getClient();

  const users: Api.TypeInputUser[] = [];
  for (const uid of userIds) {
    const inputUser = await client.getInputEntity(String(uid));
    users.push(toInputUser(inputUser));
  }

  const result = await mtprotoService.withFloodWaitHandling(async () => {
    return client.invoke(new Api.messages.CreateChat({ title, users }));
  });

  const chatId = narrow<ResultWithChats>(result)?.chats?.[0]?.id ?? "unknown";

  return { data: { id: String(chatId) }, fromCache: false };
}
