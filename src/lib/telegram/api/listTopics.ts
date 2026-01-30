/**
 * List Topics API - List forum topics in a chat
 *
 * Fetches forum topics from channels/supergroups only.
 */

import { mtprotoService } from "../services/mtprotoService";
import { getChatById } from "./helpers";
import type { ApiResult } from "./types";
import type { ForumTopicsResult } from "./apiResultTypes";
import { toInputChannel, narrow } from "./apiCastHelpers";
import { Api } from "telegram";

export interface TopicResult {
  id: number;
  title: string;
}

export async function listTopics(
  chatId: string | number
): Promise<ApiResult<TopicResult[]>> {
  const chat = getChatById(chatId);
  if (!chat) {
    throw new Error("Chat not found: " + chatId);
  }

  if (chat.type !== "channel" && chat.type !== "supergroup") {
    throw new Error(
      "Forum topics are only available for channels/supergroups."
    );
  }

  const client = mtprotoService.getClient();
  const entity = chat.username ? chat.username : chat.id;

  const result = await mtprotoService.withFloodWaitHandling(async () => {
    const inputChannel = await client.getInputEntity(entity);
    return client.invoke(
      new Api.channels.GetForumTopics({
        channel: toInputChannel(inputChannel),
        offsetDate: 0,
        offsetId: 0,
        offsetTopic: 0,
        limit: 100,
      })
    );
  });

  const topics = narrow<ForumTopicsResult>(result)?.topics;
  if (!topics || !Array.isArray(topics) || topics.length === 0) {
    return { data: [], fromCache: false };
  }

  const topicResults: TopicResult[] = topics.map((t) => ({
    id: t.id,
    title: t.title ?? "Untitled",
  }));

  return { data: topicResults, fromCache: false };
}
