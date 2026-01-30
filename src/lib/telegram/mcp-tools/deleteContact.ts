import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { deleteContact as deleteContactApi } from "../api/deleteContact";

export const tool: MCPTool = {
  name: "delete_contact",
  description: "Delete a contact from your Telegram account",
  inputSchema: {
    type: "object",
    properties: {
      user_id: {
        type: "string",
        description: "User ID to remove from contacts",
      },
    },
    required: ["user_id"],
  },
};

export async function deleteContact(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const userId = validateId(args.user_id, "user_id");

    await deleteContactApi(userId);

    return {
      content: [{ type: "text", text: `Contact ${userId} deleted.` }],
    };
  } catch (error) {
    return logAndFormatError(
      "delete_contact",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.CONTACT
    );
  }
}
