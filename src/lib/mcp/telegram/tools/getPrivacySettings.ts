import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'get_privacy_settings',
  description: 'Get privacy settings',
  inputSchema: { type: 'object', properties: {} },
};

export async function getPrivacySettings(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('get_privacy_settings');
}
