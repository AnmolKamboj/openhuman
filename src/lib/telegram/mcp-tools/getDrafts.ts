import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { getDrafts as getDraftsApi } from "../api/getDrafts";

export const tool: MCPTool = {
  name: "get_drafts",
  description: "Get all drafts",
  inputSchema: { type: "object", properties: {} },
};

export async function getDrafts(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const { data: drafts, fromCache } = await getDraftsApi();

    if (drafts.length === 0) {
      return {
        content: [{ type: "text", text: "No drafts found." }],
        fromCache,
      };
    }

    const lines = drafts.map((d) => "Peer " + d.peerId + ": " + d.message);

    return {
      content: [{ type: "text", text: lines.join("\n") }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "get_drafts",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.DRAFT
    );
  }
}
