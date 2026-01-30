import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import type { BotCommandInput } from "../api/apiResultTypes";
import { setBotCommands as setBotCommandsApi } from "../api/setBotCommands";

export const tool: MCPTool = {
  name: "set_bot_commands",
  description: "Set bot commands",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
      commands: { type: "array", description: "List of commands" },
    },
    required: ["commands"],
  },
};

export async function setBotCommands(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const cmds = Array.isArray(args.commands) ? args.commands : [];

    const { data: count, fromCache } = await setBotCommandsApi(
      cmds as BotCommandInput[]
    );

    return {
      content: [
        {
          type: "text",
          text: "Bot commands updated: " + count + " commands.",
        },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "set_bot_commands",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.PROFILE
    );
  }
}
