import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { optNumber } from "../args";
import { getBlockedUsers as getBlockedUsersApi } from "../api/getBlockedUsers";

export const tool: MCPTool = {
  name: "get_blocked_users",
  description: "Get list of blocked users",
  inputSchema: {
    type: "object",
    properties: {
      limit: { type: "number", description: "Max results", default: 50 },
    },
  },
};

export async function getBlockedUsers(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const limit = optNumber(args, "limit", 50);

    const { data: blockedUsers, fromCache } = await getBlockedUsersApi(limit);

    if (blockedUsers.length === 0) {
      return {
        content: [{ type: "text", text: "No blocked users." }],
        fromCache,
      };
    }

    const lines = blockedUsers.map((u) => {
      const username = u.username ? `@${u.username}` : "";
      return `ID: ${u.id} | ${u.name} ${username}`.trim();
    });

    return {
      content: [{ type: "text", text: lines.join("\n") }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "get_blocked_users",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.CONTACT
    );
  }
}
