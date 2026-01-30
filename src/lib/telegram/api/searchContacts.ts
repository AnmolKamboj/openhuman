import { mtprotoService } from "../../../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import type { ApiUser } from "./apiResultTypes";
import { updateUsersFromApiUsers } from "../state";

export interface ContactSearchResult {
  id: string;
  name: string;
  username?: string;
}

export async function searchContacts(
  query: string,
  limit: number = 20
): Promise<ApiResult<ContactSearchResult[]>> {
  if (!query) {
    throw new Error("query is required");
  }

  const client = mtprotoService.getClient();

  const result = await mtprotoService.withFloodWaitHandling(async () => {
    return client.invoke(new Api.contacts.Search({ q: query, limit }));
  });

  if (
    !result ||
    !("users" in result) ||
    !Array.isArray(result.users) ||
    result.users.length === 0
  ) {
    return { data: [], fromCache: false };
  }

  // Update Redux state with fetched users
  updateUsersFromApiUsers(result.users as ApiUser[]);

  const contacts: ContactSearchResult[] = result.users.map((u: ApiUser) => ({
    id: String(u.id),
    name: [u.firstName, u.lastName].filter(Boolean).join(" ") || "Unknown",
    username: u.username,
  }));

  return { data: contacts, fromCache: false };
}
