import type { MCPTool, MCPToolResult } from "../../types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../errorHandler";
import { validateId } from "../../validation";
import { getInviteLink as getInviteLinkApi } from "../api/getInviteLink";

export const tool: MCPTool = {
  name: "get_invite_link",
  description: "Get the invite link for a chat",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
    },
    required: ["chat_id"],
  },
};

export async function getInviteLink(
  args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const { data, fromCache } = await getInviteLinkApi(chatId);
    return {
      content: [{ type: "text", text: `Invite link: ${data.link}` }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "get_invite_link",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.GROUP,
    );
  }
}
