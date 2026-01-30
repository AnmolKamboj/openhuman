import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { unarchiveChat as unarchiveChatApi } from "../api/unarchiveChat";

export const tool: MCPTool = {
  name: "unarchive_chat",
  description: "Unarchive a chat",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
    },
    required: ["chat_id"],
  },
};

export async function unarchiveChat(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const { fromCache } = await unarchiveChatApi(chatId);
    return {
      content: [{ type: "text", text: `Chat ${chatId} unarchived.` }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "unarchive_chat",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.CHAT
    );
  }
}
