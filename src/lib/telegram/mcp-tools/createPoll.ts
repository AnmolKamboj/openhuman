import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { createPoll as createPollApi } from "../api/createPoll";

export const tool: MCPTool = {
  name: "create_poll",
  description: "Create a poll in a chat",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
      question: { type: "string", description: "Poll question" },
      options: { type: "array", description: "Poll options" },
    },
    required: ["chat_id", "question", "options"],
  },
};

export async function createPoll(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const question = typeof args.question === "string" ? args.question : "";
    const options = Array.isArray(args.options) ? args.options.map(String) : [];

    if (!question)
      return {
        content: [{ type: "text", text: "question is required" }],
        isError: true,
      };
    if (options.length < 2)
      return {
        content: [{ type: "text", text: "At least 2 options are required" }],
        isError: true,
      };

    const { fromCache } = await createPollApi(chatId, question, options);

    return {
      content: [{ type: "text", text: "Poll created: " + question }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "create_poll",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.MSG
    );
  }
}
