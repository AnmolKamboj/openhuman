import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId, validatePositiveInt } from "../../mcp/validation";
import { getMessageReactions as getMessageReactionsApi } from "../api/getMessageReactions";

export const tool: MCPTool = {
  name: "get_message_reactions",
  description: "Get reactions on a message",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
      message_id: { type: "number", description: "Message ID" },
    },
    required: ["chat_id", "message_id"],
  },
};

export async function getMessageReactions(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const messageId = validatePositiveInt(args.message_id, "message_id");

    const { data: reactions, fromCache } = await getMessageReactionsApi(
      chatId,
      messageId
    );

    if (reactions.length === 0) {
      return {
        content: [
          {
            type: "text",
            text: "No reactions found on message " + messageId + ".",
          },
        ],
        fromCache,
      };
    }

    const lines = reactions.map((r) => r.emoji + ": " + r.count);

    return {
      content: [
        {
          type: "text",
          text: "Reactions on message " + messageId + ":\n" + lines.join("\n"),
        },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "get_message_reactions",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.MSG
    );
  }
}
