import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { muteChat as muteChatApi } from "../api/muteChat";

export const tool: MCPTool = {
  name: "mute_chat",
  description: "Mute notifications for a chat",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
      duration: {
        type: "number",
        description: "Mute duration in seconds (0 = forever)",
        default: 0,
      },
    },
    required: ["chat_id"],
  },
};

export async function muteChat(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const duration = typeof args.duration === "number" ? args.duration : 0;
    const { fromCache } = await muteChatApi(chatId, duration);
    return {
      content: [{ type: "text", text: `Chat ${chatId} muted.` }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "mute_chat",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.CHAT
    );
  }
}
