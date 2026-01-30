import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { optString } from "../args";
import { exportChatInvite as exportChatInviteApi } from "../api/exportChatInvite";

export const tool: MCPTool = {
  name: "export_chat_invite",
  description: "Export a new chat invite link",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
      title: { type: "string", description: "Link title" },
      expire_date: { type: "number", description: "Expiration timestamp" },
      usage_limit: { type: "number", description: "Max number of uses" },
    },
    required: ["chat_id"],
  },
};

export async function exportChatInvite(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const title = optString(args, "title");
    const expireDate =
      typeof args.expire_date === "number" ? args.expire_date : undefined;
    const usageLimit =
      typeof args.usage_limit === "number" ? args.usage_limit : undefined;

    const { data, fromCache } = await exportChatInviteApi(
      chatId,
      title,
      expireDate,
      usageLimit
    );
    return {
      content: [{ type: "text", text: `Invite link created: ${data.link}` }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "export_chat_invite",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.GROUP
    );
  }
}
