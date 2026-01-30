import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { markAsRead as markAsReadApi } from "../api/markAsRead";

export const tool: MCPTool = {
  name: "mark_as_read",
  description: "Mark messages as read in a chat",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
    },
    required: ["chat_id"],
  },
};

export async function markAsRead(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");

    const { fromCache } = await markAsReadApi(chatId);

    return {
      content: [
        { type: "text", text: `Messages in chat ${chatId} marked as read.` },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "mark_as_read",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.MSG
    );
  }
}
