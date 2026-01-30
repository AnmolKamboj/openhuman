import { mtprotoService } from "../../../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import type { ApiUser } from "./apiResultTypes";
import { toInputUser, narrow } from "./apiCastHelpers";
import { updateUsersFromApiUsers } from "../state";

export interface UserStatus {
  userId: string;
  name: string;
  status: string;
  lastSeen?: string;
}

export async function getUserStatus(
  userId: string | number
): Promise<ApiResult<UserStatus>> {
  const client = mtprotoService.getClient();

  const result = await mtprotoService.withFloodWaitHandling(async () => {
    const inputUser = await client.getInputEntity(userId);
    return client.invoke(
      new Api.users.GetUsers({ id: [toInputUser(inputUser)] })
    );
  });

  if (!result || !Array.isArray(result) || result.length === 0) {
    throw new Error("User " + userId + " not found.");
  }

  const user = narrow<ApiUser>(result[0]);

  // Update Redux state with fetched user
  updateUsersFromApiUsers([user]);

  const name =
    [user.firstName, user.lastName].filter(Boolean).join(" ") || "Unknown";
  let statusText = "unknown";
  let lastSeen: string | undefined = undefined;

  if (user.status) {
    const s = user.status;
    if (s.className === "UserStatusOnline") {
      statusText = "Online";
    } else if (s.className === "UserStatusOffline") {
      const lastSeenDate = s.wasOnline
        ? new Date(s.wasOnline * 1000).toISOString()
        : "unknown";
      statusText = "Offline";
      lastSeen = lastSeenDate;
    } else if (s.className === "UserStatusRecently") {
      statusText = "Recently";
    } else if (s.className === "UserStatusLastWeek") {
      statusText = "Last week";
    } else if (s.className === "UserStatusLastMonth") {
      statusText = "Last month";
    } else {
      statusText = s.className ?? "unknown";
    }
  }

  return {
    data: {
      userId: String(user.id),
      name,
      status: statusText,
      lastSeen,
    },
    fromCache: false,
  };
}
