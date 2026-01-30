import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { setProfilePhoto as setProfilePhotoApi } from "../api/setProfilePhoto";

export const tool: MCPTool = {
  name: "set_profile_photo",
  description: "Set profile photo",
  inputSchema: {
    type: "object",
    properties: {
      file_path: { type: "string", description: "Path to the photo file" },
    },
    required: ["file_path"],
  },
};

export async function setProfilePhoto(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const filePath = typeof args.file_path === "string" ? args.file_path : "";
    await setProfilePhotoApi(filePath);

    return {
      content: [
        {
          type: "text",
          text: "This should not be reached",
        },
      ],
    };
  } catch (error) {
    return logAndFormatError(
      "set_profile_photo",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.PROFILE
    );
  }
}
