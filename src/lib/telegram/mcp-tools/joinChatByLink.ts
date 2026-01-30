import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { joinChatByLink as joinChatByLinkApi } from "../api/joinChatByLink";

export const tool: MCPTool = {
  name: "join_chat_by_link",
  description: "Join a chat using an invite link",
  inputSchema: {
    type: "object",
    properties: {
      link: {
        type: "string",
        description:
          "Invite link (e.g. https://t.me/+HASH or https://t.me/joinchat/HASH)",
      },
    },
    required: ["link"],
  },
};

export async function joinChatByLink(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const link = typeof args.link === "string" ? args.link : "";
    if (!link)
      return {
        content: [{ type: "text", text: "link is required" }],
        isError: true,
      };

    const { data, fromCache } = await joinChatByLinkApi(link);
    return {
      content: [{ type: "text", text: `Joined chat: ${data.chatTitle}` }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "join_chat_by_link",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.GROUP
    );
  }
}
