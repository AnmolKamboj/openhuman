/**
 * API: Get all drafts
 */

import { mtprotoService } from "../../../services/mtprotoService";
import type { ApiResult } from "./types";
import type { UpdatesResult } from "./apiResultTypes";
import { Api } from "telegram";
import { narrow } from "./apiCastHelpers";

export interface DraftData {
  peerId: string;
  message: string;
}

export async function getDrafts(): Promise<ApiResult<DraftData[]>> {
  const client = mtprotoService.getClient();

  const result = await mtprotoService.withFloodWaitHandling(async () => {
    return client.invoke(new Api.messages.GetAllDrafts());
  });

  const updates = narrow<UpdatesResult>(result);
  if (!updates || !updates.updates || updates.updates.length === 0) {
    return { data: [], fromCache: false };
  }

  const drafts: DraftData[] = [];
  for (const update of updates.updates) {
    if (update.draft && update.draft.message) {
      const peerId =
        update.peer?.userId ??
        update.peer?.chatId ??
        update.peer?.channelId ??
        "?";
      drafts.push({
        peerId: String(peerId),
        message: update.draft.message,
      });
    }
  }

  return { data: drafts, fromCache: false };
}
