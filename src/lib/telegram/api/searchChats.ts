/**
 * API: Search chats — cache-only.
 */

import type { TelegramChat } from "../../../store/telegram/types";
import { searchChatsInCache } from "./helpers";
import type { ApiResult } from "./types";

export async function searchChats(
  query: string
): Promise<ApiResult<TelegramChat[]>> {
  const data = searchChatsInCache(query);
  return { data, fromCache: true };
}
