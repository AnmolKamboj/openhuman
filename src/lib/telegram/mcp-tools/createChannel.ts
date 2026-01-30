import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { optString } from "../args";
import { createChannel as createChannelApi } from "../api/createChannel";

export const tool: MCPTool = {
  name: "create_channel",
  description: "Create a new channel",
  inputSchema: {
    type: "object",
    properties: {
      title: { type: "string", description: "Channel title" },
      about: { type: "string", description: "Channel description" },
      megagroup: {
        type: "boolean",
        description: "Create as supergroup instead of channel",
        default: false,
      },
    },
    required: ["title"],
  },
};

export async function createChannel(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const title = typeof args.title === "string" ? args.title : "";
    if (!title)
      return {
        content: [{ type: "text", text: "title is required" }],
        isError: true,
      };

    const about = optString(args, "about") ?? "";
    const megagroup = args.megagroup === true;

    const { data, fromCache } = await createChannelApi(title, about, megagroup);
    return {
      content: [
        {
          type: "text",
          text: `${data.type} "${title}" created. ID: ${data.id}`,
        },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "create_channel",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.GROUP
    );
  }
}
