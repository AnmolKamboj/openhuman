import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { subscribePublicChannel as subscribePublicChannelApi } from "../api/subscribePublicChannel";

export const tool: MCPTool = {
  name: "subscribe_public_channel",
  description: "Subscribe to a public channel by username",
  inputSchema: {
    type: "object",
    properties: {
      username: { type: "string", description: "Channel username" },
    },
    required: ["username"],
  },
};

export async function subscribePublicChannel(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const username = typeof args.username === "string" ? args.username : "";
    if (!username)
      return {
        content: [{ type: "text", text: "username is required" }],
        isError: true,
      };

    const { fromCache } = await subscribePublicChannelApi(username);
    return {
      content: [{ type: "text", text: `Subscribed to channel: ${username}` }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "subscribe_public_channel",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.GROUP
    );
  }
}
