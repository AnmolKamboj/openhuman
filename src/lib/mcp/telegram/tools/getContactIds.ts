import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'get_contact_ids',
  description: 'Get contact IDs',
  inputSchema: { type: 'object', properties: {} },
};

export async function getContactIds(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('get_contact_ids');
}
