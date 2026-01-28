import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'join_chat_by_link',
  description: 'Join chat via invite link',
  inputSchema: {
    type: 'object',
    properties: { invite_link: { type: 'string', description: 'Invite link URL' } },
    required: ['invite_link'],
  },
};

export async function joinChatByLink(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('join_chat_by_link');
}
