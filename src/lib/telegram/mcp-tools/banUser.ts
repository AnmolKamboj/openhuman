import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { banUser as banUserApi } from "../api/banUser";
import { getChatById } from "../api/helpers";

export const tool: MCPTool = {
  name: "ban_user",
  description: "Ban a user from a group or channel",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
      user_id: { type: "string", description: "User ID to ban" },
    },
    required: ["chat_id", "user_id"],
  },
};

export async function banUser(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const userId = validateId(args.user_id, "user_id");

    await banUserApi(chatId, userId);

    const chat = getChatById(chatId);
    return {
      content: [
        {
          type: "text",
          text: `User ${userId} banned from ${chat?.title ?? chatId}.`,
        },
      ],
    };
  } catch (error) {
    return logAndFormatError(
      "ban_user",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.ADMIN
    );
  }
}
