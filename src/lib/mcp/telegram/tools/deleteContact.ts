import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'delete_contact',
  description: 'Delete a contact',
  inputSchema: {
    type: 'object',
    properties: { user_id: { type: 'string', description: 'User ID' } },
    required: ['user_id'],
  },
};

export async function deleteContact(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('delete_contact');
}
