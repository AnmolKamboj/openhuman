import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { optNumber } from "../args";
import { searchContacts as searchContactsApi } from "../api/searchContacts";

export const tool: MCPTool = {
  name: "search_contacts",
  description: "Search contacts by name or username",
  inputSchema: {
    type: "object",
    properties: {
      query: { type: "string", description: "Search query" },
      limit: { type: "number", description: "Max results", default: 20 },
    },
    required: ["query"],
  },
};

export async function searchContacts(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const query = typeof args.query === "string" ? args.query : "";
    const limit = optNumber(args, "limit", 20);

    const { data: contacts, fromCache } = await searchContactsApi(query, limit);

    if (contacts.length === 0) {
      return {
        content: [{ type: "text", text: `No contacts found for "${query}".` }],
        fromCache,
      };
    }

    const lines = contacts.map((c) => {
      const username = c.username ? `@${c.username}` : "";
      return `ID: ${c.id} | ${c.name} ${username}`.trim();
    });

    return {
      content: [{ type: "text", text: lines.join("\n") }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "search_contacts",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.CONTACT
    );
  }
}
