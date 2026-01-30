/**
 * Resolve Username tool - Resolve a username to a user or chat ID
 *
 * Uses Telegram's contacts.ResolveUsername API for server-side resolution.
 * Falls back to cached chat lookup if the API call fails.
 */

import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";

import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { resolveUsername as resolveUsernameApi } from "../api/resolveUsername";

export const tool: MCPTool = {
  name: "resolve_username",
  description: "Resolve a username to a user or chat ID",
  inputSchema: {
    type: "object",
    properties: {
      username: {
        type: "string",
        description: "Username to resolve (without @)",
      },
    },
    required: ["username"],
  },
};

export async function resolveUsername(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const raw = typeof args.username === "string" ? args.username : "";
    if (!raw) {
      return {
        content: [{ type: "text", text: "username is required" }],
        isError: true,
      };
    }

    const { data: resolved, fromCache } = await resolveUsernameApi(raw);

    if (!resolved) {
      const username = raw.startsWith("@") ? raw : `@${raw}`;
      return {
        content: [{ type: "text", text: `Username ${username} not found` }],
        isError: true,
      };
    }

    return {
      content: [
        {
          type: "text",
          text: JSON.stringify(resolved, undefined, 2),
        },
      ],
      fromCache,
    };
  } catch (error) {
    return logAndFormatError(
      "resolve_username",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.SEARCH
    );
  }
}
