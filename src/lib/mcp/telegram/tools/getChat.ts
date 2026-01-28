/**
 * Get Chat tool - Get detailed information about a specific chat
 */

import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';

import { ErrorCategory, logAndFormatError } from '../../errorHandler';
import { formatEntity, getChatById } from '../telegramApi';
import { validateId } from '../../validation';

export const tool: MCPTool = {
  name: 'get_chat',
  description: 'Get detailed information about a specific chat',
  inputSchema: {
    type: 'object',
    properties: {
      chat_id: {
        type: 'string',
        description: 'The ID or username of the chat',
      },
    },
    required: ['chat_id'],
  },
};

export async function getChat(
  args: { chat_id: string | number },
  context: TelegramMCPContext,
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, 'chat_id');
    const chat = getChatById(chatId);

    if (!chat) {
      return {
        content: [{ type: 'text', text: `Chat not found: ${chatId}` }],
        isError: true,
      };
    }

    const entity = formatEntity(chat);
    const result: string[] = [];

    result.push(`ID: ${entity.id}`);
    result.push(`Title: ${entity.name}`);
    result.push(`Type: ${entity.type}`);
    if (entity.username) result.push(`Username: @${entity.username}`);
    if ('participantsCount' in chat && chat.participantsCount) {
      result.push(`Participants: ${chat.participantsCount}`);
    }
    if ('unreadCount' in chat) {
      result.push(`Unread Messages: ${chat.unreadCount ?? 0}`);
    }

    const lastMsg = chat.lastMessage;
    if (lastMsg) {
      const from = lastMsg.fromName ?? lastMsg.fromId ?? 'Unknown';
      const date = new Date(lastMsg.date * 1000).toISOString();
      result.push(`Last Message: From ${from} at ${date}`);
      result.push(`Message: ${lastMsg.message || '[Media/No text]'}`);
    }

    return {
      content: [{ type: 'text', text: result.join('\n') }],
    };
  } catch (error) {
    return logAndFormatError(
      'get_chat',
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.CHAT,
    );
  }
}
