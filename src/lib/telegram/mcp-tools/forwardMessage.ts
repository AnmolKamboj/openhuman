import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId, validatePositiveInt } from "../../mcp/validation";
import { forwardMessage as forwardMessageApi } from "../api/forwardMessage";

export const tool: MCPTool = {
  name: "forward_message",
  description: "Forward a message to another chat",
  inputSchema: {
    type: "object",
    properties: {
      from_chat_id: { type: "string", description: "Source chat ID" },
      to_chat_id: { type: "string", description: "Target chat ID" },
      message_id: { type: "number", description: "Message ID" },
    },
    required: ["from_chat_id", "to_chat_id", "message_id"],
  },
};

export async function forwardMessage(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const fromChatId = validateId(args.from_chat_id, "from_chat_id");
    const toChatId = validateId(args.to_chat_id, "to_chat_id");
    const messageId = validatePositiveInt(args.message_id, "message_id");

    const { fromCache } = await forwardMessageApi(
      fromChatId,
      toChatId,
      messageId
    );

    return {
      content: [
        {
          type: "text",
          text: `Message ${messageId} forwarded from ${fromChatId} to ${toChatId}.`,
        },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "forward_message",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.MSG
    );
  }
}
