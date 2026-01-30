/**
 * API: Get message reactions
 */

import { mtprotoService } from "../services/mtprotoService";
import { getChatById } from "./helpers";
import type { ApiResult } from "./types";
import { updateMessageReactionsInState } from "../state";
import type { UpdatesResult } from "./apiResultTypes";
import { Api } from "telegram";
import { narrow } from "./apiCastHelpers";

export interface ReactionData {
  emoji: string;
  count: number;
}

export async function getMessageReactions(
  chatId: string | number,
  messageId: number
): Promise<ApiResult<ReactionData[]>> {
  const chat = getChatById(chatId);
  if (!chat) {
    throw new Error(`Chat not found: ${chatId}`);
  }

  const client = mtprotoService.getClient();
  const entity = chat.username ? `@${chat.username.replace("@", "")}` : chat.id;

  const result = await mtprotoService.withFloodWaitHandling(async () => {
    const inputPeer = await client.getInputEntity(entity);
    return client.invoke(
      new Api.messages.GetMessagesReactions({
        peer: inputPeer,
        id: [messageId],
      })
    );
  });

  const updates = narrow<UpdatesResult>(result);
  if (!updates || !updates.updates || updates.updates.length === 0) {
    return { data: [], fromCache: false };
  }

  const reactions: ReactionData[] = [];
  for (const update of updates.updates) {
    if (update.reactions && update.reactions.results) {
      for (const r of update.reactions.results) {
        const emoji = r.reaction?.emoticon ?? r.reaction?.className ?? "?";
        const count = r.count ?? 0;
        reactions.push({ emoji, count });
      }
    }
  }

  // Update Redux state with fetched reactions
  if (reactions.length > 0) {
    updateMessageReactionsInState(
      chat.id,
      String(messageId),
      reactions.map((r) => ({ emoticon: r.emoji, count: r.count }))
    );
  }

  return { data: reactions, fromCache: false };
}
