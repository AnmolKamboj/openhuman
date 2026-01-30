import { mtprotoService } from "../services/mtprotoService";
import { getChatById } from "./helpers";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import type { ChatInviteResult } from "./apiResultTypes";
import { narrow } from "./apiCastHelpers";

export async function exportChatInvite(
  chatId: string | number,
  title?: string,
  expireDate?: number,
  usageLimit?: number
): Promise<ApiResult<{ link: string }>> {
  const chat = getChatById(chatId);
  if (!chat) throw new Error(`Chat not found: ${chatId}`);

  const client = mtprotoService.getClient();
  const entity = chat.username ? chat.username : chat.id;

  const result = await mtprotoService.withFloodWaitHandling(async () => {
    const inputPeer = await client.getInputEntity(entity);
    return client.invoke(
      new Api.messages.ExportChatInvite({
        peer: inputPeer,
        title: title ?? undefined,
        expireDate: expireDate ?? undefined,
        usageLimit: usageLimit ?? undefined,
      })
    );
  });

  const link = narrow<ChatInviteResult>(result)?.link;
  if (!link) {
    throw new Error("Could not create invite link.");
  }

  return { data: { link }, fromCache: false };
}
