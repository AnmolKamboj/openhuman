import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { unbanUser as unbanUserApi } from "../api/unbanUser";
import { getChatById } from "../api/helpers";

export const tool: MCPTool = {
  name: "unban_user",
  description: "Unban a user from a group or channel",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
      user_id: { type: "string", description: "User ID to unban" },
    },
    required: ["chat_id", "user_id"],
  },
};

export async function unbanUser(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const userId = validateId(args.user_id, "user_id");

    await unbanUserApi(chatId, userId);

    const chat = getChatById(chatId);
    return {
      content: [
        {
          type: "text",
          text: `User ${userId} unbanned from ${chat?.title ?? chatId}.`,
        },
      ],
    };
  } catch (error) {
    return logAndFormatError(
      "unban_user",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.ADMIN
    );
  }
}
