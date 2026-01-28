import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'add_contact',
  description: 'Add a contact',
  inputSchema: {
    type: 'object',
    properties: {
      first_name: { type: 'string', description: 'First name' },
      last_name: { type: 'string', description: 'Last name' },
      phone_number: { type: 'string', description: 'Phone number' },
    },
    required: ['phone_number'],
  },
};

export async function addContact(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('add_contact');
}
