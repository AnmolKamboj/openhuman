import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'get_blocked_users',
  description: 'Get list of blocked users',
  inputSchema: { type: 'object', properties: {} },
};

export async function getBlockedUsers(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('get_blocked_users');
}
