import { mtprotoService } from "../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import bigInt from "big-integer";
import type { ApiPhoto } from "./apiResultTypes";
import { narrow } from "./apiCastHelpers";

export async function deleteProfilePhoto(): Promise<ApiResult<void>> {
  const client = mtprotoService.getClient();

  const photos = await mtprotoService.withFloodWaitHandling(async () => {
    return client.invoke(
      new Api.photos.GetUserPhotos({
        userId: new Api.InputUserSelf(),
        offset: 0,
        maxId: bigInt(0),
        limit: 1,
      })
    );
  });

  if (
    !photos ||
    !("photos" in photos) ||
    !Array.isArray(photos.photos) ||
    photos.photos.length === 0
  ) {
    throw new Error("No profile photo to delete.");
  }

  const photo = narrow<ApiPhoto>(photos.photos[0]);

  await mtprotoService.withFloodWaitHandling(async () => {
    await client.invoke(
      new Api.photos.DeletePhotos({
        id: [
          new Api.InputPhoto({
            id: photo.id,
            accessHash: photo.accessHash,
            fileReference: photo.fileReference,
          }),
        ],
      })
    );
  });

  return { data: undefined, fromCache: false };
}
