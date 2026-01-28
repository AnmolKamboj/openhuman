import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { notImplemented } from './notImplemented';

export const tool: MCPTool = {
  name: 'get_sticker_sets',
  description: 'Get sticker sets',
  inputSchema: { type: 'object', properties: {} },
};

export async function getStickerSets(
  _args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  return notImplemented('get_sticker_sets');
}
