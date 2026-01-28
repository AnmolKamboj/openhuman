import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'create_poll',
  description: 'Create a poll in a chat',
  inputSchema: {
    type: 'object',
    properties: {
      chat_id: { type: 'string', description: 'Chat ID or username' },
      question: { type: 'string', description: 'Poll question' },
      options: { type: 'array', description: 'Poll options' },
    },
    required: ['chat_id', 'question', 'options'],
  },
};

export async function createPoll(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('create_poll');
}
