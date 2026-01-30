/**
 * Send Message tool - Send a message to a specific chat
 */

import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";

import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { sendMessage as sendMessageApiCall } from "../api/sendMessage";
import { toHumanReadableAction } from "../toolActionParser";
import { validateId } from "../../mcp/validation";

export const tool: MCPTool = {
  name: "send_message",
  description: "Send a message to a specific chat",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: {
        type: "string",
        description: "The ID or username of the chat",
      },
      message: { type: "string", description: "The message content to send" },
    },
    required: ["chat_id", "message"],
  },
  toHumanReadableAction: (args) => toHumanReadableAction("send_message", args),
};

export async function sendMessage(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const message = typeof args.message === "string" ? args.message : "";
    if (!message) {
      return {
        content: [{ type: "text", text: "Message content is required" }],
        isError: true,
      };
    }
    const { data: result, fromCache } = await sendMessageApiCall(
      chatId,
      message
    );
    if (!result) {
      return {
        content: [
          { type: "text", text: `Failed to send message to chat ${chatId}` },
        ],
        isError: true,
      };
    }
    return {
      content: [{ type: "text", text: "Message sent successfully." }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "send_message",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.MSG
    );
  }
}
