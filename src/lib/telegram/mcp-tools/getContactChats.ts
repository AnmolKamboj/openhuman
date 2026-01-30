import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { getContactChats as getContactChatsApi } from "../api/getContactChats";

export const tool: MCPTool = {
  name: "get_contact_chats",
  description: "Get all chats that are direct messages with contacts",
  inputSchema: { type: "object", properties: {} },
};

export async function getContactChats(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const { data: dmChats, fromCache } = await getContactChatsApi();

    if (dmChats.length === 0) {
      return {
        content: [{ type: "text", text: "No contact chats found." }],
        fromCache,
      };
    }

    const lines = dmChats.map((c) => {
      const username = c.username ? "@" + c.username : "";
      return ("ID: " + c.id + " | " + c.title + " " + username).trim();
    });

    return { content: [{ type: "text", text: lines.join("\n") }], fromCache };
  } catch (error) {
    return logAndFormatError(
      "get_contact_chats",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.CONTACT
    );
  }
}
