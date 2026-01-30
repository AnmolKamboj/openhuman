import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { optNumber } from "../args";
import { getRecentActions as getRecentActionsApi } from "../api/getRecentActions";

export const tool: MCPTool = {
  name: "get_recent_actions",
  description: "Get recent admin actions in a chat",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
      limit: { type: "number", description: "Max actions", default: 20 },
    },
    required: ["chat_id"],
  },
};

export async function getRecentActions(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const limit = optNumber(args, "limit", 20);

    const { data: actions, fromCache } = await getRecentActionsApi(
      chatId,
      limit
    );

    if (actions.length === 0) {
      return {
        content: [{ type: "text", text: "No recent actions found." }],
        fromCache,
      };
    }

    const lines = actions.map((a) => a.date + " | " + a.action);

    return {
      content: [{ type: "text", text: lines.join("\n") }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "get_recent_actions",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.ADMIN
    );
  }
}
