import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { optNumber } from "../args";
import { getGifSearch as getGifSearchApi } from "../api/getGifSearch";

export const tool: MCPTool = {
  name: "get_gif_search",
  description: "Search GIFs",
  inputSchema: {
    type: "object",
    properties: {
      query: { type: "string", description: "Search query" },
      limit: { type: "number", description: "Max results", default: 10 },
    },
    required: ["query"],
  },
};

export async function getGifSearch(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const query = typeof args.query === "string" ? args.query : "";
    if (!query)
      return {
        content: [{ type: "text", text: "query is required" }],
        isError: true,
      };
    const limit = optNumber(args, "limit", 10);

    const { data: gifs, fromCache } = await getGifSearchApi(query, limit);

    if (gifs.length === 0) {
      return {
        content: [{ type: "text", text: "No GIFs found for: " + query }],
        fromCache,
      };
    }

    const lines = gifs.map((g, i) => i + 1 + ". " + g.title);

    return {
      content: [
        {
          type: "text",
          text: lines.length + " GIFs found:\n" + lines.join("\n"),
        },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "get_gif_search",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.MEDIA
    );
  }
}
