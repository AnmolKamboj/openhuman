import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId, validatePositiveInt } from "../../mcp/validation";
import { sendReaction as sendReactionApi } from "../api/sendReaction";

export const tool: MCPTool = {
  name: "send_reaction",
  description: "Send a reaction to a message",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
      message_id: { type: "number", description: "Message ID" },
      reaction: { type: "string", description: "Reaction emoji" },
    },
    required: ["chat_id", "message_id"],
  },
};

export async function sendReaction(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const messageId = validatePositiveInt(args.message_id, "message_id");
    const emoji =
      typeof args.reaction === "string" ? args.reaction : "\ud83d\udc4d";

    const { fromCache } = await sendReactionApi(chatId, messageId, emoji);

    return {
      content: [
        {
          type: "text",
          text: "Reaction " + emoji + " sent to message " + messageId + ".",
        },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "send_reaction",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.MSG
    );
  }
}
