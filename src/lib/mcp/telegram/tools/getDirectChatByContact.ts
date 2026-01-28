import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'get_direct_chat_by_contact',
  description: 'Get direct chat by contact',
  inputSchema: {
    type: 'object',
    properties: { user_id: { type: 'string', description: 'User ID' } },
    required: ['user_id'],
  },
};

export async function getDirectChatByContact(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('get_direct_chat_by_contact');
}
