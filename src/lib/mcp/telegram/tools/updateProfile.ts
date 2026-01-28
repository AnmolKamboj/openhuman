import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'update_profile',
  description: 'Update your profile',
  inputSchema: {
    type: 'object',
    properties: {
      first_name: { type: 'string', description: 'First name' },
      last_name: { type: 'string', description: 'Last name' },
      bio: { type: 'string', description: 'Bio' },
    },
  },
};

export async function updateProfile(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('update_profile');
}
