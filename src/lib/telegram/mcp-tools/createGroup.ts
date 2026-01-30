import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { createGroup as createGroupApi } from "../api/createGroup";

export const tool: MCPTool = {
  name: "create_group",
  description: "Create a new group chat",
  inputSchema: {
    type: "object",
    properties: {
      title: { type: "string", description: "Group title" },
      user_ids: {
        type: "array",
        items: { type: "string" },
        description: "User IDs to add",
      },
    },
    required: ["title", "user_ids"],
  },
};

export async function createGroup(
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

    const userIds = Array.isArray(args.user_ids) ? args.user_ids : [];
    if (userIds.length === 0)
      return {
        content: [{ type: "text", text: "user_ids must not be empty" }],
        isError: true,
      };

    const { data, fromCache } = await createGroupApi(
      title,
      userIds.map(String)
    );
    return {
      content: [
        { type: "text", text: `Group "${title}" created. Chat ID: ${data.id}` },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "create_group",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.GROUP
    );
  }
}
