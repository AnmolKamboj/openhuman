import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { deleteProfilePhoto as deleteProfilePhotoApi } from "../api/deleteProfilePhoto";

export const tool: MCPTool = {
  name: "delete_profile_photo",
  description: "Delete profile photo",
  inputSchema: { type: "object", properties: {} },
};

export async function deleteProfilePhoto(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const { fromCache } = await deleteProfilePhotoApi();

    return {
      content: [{ type: "text", text: "Profile photo deleted." }],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "delete_profile_photo",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.PROFILE
    );
  }
}
