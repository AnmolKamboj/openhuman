/**
 * Search Messages tool - Search for messages in a chat by text
 *
 * Uses Telegram's messages.Search API for server-side full-text search.
 * Falls back to filtering cached messages if the API call fails.
 */

import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";

import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { optNumber } from "../args";
import { validateId } from "../../mcp/validation";
import { searchMessages as searchMessagesApi } from "../api/searchMessages";

export const tool: MCPTool = {
  name: "search_messages",
  description: "Search for messages in a chat by text",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "The chat ID or username" },
      query: { type: "string", description: "Search query" },
      limit: {
        type: "number",
        description: "Maximum number of messages to return",
        default: 20,
      },
    },
    required: ["chat_id", "query"],
  },
};

export async function searchMessages(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const query = typeof args.query === "string" ? args.query : "";
    const limit = optNumber(args, "limit", 20);

    if (!query) {
      return {
        content: [{ type: "text", text: "query is required" }],
        isError: true,
      };
    }

    const { data: messages, fromCache } = await searchMessagesApi(
      chatId,
      query,
      limit
    );

    if (messages.length === 0) {
      return {
        content: [
          { type: "text", text: `No messages matching "${query}" found.` },
        ],
        fromCache,
      };
    }

    const lines = messages.map((msg) => {
      const parts = [`ID: ${msg.id}`, `Date: ${msg.date}`, msg.text];
      if (msg.from) {
        parts.splice(1, 0, msg.from);
      }
      return parts.join(" | ");
    });

    const prefix = fromCache ? "(cached search)\n" : "";

    return {
      content: [{ type: "text", text: prefix + lines.join("\n") }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "search_messages",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.SEARCH
    );
  }
}
