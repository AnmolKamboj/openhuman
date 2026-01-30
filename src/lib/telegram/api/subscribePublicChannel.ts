import { mtprotoService } from "../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import { toInputChannel } from "./apiCastHelpers";

export async function subscribePublicChannel(
  username: string
): Promise<ApiResult<void>> {
  const client = mtprotoService.getClient();

  await mtprotoService.withFloodWaitHandling(async () => {
    const inputChannel = await client.getInputEntity(username);
    await client.invoke(
      new Api.channels.JoinChannel({
        channel: toInputChannel(inputChannel),
      })
    );
  });

  return { data: undefined as unknown as void, fromCache: false };
}
