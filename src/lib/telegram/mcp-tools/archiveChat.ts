import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { archiveChat as archiveChatApi } from "../api/archiveChat";

export const tool: MCPTool = {
  name: "archive_chat",
  description: "Archive a chat",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
    },
    required: ["chat_id"],
  },
};

export async function archiveChat(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const { fromCache } = await archiveChatApi(chatId);
    return {
      content: [{ type: "text", text: `Chat ${chatId} archived.` }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "archive_chat",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.CHAT
    );
  }
}
