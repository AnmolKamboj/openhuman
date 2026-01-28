import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'create_group',
  description: 'Create a group',
  inputSchema: {
    type: 'object',
    properties: {
      title: { type: 'string', description: 'Group title' },
      user_ids: { type: 'array', description: 'User IDs to add' },
    },
    required: ['title'],
  },
};

export async function createGroup(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('create_group');
}
