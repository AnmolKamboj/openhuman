import { mtprotoService } from "../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import bigInt from "big-integer";
import type { ApiUser } from "./apiResultTypes";

export interface ExportedContact {
  id: string;
  firstName: string;
  lastName: string;
  username: string;
  phone: string;
}

export async function exportContacts(): Promise<ApiResult<ExportedContact[]>> {
  const client = mtprotoService.getClient();

  const result = await mtprotoService.withFloodWaitHandling(async () => {
    return client.invoke(new Api.contacts.GetContacts({ hash: bigInt(0) }));
  });

  if (
    !result ||
    !("users" in result) ||
    !Array.isArray(result.users) ||
    result.users.length === 0
  ) {
    return { data: [], fromCache: false };
  }

  const contacts: ExportedContact[] = result.users.map((u: ApiUser) => ({
    id: String(u.id),
    firstName: u.firstName ?? "",
    lastName: u.lastName ?? "",
    username: u.username ?? "",
    phone: u.phone ?? "",
  }));

  return { data: contacts, fromCache: false };
}
