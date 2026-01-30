import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId, validatePositiveInt } from "../../mcp/validation";
import { editMessage as editMessageApi } from "../api/editMessage";

export const tool: MCPTool = {
  name: "edit_message",
  description: "Edit an existing message",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
      message_id: { type: "number", description: "Message ID" },
      new_text: { type: "string", description: "New message text" },
    },
    required: ["chat_id", "message_id", "new_text"],
  },
};

export async function editMessage(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const messageId = validatePositiveInt(args.message_id, "message_id");
    const newText = typeof args.new_text === "string" ? args.new_text : "";

    if (!newText) {
      return {
        content: [{ type: "text", text: "new_text is required" }],
        isError: true,
      };
    }

    const { fromCache } = await editMessageApi(chatId, messageId, newText);

    return {
      content: [
        { type: "text", text: `Message ${messageId} edited successfully.` },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "edit_message",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.MSG
    );
  }
}
