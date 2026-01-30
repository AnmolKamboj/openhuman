import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { exportContacts as exportContactsApi } from "../api/exportContacts";

export const tool: MCPTool = {
  name: "export_contacts",
  description: "Export all contacts from Telegram",
  inputSchema: { type: "object", properties: {} },
};

export async function exportContacts(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const { data: contacts, fromCache } = await exportContactsApi();

    if (contacts.length === 0) {
      return {
        content: [{ type: "text", text: "No contacts to export." }],
        fromCache,
      };
    }

    return {
      content: [{ type: "text", text: JSON.stringify(contacts, null, 2) }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "export_contacts",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.CONTACT
    );
  }
}
