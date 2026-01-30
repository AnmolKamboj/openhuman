import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { listInlineButtons as listInlineButtonsApi } from "../api/listInlineButtons";

export const tool: MCPTool = {
  name: "list_inline_buttons",
  description: "List inline buttons on a message",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
      message_id: { type: "number", description: "Message ID" },
    },
    required: ["chat_id", "message_id"],
  },
};

export async function listInlineButtons(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const messageId =
      typeof args.message_id === "number" && Number.isInteger(args.message_id)
        ? args.message_id
        : undefined;

    if (messageId === undefined) {
      return {
        content: [
          { type: "text", text: "message_id must be a positive integer" },
        ],
        isError: true,
      };
    }

    const { data: buttons, fromCache } = await listInlineButtonsApi(
      chatId,
      messageId
    );

    if (buttons.length === 0) {
      return {
        content: [
          {
            type: "text",
            text: "No inline buttons on message " + messageId + ".",
          },
        ],
        fromCache,
      };
    }

    const lines = buttons.map(
      (b) => "Row " + b.row + ", Button " + b.button + ': "' + b.text + '"'
    );

    return { content: [{ type: "text", text: lines.join("\n") }], fromCache };
  } catch (error) {
    return logAndFormatError(
      "list_inline_buttons",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.MSG
    );
  }
}
