import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'edit_chat_title',
  description: 'Edit chat title',
  inputSchema: {
    type: 'object',
    properties: {
      chat_id: { type: 'string', description: 'Chat ID or username' },
      new_title: { type: 'string', description: 'New title' },
    },
    required: ['chat_id', 'new_title'],
  },
};

export async function editChatTitle(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('edit_chat_title');
}
