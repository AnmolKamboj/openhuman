/**
 * API: Create poll
 */

import { mtprotoService } from "../services/mtprotoService";
import { getChatById } from "./helpers";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import bigInt from "big-integer";

export async function createPoll(
  chatId: string | number,
  question: string,
  options: string[]
): Promise<ApiResult<void>> {
  const chat = getChatById(chatId);
  if (!chat) {
    throw new Error(`Chat not found: ${chatId}`);
  }

  const client = mtprotoService.getClient();
  const entity = chat.username ? `@${chat.username.replace("@", "")}` : chat.id;

  await mtprotoService.withFloodWaitHandling(async () => {
    const inputPeer = await client.getInputEntity(entity);
    await client.invoke(
      new Api.messages.SendMedia({
        peer: inputPeer,
        media: new Api.InputMediaPoll({
          poll: new Api.Poll({
            id: bigInt(0),
            question: new Api.TextWithEntities({
              text: question,
              entities: [],
            }),
            answers: options.map(
              (opt, i) =>
                new Api.PollAnswer({
                  text: new Api.TextWithEntities({ text: opt, entities: [] }),
                  option: Buffer.from([i]),
                })
            ),
          }),
        }),
        message: "",
        randomId: bigInt(Math.floor(Math.random() * Number.MAX_SAFE_INTEGER)),
      })
    );
  });

  return { data: undefined as unknown as void, fromCache: false };
}
