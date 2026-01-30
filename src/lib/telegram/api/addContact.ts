import { mtprotoService } from "../../../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import bigInt from "big-integer";

export async function addContact(
  phone: string,
  firstName: string,
  lastName?: string
): Promise<ApiResult<void>> {
  if (!phone) {
    throw new Error("phone is required");
  }
  if (!firstName) {
    throw new Error("first_name is required");
  }

  const client = mtprotoService.getClient();

  await mtprotoService.withFloodWaitHandling(async () => {
    await client.invoke(
      new Api.contacts.ImportContacts({
        contacts: [
          new Api.InputPhoneContact({
            clientId: bigInt(0),
            phone,
            firstName,
            lastName: lastName ?? "",
          }),
        ],
      })
    );
  });

  return { data: undefined, fromCache: false };
}
