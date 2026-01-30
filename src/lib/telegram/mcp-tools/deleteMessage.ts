import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId, validatePositiveInt } from "../../mcp/validation";
import { deleteMessage as deleteMessageApi } from "../api/deleteMessage";

export const tool: MCPTool = {
  name: "delete_message",
  description: "Delete a message",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
      message_id: { type: "number", description: "Message ID" },
    },
    required: ["chat_id", "message_id"],
  },
};

export async function deleteMessage(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const messageId = validatePositiveInt(args.message_id, "message_id");

    const { fromCache } = await deleteMessageApi(chatId, messageId);

    return {
      content: [
        { type: "text", text: `Message ${messageId} deleted successfully.` },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "delete_message",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.MSG
    );
  }
}
