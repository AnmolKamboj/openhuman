import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId, validatePositiveInt } from "../../mcp/validation";
import { removeReaction as removeReactionApi } from "../api/removeReaction";

export const tool: MCPTool = {
  name: "remove_reaction",
  description: "Remove a reaction from a message",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
      message_id: { type: "number", description: "Message ID" },
      reaction: { type: "string", description: "Reaction to remove" },
    },
    required: ["chat_id", "message_id"],
  },
};

export async function removeReaction(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const messageId = validatePositiveInt(args.message_id, "message_id");

    const { fromCache } = await removeReactionApi(chatId, messageId);

    return {
      content: [
        {
          type: "text",
          text: "Reaction removed from message " + messageId + ".",
        },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "remove_reaction",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.MSG
    );
  }
}
