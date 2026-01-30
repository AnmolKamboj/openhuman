import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { getUserStatus as getUserStatusApi } from "../api/getUserStatus";

export const tool: MCPTool = {
  name: "get_user_status",
  description: "Get online status of a user",
  inputSchema: {
    type: "object",
    properties: {
      user_id: { type: "string", description: "User ID" },
    },
    required: ["user_id"],
  },
};

export async function getUserStatus(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const userId = validateId(args.user_id, "user_id");

    const { data: userStatus, fromCache } = await getUserStatusApi(userId);

    let statusText = userStatus.status;
    if (userStatus.lastSeen) {
      statusText += " (last seen: " + userStatus.lastSeen + ")";
    }

    return {
      content: [
        {
          type: "text",
          text:
            userStatus.name + " (ID: " + userStatus.userId + "): " + statusText,
        },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "get_user_status",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.CONTACT
    );
  }
}
