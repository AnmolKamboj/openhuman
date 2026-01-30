import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { blockUser as blockUserApi } from "../api/blockUser";

export const tool: MCPTool = {
  name: "block_user",
  description: "Block a user",
  inputSchema: {
    type: "object",
    properties: {
      user_id: { type: "string", description: "User ID to block" },
    },
    required: ["user_id"],
  },
};

export async function blockUser(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const userId = validateId(args.user_id, "user_id");

    await blockUserApi(userId);

    return {
      content: [{ type: "text", text: `User ${userId} blocked.` }],
    };
  } catch (error) {
    return logAndFormatError(
      "block_user",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.CONTACT
    );
  }
}
