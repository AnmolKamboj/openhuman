import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { getContactIds as getContactIdsApi } from "../api/getContactIds";

export const tool: MCPTool = {
  name: "get_contact_ids",
  description: "Get IDs of all contacts",
  inputSchema: { type: "object", properties: {} },
};

export async function getContactIds(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const { data: ids, fromCache } = await getContactIdsApi();

    if (ids.length === 0) {
      return {
        content: [{ type: "text", text: "No contact IDs found." }],
        fromCache,
      };
    }

    return {
      content: [
        { type: "text", text: `${ids.length} contacts:\n${ids.join("\n")}` },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "get_contact_ids",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.CONTACT
    );
  }
}
