/**
 * Get GIF Search API - Search GIFs using Telegram's inline bot
 *
 * Uses the @gif bot to search for GIFs.
 */

import { mtprotoService } from "../services/mtprotoService";
import type { ApiResult } from "./types";
import type { InlineBotResults } from "./apiResultTypes";
import { toInputUser, narrow } from "./apiCastHelpers";
import { Api } from "telegram";

export interface GifResult {
  title: string;
  description?: string;
}

export async function getGifSearch(
  query: string,
  limit: number
): Promise<ApiResult<GifResult[]>> {
  const client = mtprotoService.getClient();

  const result = await mtprotoService.withFloodWaitHandling(async () => {
    const bot = await client.getInputEntity("gif");
    return client.invoke(
      new Api.messages.GetInlineBotResults({
        bot: toInputUser(bot),
        peer: new Api.InputPeerSelf(),
        query,
        offset: "",
      })
    );
  });

  const results = narrow<InlineBotResults>(result)?.results;
  if (!results || !Array.isArray(results) || results.length === 0) {
    return { data: [], fromCache: false };
  }

  const gifs: GifResult[] = results.slice(0, limit).map((r) => ({
    title: r.title ?? r.description ?? "GIF",
    description: r.description,
  }));

  return { data: gifs, fromCache: false };
}
