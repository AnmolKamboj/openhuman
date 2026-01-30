import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { deleteChatPhoto as deleteChatPhotoApi } from "../api/deleteChatPhoto";

export const tool: MCPTool = {
  name: "delete_chat_photo",
  description: "Delete the photo of a chat",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
    },
    required: ["chat_id"],
  },
};

export async function deleteChatPhoto(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const { fromCache } = await deleteChatPhotoApi(chatId);
    return {
      content: [{ type: "text", text: `Chat photo deleted for ${chatId}.` }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "delete_chat_photo",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.GROUP
    );
  }
}
