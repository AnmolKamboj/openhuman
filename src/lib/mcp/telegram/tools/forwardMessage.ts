import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'forward_message',
  description: 'Forward a message to another chat',
  inputSchema: {
    type: 'object',
    properties: {
      from_chat_id: { type: 'string', description: 'Source chat ID' },
      to_chat_id: { type: 'string', description: 'Target chat ID' },
      message_id: { type: 'number', description: 'Message ID' },
    },
    required: ['from_chat_id', 'to_chat_id', 'message_id'],
  },
};

export async function forwardMessage(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('forward_message');
}
