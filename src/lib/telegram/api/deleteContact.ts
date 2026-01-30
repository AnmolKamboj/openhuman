import { mtprotoService } from "../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import { toInputUser } from "./apiCastHelpers";

export async function deleteContact(
  userId: string | number
): Promise<ApiResult<void>> {
  const client = mtprotoService.getClient();

  await mtprotoService.withFloodWaitHandling(async () => {
    const inputUser = await client.getInputEntity(userId);
    await client.invoke(
      new Api.contacts.DeleteContacts({
        id: [toInputUser(inputUser)],
      })
    );
  });

  return { data: undefined, fromCache: false };
}
