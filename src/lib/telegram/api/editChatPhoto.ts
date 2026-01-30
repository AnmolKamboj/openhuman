import type { ApiResult } from "./types";

export async function editChatPhoto(
  _chatId: string | number,
  _filePath: string,
): Promise<ApiResult<void>> {
  throw new Error(
    "edit_chat_photo requires file upload which is not supported via MCP text interface. Use the Telegram client directly.",
  );
}
