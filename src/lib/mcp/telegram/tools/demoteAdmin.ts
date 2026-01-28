import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'demote_admin',
  description: 'Demote admin',
  inputSchema: {
    type: 'object',
    properties: {
      chat_id: { type: 'string', description: 'Chat ID or username' },
      user_id: { type: 'string', description: 'User ID' },
    },
    required: ['chat_id', 'user_id'],
  },
};

export async function demoteAdmin(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('demote_admin');
}
