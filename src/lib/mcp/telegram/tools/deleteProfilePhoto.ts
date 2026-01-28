import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'delete_profile_photo',
  description: 'Delete profile photo',
  inputSchema: { type: 'object', properties: {} },
};

export async function deleteProfilePhoto(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('delete_profile_photo');
}
