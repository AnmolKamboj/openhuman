import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { getAdmins as getAdminsApi } from "../api/getAdmins";

export const tool: MCPTool = {
  name: "get_admins",
  description: "Get admins of a group or channel",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
    },
    required: ["chat_id"],
  },
};

export async function getAdmins(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");

    const { data: admins, fromCache } = await getAdminsApi(chatId);

    if (admins.length === 0) {
      return {
        content: [{ type: "text", text: "No admins found." }],
        fromCache,
      };
    }

    const lines = admins.map((a) => {
      const username = a.username ? `@${a.username}` : "";
      return `ID: ${a.id} | ${a.name} ${username}`.trim();
    });

    return {
      content: [
        { type: "text", text: `${lines.length} admins:\n${lines.join("\n")}` },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "get_admins",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.ADMIN
    );
  }
}
