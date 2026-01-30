import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { getDirectChatByContact as getDirectChatApi } from "../api/getDirectChatByContact";

export const tool: MCPTool = {
  name: "get_direct_chat_by_contact",
  description: "Get direct message chat with a contact",
  inputSchema: {
    type: "object",
    properties: {
      user_id: { type: "string", description: "User ID" },
    },
    required: ["user_id"],
  },
};

export async function getDirectChatByContact(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const userId = validateId(args.user_id, "user_id");
    const { data: dmChat, fromCache } = await getDirectChatApi(userId);

    if (!dmChat) {
      return {
        content: [
          {
            type: "text",
            text: "No direct chat found with user " + userId + ".",
          },
        ],
        fromCache,
      };
    }

    return {
      content: [
        {
          type: "text",
          text:
            "Chat ID: " +
            dmChat.id +
            " | Title: " +
            dmChat.title +
            " | Username: " +
            (dmChat.username ?? "N/A"),
        },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "get_direct_chat_by_contact",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.CONTACT
    );
  }
}
