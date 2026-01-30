import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { editChatPhoto as editChatPhotoApi } from "../api/editChatPhoto";

export const tool: MCPTool = {
  name: "edit_chat_photo",
  description: "Edit chat photo",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
      file_path: { type: "string", description: "Path to the photo file" },
    },
    required: ["chat_id", "file_path"],
  },
};

export async function editChatPhoto(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const filePath = typeof args.file_path === "string" ? args.file_path : "";
    await editChatPhotoApi(chatId, filePath);
    return {
      content: [{ type: "text", text: "Unreachable code" }],
      isError: true,
    };
  } catch (error) {
    return logAndFormatError(
      "edit_chat_photo",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.GROUP
    );
  }
}
