import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'subscribe_public_channel',
  description: 'Subscribe to public channel by username',
  inputSchema: {
    type: 'object',
    properties: { username: { type: 'string', description: 'Channel username' } },
    required: ['username'],
  },
};

export async function subscribePublicChannel(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('subscribe_public_channel');
}
