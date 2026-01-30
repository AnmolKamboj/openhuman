/**
 * Get Recent Actions API - Get recent admin actions in a chat
 *
 * Fetches admin log from channels/supergroups only.
 */

import { mtprotoService } from "../services/mtprotoService";
import { getChatById } from "./helpers";
import type { ApiResult } from "./types";
import type { AdminLogResult } from "./apiResultTypes";
import { toInputChannel, narrow } from "./apiCastHelpers";
import { Api } from "telegram";
import bigInt from "big-integer";

export interface AdminActionResult {
  date: string;
  action: string;
}

export async function getRecentActions(
  chatId: string | number,
  limit: number
): Promise<ApiResult<AdminActionResult[]>> {
  const chat = getChatById(chatId);
  if (!chat) {
    throw new Error("Chat not found: " + chatId);
  }

  if (chat.type !== "channel" && chat.type !== "supergroup") {
    throw new Error(
      "Recent actions are only available for channels/supergroups."
    );
  }

  const client = mtprotoService.getClient();
  const entity = chat.username ? chat.username : chat.id;

  const result = await mtprotoService.withFloodWaitHandling(async () => {
    const inputChannel = await client.getInputEntity(entity);
    return client.invoke(
      new Api.channels.GetAdminLog({
        channel: toInputChannel(inputChannel),
        q: "",
        maxId: bigInt(0),
        minId: bigInt(0),
        limit,
      })
    );
  });

  const events = narrow<AdminLogResult>(result)?.events;
  if (!events || !Array.isArray(events) || events.length === 0) {
    return { data: [], fromCache: false };
  }

  const actions: AdminActionResult[] = events.map((e) => {
    const date = e.date ? new Date(e.date * 1000).toISOString() : "unknown";
    const action = e.action?.className ?? "unknown";
    return { date, action };
  });

  return { data: actions, fromCache: false };
}
