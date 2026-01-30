import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { unmuteChat as unmuteChatApi } from "../api/unmuteChat";

export const tool: MCPTool = {
  name: "unmute_chat",
  description: "Unmute notifications for a chat",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
    },
    required: ["chat_id"],
  },
};

export async function unmuteChat(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const { fromCache } = await unmuteChatApi(chatId);
    return {
      content: [{ type: "text", text: `Chat ${chatId} unmuted.` }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "unmute_chat",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.CHAT
    );
  }
}
