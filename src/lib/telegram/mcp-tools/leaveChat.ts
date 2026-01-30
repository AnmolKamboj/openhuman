import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { getChatById } from "../api/helpers";
import { leaveChat as leaveChatApi } from "../api/leaveChat";

export const tool: MCPTool = {
  name: "leave_chat",
  description: "Leave a group or channel",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
    },
    required: ["chat_id"],
  },
};

export async function leaveChat(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const chat = getChatById(chatId);
    const { fromCache } = await leaveChatApi(chatId);
    return {
      content: [{ type: "text", text: `Left chat ${chat?.title ?? chatId}.` }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "leave_chat",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.GROUP
    );
  }
}
