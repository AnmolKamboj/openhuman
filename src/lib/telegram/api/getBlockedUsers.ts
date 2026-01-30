import { mtprotoService } from "../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import type { ApiUser } from "./apiResultTypes";
import { updateUsersFromApiUsers } from "../state";

export interface BlockedUser {
  id: string;
  name: string;
  username?: string;
}

export async function getBlockedUsers(
  limit: number = 50
): Promise<ApiResult<BlockedUser[]>> {
  const client = mtprotoService.getClient();

  const result = await mtprotoService.withFloodWaitHandling(async () => {
    return client.invoke(new Api.contacts.GetBlocked({ offset: 0, limit }));
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

  const blockedUsers: BlockedUser[] = result.users.map((u: ApiUser) => ({
    id: String(u.id),
    name: [u.firstName, u.lastName].filter(Boolean).join(" ") || "Unknown",
    username: u.username,
  }));

  return { data: blockedUsers, fromCache: false };
}
