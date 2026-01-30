import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { optNumber } from "../args";
import { getParticipants as getParticipantsApi } from "../api/getParticipants";

export const tool: MCPTool = {
  name: "get_participants",
  description: "Get participants of a group or channel",
  inputSchema: {
    type: "object",
    properties: {
      chat_id: { type: "string", description: "Chat ID or username" },
      limit: { type: "number", description: "Max results", default: 50 },
    },
    required: ["chat_id"],
  },
};

export async function getParticipants(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, "chat_id");
    const limit = optNumber(args, "limit", 50);

    const { data: participants, fromCache } = await getParticipantsApi(
      chatId,
      limit
    );

    if (participants.length === 0) {
      return {
        content: [{ type: "text", text: "No participants found." }],
        fromCache,
      };
    }

    const lines = participants.map((p) => {
      const username = p.username ? `@${p.username}` : "";
      return `ID: ${p.id} | ${p.name} ${username}`.trim();
    });

    return {
      content: [
        {
          type: "text",
          text: `${lines.length} participants:\n${lines.join("\n")}`,
        },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "get_participants",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.GROUP
    );
  }
}
