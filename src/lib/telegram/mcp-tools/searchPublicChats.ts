/**
 * Search Public Chats tool - Search for public chats, channels, or bots
 *
 * Uses Telegram's contacts.Search API for server-side discovery.
 * Falls back to filtering cached chats if the API call fails.
 */

import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { searchPublicChats as searchPublicChatsApi } from "../api/searchPublicChats";

export const tool: MCPTool = {
  name: "search_public_chats",
  description:
    "Search for public chats, channels, or bots by username or title",
  inputSchema: {
    type: "object",
    properties: {
      query: { type: "string", description: "Search query" },
    },
    required: ["query"],
  },
};

export async function searchPublicChats(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const query = typeof args.query === "string" ? args.query : "";
    if (!query) {
      return {
        content: [{ type: "text", text: "query is required" }],
        isError: true,
      };
    }

    const { data: results, fromCache } = await searchPublicChatsApi(query);

    if (results.length === 0) {
      return {
        content: [
          {
            type: "text",
            text: `No public chats matching "${query}" found.`,
          },
        ],
        fromCache,
      };
    }

    const prefix = fromCache ? "(cached search)\n" : "";
    return {
      content: [
        {
          type: "text",
          text: prefix + JSON.stringify(results, undefined, 2),
        },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "search_public_chats",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.SEARCH
    );
  }
}
