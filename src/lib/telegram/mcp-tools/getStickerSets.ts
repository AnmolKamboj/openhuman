import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { getStickerSets as getStickerSetsApi } from "../api/getStickerSets";

export const tool: MCPTool = {
  name: "get_sticker_sets",
  description: "Get sticker sets",
  inputSchema: { type: "object", properties: {} },
};

export async function getStickerSets(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const { data: sets, fromCache } = await getStickerSetsApi();

    if (sets.length === 0) {
      return {
        content: [{ type: "text", text: "No sticker sets found." }],
        fromCache,
      };
    }

    const lines = sets.map(
      (s) => "ID: " + s.id + " | " + s.title + " (" + s.count + " stickers)"
    );

    return {
      content: [
        {
          type: "text",
          text: lines.length + " sticker sets:\n" + lines.join("\n"),
        },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "get_sticker_sets",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.MEDIA
    );
  }
}
