import type { ApiResult } from "./types";

export async function setProfilePhoto(
  _filePath: string,
): Promise<ApiResult<void>> {
  throw new Error(
    "set_profile_photo requires file upload which is not supported via MCP text interface. Use the Telegram client directly.",
  );
}
