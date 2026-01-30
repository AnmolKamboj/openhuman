/**
 * Get Chat tool - Get detailed information about a specific chat
 *
 * Tries cached Redux state first, falls back to Telegram API when cache misses.
 */

import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";

import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { getChat as getChatApi } from "../api/getChat";

export const tool: MCPTool = {
  name: "get_chat",
  description: "Get detailed information about a specific chat",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: {
        type: "string",
        description: "The ID or username of the chat",
      },
    },
    required: ["chat_id"],
  },
};

export async function getChat(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const { data: chatInfo, fromCache } = await getChatApi(chatId);

    if (!chatInfo) {
      return {
        content: [{ type: "text", text: `Chat not found: ${chatId}` }],
        isError: true,
      };
    }

    const result: string[] = [];
    result.push(`ID: ${chatInfo.id}`);
    if (chatInfo.type === "user") {
      result.push(`Name: ${chatInfo.name}`);
    } else {
      result.push(`Title: ${chatInfo.name}`);
    }
    result.push(`Type: ${chatInfo.type}`);
    if (chatInfo.username) result.push(`Username: @${chatInfo.username}`);
    if (chatInfo.phone) result.push(`Phone: +${chatInfo.phone}`);
    if (chatInfo.isBot) result.push(`Bot: true`);
    if (chatInfo.participantsCount)
      result.push(`Participants: ${chatInfo.participantsCount}`);
    if (chatInfo.unreadCount !== undefined)
      result.push(`Unread Messages: ${chatInfo.unreadCount}`);
    if (chatInfo.lastMessage) {
      result.push(
        `Last Message: From ${chatInfo.lastMessage.from} at ${chatInfo.lastMessage.date}`
      );
      result.push(`Message: ${chatInfo.lastMessage.text}`);
    }

    return {
      content: [{ type: "text", text: result.join("\n") }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "get_chat",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.CHAT
    );
  }
}
