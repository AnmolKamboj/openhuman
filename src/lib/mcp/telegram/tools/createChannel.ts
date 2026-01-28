import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'create_channel',
  description: 'Create a channel',
  inputSchema: {
    type: 'object',
    properties: {
      title: { type: 'string', description: 'Channel title' },
      description: { type: 'string', description: 'Channel description' },
    },
    required: ['title'],
  },
};

export async function createChannel(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('create_channel');
}
