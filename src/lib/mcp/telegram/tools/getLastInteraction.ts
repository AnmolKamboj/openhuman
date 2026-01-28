import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'get_last_interaction',
  description: 'Get last interaction with a user',
  inputSchema: {
    type: 'object',
    properties: { user_id: { type: 'string', description: 'User ID' } },
    required: ['user_id'],
  },
};

export async function getLastInteraction(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('get_last_interaction');
}
