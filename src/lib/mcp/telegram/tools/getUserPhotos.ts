import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'get_user_photos',
  description: 'Get photos of a user',
  inputSchema: {
    type: 'object',
    properties: {
      user_id: { type: 'string', description: 'User ID' },
      limit: { type: 'number', description: 'Max photos', default: 20 },
    },
    required: ['user_id'],
  },
};

export async function getUserPhotos(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('get_user_photos');
}
