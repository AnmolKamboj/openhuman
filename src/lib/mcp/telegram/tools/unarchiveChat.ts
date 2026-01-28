import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'unarchive_chat',
  description: 'Unarchive a chat',
  inputSchema: {
    type: 'object',
    properties: { chat_id: { type: 'string', description: 'Chat ID or username' } },
    required: ['chat_id'],
  },
};

export async function unarchiveChat(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('unarchive_chat');
}
