import type { MCPTool, MCPToolResult } from "../../types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../errorHandler";
import { validateId } from "../../validation";
import { clearDraft as clearDraftApi } from "../api/clearDraft";

export const tool: MCPTool = {
  name: "clear_draft",
  description: "Clear draft in a chat",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
    },
    required: ["chat_id"],
  },
};

export async function clearDraft(
  args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");

    const { fromCache } = await clearDraftApi(chatId);

    return {
      content: [
        { type: "text", text: "Draft cleared in chat " + chatId + "." },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "clear_draft",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.DRAFT,
    );
  }
}
