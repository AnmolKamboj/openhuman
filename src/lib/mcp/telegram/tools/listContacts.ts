import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'list_contacts',
  description: 'List all contacts in your Telegram account',
  inputSchema: { type: 'object', properties: {} },
};

export async function listContacts(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('list_contacts');
}
