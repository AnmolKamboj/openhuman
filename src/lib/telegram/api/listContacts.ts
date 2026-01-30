import { mtprotoService } from "../../../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import bigInt from "big-integer";
import type { ApiUser } from "./apiResultTypes";
import { updateUsersFromApiUsers } from "../state";

export interface Contact {
  id: string;
  name: string;
  username?: string;
  phone?: string;
}

export async function listContacts(): Promise<ApiResult<Contact[]>> {
  const client = mtprotoService.getClient();

  const result = await mtprotoService.withFloodWaitHandling(async () => {
    return client.invoke(new Api.contacts.GetContacts({ hash: bigInt(0) }));
  });

  if (!result || !("users" in result) || !Array.isArray(result.users)) {
    return { data: [], fromCache: false };
  }

  // Update Redux state with fetched users
  updateUsersFromApiUsers(result.users as ApiUser[]);

  const contacts: Contact[] = result.users.map((u: ApiUser) => ({
    id: String(u.id),
    name: [u.firstName, u.lastName].filter(Boolean).join(" ") || "Unknown",
    username: u.username,
    phone: u.phone,
  }));

  return { data: contacts, fromCache: false };
}
