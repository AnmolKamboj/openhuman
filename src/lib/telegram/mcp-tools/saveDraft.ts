import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { saveDraft as saveDraftApi } from "../api/saveDraft";

export const tool: MCPTool = {
  name: "save_draft",
  description: "Save a draft in a chat",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
      text: { type: "string", description: "Draft text" },
    },
    required: ["chat_id", "text"],
  },
};

export async function saveDraft(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const text = typeof args.text === "string" ? args.text : "";
    if (!text)
      return {
        content: [{ type: "text", text: "text is required" }],
        isError: true,
      };

    const { fromCache } = await saveDraftApi(chatId, text);

    return {
      content: [{ type: "text", text: "Draft saved in chat " + chatId + "." }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "save_draft",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.DRAFT
    );
  }
}
