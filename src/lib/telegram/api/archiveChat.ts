import { mtprotoService } from "../services/mtprotoService";
import { getChatById } from "./helpers";
import type { ApiResult } from "./types";
import { Api } from "telegram";

export async function archiveChat(
  chatId: string | number
): Promise<ApiResult<void>> {
  const chat = getChatById(chatId);
  if (!chat) throw new Error(`Chat not found: ${chatId}`);

  const client = mtprotoService.getClient();
  const entity = chat.username ? chat.username : chat.id;

  await mtprotoService.withFloodWaitHandling(async () => {
    const inputPeer = await client.getInputEntity(entity);
    await client.invoke(
      new Api.folders.EditPeerFolders({
        folderPeers: [
          new Api.InputFolderPeer({
            peer: inputPeer,
            folderId: 1, // 1 = Archive folder
          }),
        ],
      })
    );
  });

  return { data: undefined as unknown as void, fromCache: false };
}
