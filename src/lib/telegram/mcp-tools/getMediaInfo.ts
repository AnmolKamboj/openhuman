import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { getMediaInfo as getMediaInfoApi } from "../api/getMediaInfo";

export const tool: MCPTool = {
  name: "get_media_info",
  description: "Get media info from a message",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
      message_id: { type: "number", description: "Message ID" },
    },
    required: ["chat_id", "message_id"],
  },
};

export async function getMediaInfo(
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

    const { data: info, fromCache } = await getMediaInfoApi(chatId, messageId);

    if (info.type === "none") {
      return {
        content: [
          { type: "text", text: "No media in message " + messageId + "." },
        ],
        fromCache,
      };
    }

    return {
      content: [{ type: "text", text: JSON.stringify(info, null, 2) }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "get_media_info",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.MEDIA
    );
  }
}
