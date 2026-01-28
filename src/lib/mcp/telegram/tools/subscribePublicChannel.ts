import type { MCPTool, MCPToolResult } from "../../types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../errorHandler";
import { mtprotoService } from "../../../../services/mtprotoService";
import { Api } from "telegram";

export const tool: MCPTool = {
  name: "subscribe_public_channel",
  description: "Subscribe to a public channel by username",
  inputSchema: {
    type: "object",
    properties: {
      username: { type: "string", description: "Channel username" },
    },
    required: ["username"],
  },
};

export async function subscribePublicChannel(
  args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  try {
    const username = typeof args.username === "string" ? args.username : "";
    if (!username)
      return {
        content: [{ type: "text", text: "username is required" }],
        isError: true,
      };

    const client = mtprotoService.getClient();

    await mtprotoService.withFloodWaitHandling(async () => {
      const inputChannel = await client.getInputEntity(username);
      await client.invoke(
        new Api.channels.JoinChannel({
          channel: inputChannel as unknown as Api.TypeInputChannel,
        }),
      );
    });

    return {
      content: [{ type: "text", text: `Subscribed to channel: ${username}` }],
    };
  } catch (error) {
    return logAndFormatError(
      "subscribe_public_channel",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.GROUP,
    );
  }
}
