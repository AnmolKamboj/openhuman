import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'set_profile_photo',
  description: 'Set profile photo',
  inputSchema: { type: 'object', properties: {} },
};

export async function setProfilePhoto(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('set_profile_photo');
}
