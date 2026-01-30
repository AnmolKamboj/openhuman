import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { listTopics as listTopicsApi } from "../api/listTopics";

export const tool: MCPTool = {
  name: "list_topics",
  description: "List topics in a forum chat",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
    },
    required: ["chat_id"],
  },
};

export async function listTopics(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");

    const { data: topics, fromCache } = await listTopicsApi(chatId);

    if (topics.length === 0) {
      return {
        content: [{ type: "text", text: "No forum topics found." }],
        fromCache,
      };
    }

    const lines = topics.map((t) => "ID: " + t.id + " | " + t.title);

    return {
      content: [{ type: "text", text: lines.join("\n") }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "list_topics",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.GROUP
    );
  }
}
