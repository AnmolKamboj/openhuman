import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { optString } from "../args";
import { updateProfile as updateProfileApi } from "../api/updateProfile";

export const tool: MCPTool = {
  name: "update_profile",
  description: "Update your Telegram profile",
  inputSchema: {
    type: "object",
    properties: {
      first_name: { type: "string", description: "New first name" },
      last_name: { type: "string", description: "New last name" },
      about: { type: "string", description: "New bio/about text" },
    },
  },
};

export async function updateProfile(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const firstName = optString(args, "first_name");
    const lastName = optString(args, "last_name");
    const about = optString(args, "about");

    const { data: updates, fromCache } = await updateProfileApi({
      firstName,
      lastName,
      about,
    });

    return {
      content: [
        { type: "text", text: "Profile updated: " + updates.join(", ") },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "update_profile",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.PROFILE
    );
  }
}
