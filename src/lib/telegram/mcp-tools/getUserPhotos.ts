import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { validateId } from "../../mcp/validation";
import { optNumber } from "../args";
import { getUserPhotos as getUserPhotosApi } from "../api/getUserPhotos";

export const tool: MCPTool = {
  name: "get_user_photos",
  description: "Get profile photos of a user",
  inputSchema: {
    type: "object",
    properties: {
      user_id: { type: "string", description: "User ID" },
      limit: { type: "number", description: "Max photos", default: 10 },
    },
    required: ["user_id"],
  },
};

export async function getUserPhotos(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const userId = validateId(args.user_id, "user_id");
    const limit = optNumber(args, "limit", 10);

    const { data: photos, fromCache } = await getUserPhotosApi(userId, limit);

    if (photos.length === 0) {
      return {
        content: [{ type: "text", text: "No photos found." }],
        fromCache,
      };
    }

    const lines = photos.map((photo, i: number) => {
      return "Photo " + (i + 1) + ": ID " + photo.id + " | Date: " + photo.date;
    });

    return {
      content: [
        {
          type: "text",
          text: lines.length + " photos found:\n" + lines.join("\n"),
        },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "get_user_photos",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.PROFILE
    );
  }
}
