import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'remove_reaction',
  description: 'Remove a reaction from a message',
  inputSchema: {
    type: 'object',
    properties: {
      chat_id: { type: 'string', description: 'Chat ID or username' },
      message_id: { type: 'number', description: 'Message ID' },
      reaction: { type: 'string', description: 'Reaction to remove' },
    },
    required: ['chat_id', 'message_id'],
  },
};

export async function removeReaction(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('remove_reaction');
}
