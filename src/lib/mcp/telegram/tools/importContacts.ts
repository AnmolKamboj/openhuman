import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'import_contacts',
  description: 'Import contacts',
  inputSchema: {
    type: 'object',
    properties: { contacts: { type: 'array', description: 'Contacts to import' } },
    required: ['contacts'],
  },
};

export async function importContacts(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('import_contacts');
}
