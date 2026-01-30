import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { importChatInvite as importChatInviteApi } from "../api/importChatInvite";

export const tool: MCPTool = {
  name: "import_chat_invite",
  description: "Join a chat using an invite hash",
  inputSchema: {
    type: "object",
    properties: {
      hash: {
        type: "string",
        description: "Invite hash (from t.me/+HASH or t.me/joinchat/HASH)",
      },
    },
    required: ["hash"],
  },
};

export async function importChatInvite(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const hash = typeof args.hash === "string" ? args.hash : "";
    if (!hash)
      return {
        content: [{ type: "text", text: "hash is required" }],
        isError: true,
      };

    const { data, fromCache } = await importChatInviteApi(hash);
    return {
      content: [{ type: "text", text: `Joined chat: ${data.chatTitle}` }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "import_chat_invite",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.GROUP
    );
  }
}
