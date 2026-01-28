import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'get_participants',
  description: 'Get participants in a chat',
  inputSchema: {
    type: 'object',
    properties: {
      chat_id: { type: 'string', description: 'Chat ID or username' },
      limit: { type: 'number', description: 'Max participants', default: 100 },
    },
    required: ['chat_id'],
  },
};

export async function getParticipants(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('get_participants');
}
