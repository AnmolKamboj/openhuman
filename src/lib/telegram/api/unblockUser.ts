import { mtprotoService } from "../../../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import { toInputPeer } from "./apiCastHelpers";

export async function unblockUser(
  userId: string | number
): Promise<ApiResult<void>> {
  const client = mtprotoService.getClient();

  await mtprotoService.withFloodWaitHandling(async () => {
    const inputUser = await client.getInputEntity(userId);
    await client.invoke(
      new Api.contacts.Unblock({ id: toInputPeer(inputUser) })
    );
  });

  return { data: undefined, fromCache: false };
}
