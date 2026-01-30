import { mtprotoService } from "../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";

export interface ProfileUpdate {
  firstName?: string;
  lastName?: string;
  about?: string;
}

export async function updateProfile(
  update: ProfileUpdate
): Promise<ApiResult<string[]>> {
  if (!update.firstName && !update.lastName && !update.about) {
    throw new Error(
      "At least one of firstName, lastName, or about is required."
    );
  }

  const client = mtprotoService.getClient();

  await mtprotoService.withFloodWaitHandling(async () => {
    return client.invoke(
      new Api.account.UpdateProfile({
        firstName: update.firstName ?? undefined,
        lastName: update.lastName ?? undefined,
        about: update.about ?? undefined,
      })
    );
  });

  const updated: string[] = [];
  if (update.firstName) updated.push("first_name: " + update.firstName);
  if (update.lastName) updated.push("last_name: " + update.lastName);
  if (update.about) updated.push("about: " + update.about);

  return { data: updated, fromCache: false };
}
