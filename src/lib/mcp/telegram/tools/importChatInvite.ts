import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'import_chat_invite',
  description: 'Join chat via invite hash',
  inputSchema: {
    type: 'object',
    properties: { invite_hash: { type: 'string', description: 'Invite hash' } },
    required: ['invite_hash'],
  },
};

export async function importChatInvite(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('import_chat_invite');
}
