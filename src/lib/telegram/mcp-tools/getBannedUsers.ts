import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { optNumber } from "../args";
import { getBannedUsers as getBannedUsersApi } from "../api/getBannedUsers";

export const tool: MCPTool = {
  name: "get_banned_users",
  description: "Get banned users in a group or channel",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
      limit: { type: "number", description: "Max results", default: 50 },
    },
    required: ["chat_id"],
  },
};

export async function getBannedUsers(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const limit = optNumber(args, "limit", 50);

    const { data: bannedUsers, fromCache } = await getBannedUsersApi(
      chatId,
      limit
    );

    if (bannedUsers.length === 0) {
      return {
        content: [{ type: "text", text: "No banned users found." }],
        fromCache,
      };
    }

    const lines = bannedUsers.map((u) => {
      const username = u.username ? `@${u.username}` : "";
      return `ID: ${u.id} | ${u.name} ${username}`.trim();
    });

    return {
      content: [
        {
          type: "text",
          text: `${lines.length} banned users:\n${lines.join("\n")}`,
        },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "get_banned_users",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.ADMIN
    );
  }
}
