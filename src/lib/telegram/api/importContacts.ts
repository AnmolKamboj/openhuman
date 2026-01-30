import { mtprotoService } from "../../../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import bigInt from "big-integer";
import type { ContactInput, ImportContactsResult } from "./apiResultTypes";
import { narrow } from "./apiCastHelpers";

export interface ImportContactsResponse {
  imported: number;
  total: number;
}

export async function importContacts(
  contacts: ContactInput[]
): Promise<ApiResult<ImportContactsResponse>> {
  if (!Array.isArray(contacts) || contacts.length === 0) {
    throw new Error("contacts array is required and must not be empty.");
  }

  const inputContacts = contacts.map(
    (c: ContactInput, i: number) =>
      new Api.InputPhoneContact({
        clientId: bigInt(i),
        phone: String(c.phone ?? ""),
        firstName: String(c.first_name ?? ""),
        lastName: String(c.last_name ?? ""),
      })
  );

  const client = mtprotoService.getClient();

  const result = await mtprotoService.withFloodWaitHandling(async () => {
    return client.invoke(
      new Api.contacts.ImportContacts({ contacts: inputContacts })
    );
  });

  const imported = narrow<ImportContactsResult>(result)?.imported?.length ?? 0;

  return {
    data: { imported, total: contacts.length },
    fromCache: false,
  };
}
