import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { inviteToGroup as inviteToGroupApi } from "../api/inviteToGroup";
import { getChatById } from "../api/helpers";

export const tool: MCPTool = {
  name: "invite_to_group",
  description: "Invite users to a group or channel",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
      user_ids: {
        type: "array",
        items: { type: "string" },
        description: "User IDs to invite",
      },
    },
    required: ["chat_id", "user_ids"],
  },
};

export async function inviteToGroup(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const userIds = Array.isArray(args.user_ids)
      ? args.user_ids.map(String)
      : [];

    await inviteToGroupApi(chatId, userIds);

    const chat = getChatById(chatId);
    return {
      content: [
        {
          type: "text",
          text: `Invited ${userIds.length} user(s) to ${
            chat?.title ?? chatId
          }.`,
        },
      ],
    };
  } catch (error) {
    return logAndFormatError(
      "invite_to_group",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.GROUP
    );
  }
}
