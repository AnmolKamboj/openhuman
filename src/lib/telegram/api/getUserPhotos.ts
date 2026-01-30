import { mtprotoService } from "../../../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import bigInt from "big-integer";
import type { ApiPhoto } from "./apiResultTypes";
import { toInputUser, narrow } from "./apiCastHelpers";

export interface UserPhoto {
  id: string;
  date: string;
}

export async function getUserPhotos(
  userId: string | number,
  limit: number = 10
): Promise<ApiResult<UserPhoto[]>> {
  const client = mtprotoService.getClient();

  const result = await mtprotoService.withFloodWaitHandling(async () => {
    const inputUser = await client.getInputEntity(userId);
    return client.invoke(
      new Api.photos.GetUserPhotos({
        userId: toInputUser(inputUser),
        offset: 0,
        maxId: bigInt(0),
        limit,
      })
    );
  });

  if (
    !result ||
    !("photos" in result) ||
    !Array.isArray(result.photos) ||
    result.photos.length === 0
  ) {
    return { data: [], fromCache: false };
  }

  const photos = narrow<ApiPhoto[]>(result.photos).map((photo) => ({
    id: String(photo.id),
    date: photo.date ? new Date(photo.date * 1000).toISOString() : "unknown",
  }));

  return { data: photos, fromCache: false };
}
