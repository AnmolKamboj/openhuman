/**
 * Reply To Message tool - Reply to a specific message in a chat
 */

import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";

import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { sendMessage } from "../api/sendMessage";
import { getChatById } from "../api/helpers";
import { toHumanReadableAction } from "../toolActionParser";
import { validateId } from "../../mcp/validation";

export const tool: MCPTool = {
  name: "reply_to_message",
  description: "Reply to a specific message in a chat",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: {
        type: "string",
        description: "The ID or username of the chat",
      },
      message_id: { type: "number", description: "The message ID to reply to" },
      text: { type: "string", description: "The reply message text" },
    },
    required: ["chat_id", "message_id", "text"],
  },
  toHumanReadableAction: (args) =>
    toHumanReadableAction("reply_to_message", args),
};

export async function replyToMessage(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const messageId =
      typeof args.message_id === "number" && Number.isInteger(args.message_id)
        ? args.message_id
        : undefined;
    const text = typeof args.text === "string" ? args.text : "";

    if (messageId === undefined) {
      return {
        content: [{ type: "text", text: "message_id must be an integer" }],
        isError: true,
      };
    }
    if (!text) {
      return {
        content: [{ type: "text", text: "text is required" }],
        isError: true,
      };
    }

    const chat = getChatById(chatId);
    if (!chat) {
      return {
        content: [{ type: "text", text: `Chat not found: ${chatId}` }],
        isError: true,
      };
    }

    const { data, fromCache } = await sendMessage(chatId, text, messageId);
    if (!data) {
      return {
        content: [
          {
            type: "text",
            text: `Failed to reply to message ${messageId} in chat ${chatId}.`,
          },
        ],
        isError: true,
      };
    }

    return {
      content: [
        {
          type: "text",
          text: `Replied to message ${messageId} in chat ${chatId}.`,
        },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "reply_to_message",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.MSG
    );
  }
}
