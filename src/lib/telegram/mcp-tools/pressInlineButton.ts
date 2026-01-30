import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId, validatePositiveInt } from "../../mcp/validation";
import { pressInlineButton as pressInlineButtonApi } from "../api/pressInlineButton";

export const tool: MCPTool = {
  name: "press_inline_button",
  description: "Press an inline button on a message",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
      message_id: { type: "number", description: "Message ID" },
      button_text: { type: "string", description: "Button text or data" },
    },
    required: ["chat_id", "message_id"],
  },
};

export async function pressInlineButton(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const messageId = validatePositiveInt(args.message_id, "message_id");
    const data = typeof args.button_text === "string" ? args.button_text : "";

    if (!data)
      return {
        content: [{ type: "text", text: "button_text is required" }],
        isError: true,
      };

    const { data: answer, fromCache } = await pressInlineButtonApi(
      chatId,
      messageId,
      data
    );

    return {
      content: [{ type: "text", text: answer }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "press_inline_button",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.MSG
    );
  }
}
