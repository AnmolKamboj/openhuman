import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { getPinnedMessages as getPinnedMessagesApi } from "../api/getPinnedMessages";

export const tool: MCPTool = {
  name: "get_pinned_messages",
  description: "Get pinned messages from a chat",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
    },
    required: ["chat_id"],
  },
};

export async function getPinnedMessages(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");

    const { data: pinnedMessages, fromCache } = await getPinnedMessagesApi(
      chatId
    );

    if (pinnedMessages.length === 0) {
      return {
        content: [{ type: "text", text: "No pinned messages found." }],
        fromCache,
      };
    }

    const pinnedLines = pinnedMessages.map(
      (msg) => `ID: ${msg.id} | Date: ${msg.date} | ${msg.text}`
    );

    return {
      content: [{ type: "text", text: pinnedLines.join("\n") }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "get_pinned_messages",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.MSG
    );
  }
}
