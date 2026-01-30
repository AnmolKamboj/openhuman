import { mtprotoService } from "../../../services/mtprotoService";
import { getChatById } from "./helpers";
import type { ApiResult } from "./types";
import { Api } from "telegram";

export async function muteChat(
  chatId: string | number,
  duration?: number
): Promise<ApiResult<void>> {
  const chat = getChatById(chatId);
  if (!chat) throw new Error(`Chat not found: ${chatId}`);

  const client = mtprotoService.getClient();
  const entity = chat.username ? chat.username : chat.id;

  const muteUntil =
    duration === 0 || duration === undefined
      ? 2147483647
      : Math.floor(Date.now() / 1000) + duration;

  await mtprotoService.withFloodWaitHandling(async () => {
    const inputPeer = await client.getInputEntity(entity);
    await client.invoke(
      new Api.account.UpdateNotifySettings({
        peer: new Api.InputNotifyPeer({ peer: inputPeer }),
        settings: new Api.InputPeerNotifySettings({
          muteUntil,
        }),
      })
    );
  });

  return { data: undefined as unknown as void, fromCache: false };
}
