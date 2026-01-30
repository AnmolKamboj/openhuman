import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { editChatTitle as editChatTitleApi } from "../api/editChatTitle";

export const tool: MCPTool = {
  name: "edit_chat_title",
  description: "Edit the title of a chat",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
      title: { type: "string", description: "New title" },
    },
    required: ["chat_id", "title"],
  },
};

export async function editChatTitle(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const title = typeof args.title === "string" ? args.title : "";
    if (!title)
      return {
        content: [{ type: "text", text: "title is required" }],
        isError: true,
      };

    const { fromCache } = await editChatTitleApi(chatId, title);
    return {
      content: [{ type: "text", text: `Chat title updated to "${title}".` }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "edit_chat_title",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.GROUP
    );
  }
}
