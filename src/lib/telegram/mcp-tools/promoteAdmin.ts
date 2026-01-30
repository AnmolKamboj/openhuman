import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { promoteAdmin as promoteAdminApi } from "../api/promoteAdmin";
import { getChatById } from "../api/helpers";

export const tool: MCPTool = {
  name: "promote_admin",
  description: "Promote a user to admin in a group or channel",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
      user_id: { type: "string", description: "User ID to promote" },
    },
    required: ["chat_id", "user_id"],
  },
};

export async function promoteAdmin(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const userId = validateId(args.user_id, "user_id");

    await promoteAdminApi(chatId, userId);

    const chat = getChatById(chatId);
    return {
      content: [
        {
          type: "text",
          text: `User ${userId} promoted to admin in ${chat?.title ?? chatId}.`,
        },
      ],
    };
  } catch (error) {
    return logAndFormatError(
      "promote_admin",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.ADMIN
    );
  }
}
