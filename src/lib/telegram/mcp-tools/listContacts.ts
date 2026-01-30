import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { listContacts as listContactsApi } from "../api/listContacts";

export const tool: MCPTool = {
  name: "list_contacts",
  description: "List all contacts in your Telegram account",
  inputSchema: { type: "object", properties: {} },
};

export async function listContacts(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const { data: contacts, fromCache } = await listContactsApi();

    if (contacts.length === 0) {
      return {
        content: [{ type: "text", text: "No contacts found." }],
        fromCache,
      };
    }

    const lines = contacts.map((c) => {
      const username = c.username ? `@${c.username}` : "";
      const phone = c.phone ? `+${c.phone}` : "";
      return `ID: ${c.id} | ${c.name} ${username} ${phone}`.trim();
    });

    return {
      content: [{ type: "text", text: lines.join("\n") }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "list_contacts",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.CONTACT
    );
  }
}
