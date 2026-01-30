import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { getBotInfo as getBotInfoApi } from "../api/getBotInfo";

export const tool: MCPTool = {
  name: "get_bot_info",
  description: "Get bot information in a chat",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
    },
    required: ["chat_id"],
  },
};

export async function getBotInfo(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const botId = validateId(args.chat_id, "chat_id");

    const { data: botInfo, fromCache } = await getBotInfoApi(botId);

    const lines = [
      "Name: " + botInfo.name,
      "Username: @" + botInfo.username,
      "ID: " + botInfo.id,
      "Bot: " + (botInfo.isBot ? "Yes" : "No"),
      "About: " + (botInfo.about ?? "N/A"),
      "Bot Info Description: " + (botInfo.botDescription ?? "N/A"),
    ];

    if (botInfo.commands && botInfo.commands.length > 0) {
      lines.push("Commands:");
      for (const cmd of botInfo.commands) {
        lines.push("  /" + cmd.command + " - " + cmd.description);
      }
    }

    return {
      content: [{ type: "text", text: lines.join("\n") }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "get_bot_info",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.CONTACT
    );
  }
}
