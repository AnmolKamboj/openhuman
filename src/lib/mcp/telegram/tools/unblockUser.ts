import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'unblock_user',
  description: 'Unblock a user',
  inputSchema: {
    type: 'object',
    properties: { user_id: { type: 'string', description: 'User ID' } },
    required: ['user_id'],
  },
};

export async function unblockUser(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('unblock_user');
}
