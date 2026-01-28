/**
 * Send Message tool - Send a message to a specific chat
 */

import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';

import { ErrorCategory, logAndFormatError } from '../../errorHandler';
import { sendMessage as sendMessageApi } from '../telegramApi';
import { toHumanReadableAction } from '../toolActionParser';
import { validateId } from '../../validation';

export const tool: MCPTool = {
  name: 'send_message',
  description: 'Send a message to a specific chat',
  inputSchema: {
    type: 'object',
    properties: {
      chat_id: { type: 'string', description: 'The ID or username of the chat' },
      message: { type: 'string', description: 'The message content to send' },
    },
    required: ['chat_id', 'message'],
  },
  toHumanReadableAction: (args) => toHumanReadableAction('send_message', args),
};

export async function sendMessage(
  args: { chat_id: string | number; message: string },
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, 'chat_id');
    const { message } = args;
    if (!message || typeof message !== 'string') {
      return {
        content: [{ type: 'text', text: 'Message content is required' }],
        isError: true,
      };
    }
    const result = await sendMessageApi(chatId, message);
    if (!result) {
      return {
        content: [{ type: 'text', text: `Failed to send message to chat ${chatId}` }],
        isError: true,
      };
    }
    return { content: [{ type: 'text', text: 'Message sent successfully.' }] };
  } catch (error) {
    return logAndFormatError(
      'send_message',
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.MSG,
    );
  }
}
