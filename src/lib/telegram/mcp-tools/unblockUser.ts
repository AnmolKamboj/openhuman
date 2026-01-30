import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { unblockUser as unblockUserApi } from "../api/unblockUser";

export const tool: MCPTool = {
  name: "unblock_user",
  description: "Unblock a user",
  inputSchema: {
    type: "object",
    properties: {
      user_id: { type: "string", description: "User ID to unblock" },
    },
    required: ["user_id"],
  },
};

export async function unblockUser(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const userId = validateId(args.user_id, "user_id");

    await unblockUserApi(userId);

    return { content: [{ type: "text", text: `User ${userId} unblocked.` }] };
  } catch (error) {
    return logAndFormatError(
      "unblock_user",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.CONTACT
    );
  }
}
