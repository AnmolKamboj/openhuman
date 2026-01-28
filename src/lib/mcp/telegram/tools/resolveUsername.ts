/**
 * Resolve Username tool - Resolve a username to a user or chat ID
 */

import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';

import { ErrorCategory, logAndFormatError } from '../../errorHandler';
import { formatEntity, getChatById } from '../telegramApi';

export const tool: MCPTool = {
  name: 'resolve_username',
  description: 'Resolve a username to a user or chat ID',
  inputSchema: {
    type: 'object',
    properties: {
      username: { type: 'string', description: 'Username to resolve (without @)' },
    },
    required: ['username'],
  },
};

export async function resolveUsername(
  args: { username: string },
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  try {
    const raw = args.username;
    const username = raw.startsWith('@') ? raw : `@${raw}`;
    const chat = getChatById(username);
    if (!chat) {
      return {
        content: [{ type: 'text', text: `Username ${username} not found` }],
        isError: true,
      };
    }
    const entity = formatEntity(chat);
    return {
      content: [
        {
          type: 'text',
          text: JSON.stringify({ id: entity.id, name: entity.name, type: entity.type, username: entity.username }, undefined, 2),
        },
      ],
    };
  } catch (error) {
    return logAndFormatError(
      'resolve_username',
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.SEARCH,
    );
  }
}
