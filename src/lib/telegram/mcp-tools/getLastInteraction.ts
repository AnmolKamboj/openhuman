import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { getLastInteraction as getLastInteractionApi } from "../api/getLastInteraction";

export const tool: MCPTool = {
  name: "get_last_interaction",
  description: "Get the last message exchanged with a user or chat",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
    },
    required: ["chat_id"],
  },
};

export async function getLastInteraction(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const { data: interaction, fromCache } = await getLastInteractionApi(
      chatId
    );

    if (!interaction) {
      return {
        content: [{ type: "text", text: "No messages found in this chat." }],
        fromCache,
      };
    }

    return {
      content: [
        {
          type: "text",
          text:
            "Last message in " +
            interaction.chatTitle +
            ":\nFrom: " +
            interaction.from +
            " | Date: " +
            interaction.date +
            " | " +
            interaction.text,
        },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "get_last_interaction",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.CONTACT
    );
  }
}
