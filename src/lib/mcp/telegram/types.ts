/**
 * Telegram MCP server types
 */

import type { SocketIOMCPTransportImpl } from '../transport';
import type { MCPToolResult } from '../../mcp/types';
import type { TelegramState } from '../../../store/telegram/types';

export interface TelegramMCPContext {
  telegramState: TelegramState;
  transport: SocketIOMCPTransportImpl;
}

export type TelegramMCPToolHandler = (
  args: Record<string, unknown>,
  context: TelegramMCPContext,
) => Promise<MCPToolResult>;
