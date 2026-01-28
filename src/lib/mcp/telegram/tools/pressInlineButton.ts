import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'press_inline_button',
  description: 'Press an inline button on a message',
  inputSchema: {
    type: 'object',
    properties: {
      chat_id: { type: 'string', description: 'Chat ID or username' },
      message_id: { type: 'number', description: 'Message ID' },
      button_text: { type: 'string', description: 'Button text or data' },
    },
    required: ['chat_id', 'message_id'],
  },
};

export async function pressInlineButton(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('press_inline_button');
}
