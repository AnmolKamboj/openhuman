/**
 * Get Sticker Sets API - Get user's sticker sets
 *
 * Fetches all available sticker sets for the current user.
 */

import { mtprotoService } from "../../../services/mtprotoService";
import type { ApiResult } from "./types";
import type { StickerSetsResult } from "./apiResultTypes";
import { narrow } from "./apiCastHelpers";
import { Api } from "telegram";
import bigInt from "big-integer";

export interface StickerSetResult {
  id: string;
  title: string;
  count: number;
}

export async function getStickerSets(): Promise<ApiResult<StickerSetResult[]>> {
  const client = mtprotoService.getClient();

  const result = await mtprotoService.withFloodWaitHandling(async () => {
    return client.invoke(new Api.messages.GetAllStickers({ hash: bigInt(0) }));
  });

  const sets = narrow<StickerSetsResult>(result)?.sets;
  if (!sets || !Array.isArray(sets) || sets.length === 0) {
    return { data: [], fromCache: false };
  }

  const stickerSets: StickerSetResult[] = sets.map((s) => ({
    id: String(s.id),
    title: s.title ?? "Untitled",
    count: s.count ?? 0,
  }));

  return { data: stickerSets, fromCache: false };
}
