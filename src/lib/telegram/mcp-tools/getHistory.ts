import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { getMessages as getMessagesApi } from "../api/getMessages";
import { getChatById, formatMessage } from "../api/helpers";
import { validateId } from "../../mcp/validation";
import { optNumber } from "../args";

export const tool: MCPTool = {
  name: "get_history",
  description: "Get message history from a chat",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
      limit: { type: "number", description: "Number of messages", default: 20 },
    },
    required: ["chat_id"],
  },
};

export async function getHistory(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const limit = optNumber(args, "limit", 20);

    const chat = getChatById(chatId);
    if (!chat) {
      return {
        content: [{ type: "text", text: `Chat not found: ${chatId}` }],
        isError: true,
      };
    }

    const { data: messagesData, fromCache } = await getMessagesApi(
      chatId,
      limit,
      0
    );
    const messages = messagesData.length > 0 ? messagesData : undefined;
    if (!messages || messages.length === 0) {
      return {
        content: [{ type: "text", text: "No messages found in this chat." }],
        fromCache,
      };
    }

    const lines = messages.map((msg) => {
      const f = formatMessage(msg);
      const from = msg.fromName ?? msg.fromId ?? "Unknown";
      return `ID: ${f.id} | ${from} | ${f.date} | ${
        f.text || "[Media/No text]"
      }`;
    });

    return { content: [{ type: "text", text: lines.join("\n") }], fromCache };
  } catch (error) {
    return logAndFormatError(
      "get_history",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.MSG
    );
  }
}
