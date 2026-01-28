/**
 * List Messages tool - Retrieve messages with optional filters
 */

import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';

import { ErrorCategory, logAndFormatError } from '../../errorHandler';
import { formatMessage, getChatById, getMessages as getMessagesApi } from '../telegramApi';
import { validateId } from '../../validation';

export const tool: MCPTool = {
  name: 'list_messages',
  description: 'Retrieve messages with optional filters',
  inputSchema: {
    type: 'object',
    properties: {
      chat_id: { type: 'string', description: 'The ID or username of the chat to get messages from' },
      limit: { type: 'number', description: 'Maximum number of messages to retrieve', default: 20 },
      search_query: { type: 'string', description: 'Filter messages containing this text' },
      from_date: { type: 'string', description: 'Filter messages from this date (YYYY-MM-DD)' },
      to_date: { type: 'string', description: 'Filter messages until this date (YYYY-MM-DD)' },
    },
    required: ['chat_id'],
  },
};

export async function listMessages(
  args: {
    chat_id: string | number;
    limit?: number;
    search_query?: string;
    from_date?: string;
    to_date?: string;
  },
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, 'chat_id');
    const limit = args.limit ?? 20;

    const chat = getChatById(chatId);
    if (!chat) {
      return {
        content: [{ type: 'text', text: `Chat not found: ${chatId}` }],
        isError: true,
      };
    }

    let messages = await getMessagesApi(chatId, limit * 2, 0);
    if (!messages || messages.length === 0) {
      return { content: [{ type: 'text', text: 'No messages found matching the criteria.' }] };
    }

    if (args.search_query) {
      const q = args.search_query.toLowerCase();
      messages = messages.filter((m) => (m.message ?? '').toLowerCase().includes(q));
    }

    if (args.from_date || args.to_date) {
      messages = messages.filter((m) => {
        const d = new Date(m.date * 1000);
        if (args.from_date && d < new Date(args.from_date)) return false;
        if (args.to_date) {
          const to = new Date(args.to_date);
          to.setHours(23, 59, 59, 999);
          if (d > to) return false;
        }
        return true;
      });
    }

    const sliced = messages.slice(0, limit);
    const contentItems = sliced.map((msg) => {
      const formatted = formatMessage(msg);
      const from = msg.fromName ?? msg.fromId ?? 'Unknown';
      const replyStr = msg.replyToMessageId ? ` | reply to ${msg.replyToMessageId}` : '';
      const text = `ID: ${formatted.id} | ${from} | Date: ${formatted.date}${replyStr} | Message: ${formatted.text || '[Media/No text]'}`;
      return { type: 'text' as const, text };
    });

    return { content: contentItems };
  } catch (error) {
    return logAndFormatError(
      'list_messages',
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.MSG,
    );
  }
}
