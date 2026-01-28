import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'invite_to_group',
  description: 'Invite users to a group',
  inputSchema: {
    type: 'object',
    properties: {
      chat_id: { type: 'string', description: 'Chat ID or username' },
      user_ids: { type: 'array', description: 'User IDs to invite' },
    },
    required: ['chat_id', 'user_ids'],
  },
};

export async function inviteToGroup(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('invite_to_group');
}
