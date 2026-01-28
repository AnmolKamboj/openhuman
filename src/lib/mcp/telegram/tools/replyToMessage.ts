/**
 * Reply To Message tool - Reply to a specific message in a chat
 */

import type { MCPTool, MCPToolResult } from "../../types";
import type { TelegramMCPContext } from "../types";

import { ErrorCategory, logAndFormatError } from "../../errorHandler";
import { getChatById, sendMessage } from "../telegramApi";
import { toHumanReadableAction } from "../toolActionParser";
import { validateId } from "../../validation";

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
  args: { chat_id: string | number; message_id: number; text: string },
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const { message_id, text } = args;

    const chat = getChatById(chatId);
    if (!chat) {
      return {
        content: [{ type: "text", text: `Chat not found: ${chatId}` }],
        isError: true,
      };
    }

    const result = await sendMessage(chatId, text, message_id);
    if (!result) {
      return {
        content: [
          {
            type: "text",
            text: `Failed to reply to message ${message_id} in chat ${chatId}.`,
          },
        ],
        isError: true,
      };
    }

    return {
      content: [
        {
          type: "text",
          text: `Replied to message ${message_id} in chat ${chatId}.`,
        },
      ],
    };
  } catch (error) {
    return logAndFormatError(
      "reply_to_message",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.MSG,
    );
  }
}
