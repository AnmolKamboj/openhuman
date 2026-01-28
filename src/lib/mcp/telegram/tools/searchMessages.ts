/**
 * Search Messages tool - Search for messages in a chat by text
 */

import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';

import { ErrorCategory, logAndFormatError } from '../../errorHandler';
import { formatMessage, getChatById, getMessages } from '../telegramApi';
import { validateId } from '../../validation';

export const tool: MCPTool = {
  name: 'search_messages',
  description: 'Search for messages in a chat by text',
  inputSchema: {
    type: 'object',
    properties: {
      chat_id: { type: 'string', description: 'The chat ID or username' },
      query: { type: 'string', description: 'Search query' },
      limit: { type: 'number', description: 'Maximum number of messages to return', default: 20 },
    },
    required: ['chat_id', 'query'],
  },
};

export async function searchMessages(
  args: { chat_id: string | number; query: string; limit?: number },
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, 'chat_id');
    const { query, limit = 20 } = args;

    const chat = getChatById(chatId);
    if (!chat) {
      return {
        content: [{ type: 'text', text: `Chat ${chatId} not found` }],
        isError: true,
      };
    }

    const messages = await getMessages(chatId, Math.min(limit * 3, 100), 0);
    if (!messages || messages.length === 0) {
      return { content: [{ type: 'text', text: 'No messages found.' }] };
    }

    const q = query.toLowerCase();
    const filtered = messages.filter((m) => (m.message ?? '').toLowerCase().includes(q)).slice(0, limit);

    const lines = filtered.map((m) => {
      const f = formatMessage(m);
      return `ID: ${f.id} | ${m.fromName ?? m.fromId ?? 'Unknown'} | ${f.date} | ${f.text || '[Media]'}`;
    });

    return { content: [{ type: 'text', text: lines.join('\n') }] };
  } catch (error) {
    return logAndFormatError(
      'search_messages',
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.SEARCH,
    );
  }
}
