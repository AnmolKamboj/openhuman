import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { getPrivacySettings as getPrivacySettingsApi } from "../api/getPrivacySettings";

export const tool: MCPTool = {
  name: "get_privacy_settings",
  description: "Get privacy settings",
  inputSchema: {
    type: "object",
    properties: {
      key: {
        type: "string",
        description:
          "Privacy key: phone_number, last_seen, profile_photo, forwards, phone_call, chat_invite",
        default: "last_seen",
      },
    },
  },
};

export async function getPrivacySettings(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const keyStr = typeof args.key === "string" ? args.key : "last_seen";

    const { data: settings, fromCache } = await getPrivacySettingsApi(keyStr);

    if (settings.rules.length === 0) {
      return {
        content: [
          {
            type: "text",
            text: "No privacy rules found for " + settings.key + ".",
          },
        ],
        fromCache,
      };
    }

    return {
      content: [
        {
          type: "text",
          text:
            "Privacy settings for " +
            settings.key +
            ":\n" +
            settings.rules.join("\n"),
        },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "get_privacy_settings",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.PROFILE
    );
  }
}
