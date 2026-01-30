import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { setPrivacySettings as setPrivacySettingsApi } from "../api/setPrivacySettings";

export const tool: MCPTool = {
  name: "set_privacy_settings",
  description: "Set privacy settings",
  inputSchema: {
    type: "object",
    properties: {
      setting: { type: "string", description: "Setting name" },
      value: { type: "string", description: "Value" },
    },
    required: ["setting", "value"],
  },
};

export async function setPrivacySettings(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const keyStr = typeof args.setting === "string" ? args.setting : "";
    const ruleStr = typeof args.value === "string" ? args.value : "";

    const { fromCache } = await setPrivacySettingsApi(keyStr, ruleStr);

    return {
      content: [
        {
          type: "text",
          text: "Privacy setting " + keyStr + " set to " + ruleStr + ".",
        },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "set_privacy_settings",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.PROFILE
    );
  }
}
