import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { ErrorCategory, logAndFormatError } from '../../errorHandler';
import { validateId } from '../../validation';
import { getChatById } from '../telegramApi';
import { mtprotoService } from '../../../../services/mtprotoService';
import { Api } from 'telegram';

export const tool: MCPTool = {
  name: 'delete_chat_photo',
  description: 'Delete the photo of a chat',
  inputSchema: {
    type: 'object',
    properties: {
      chat_id: { type: 'string', description: 'Chat ID or username' },
    },
    required: ['chat_id'],
  },
};

export async function deleteChatPhoto(
  args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, 'chat_id');

    const chat = getChatById(chatId);
    if (!chat) return { content: [{ type: 'text', text: `Chat not found: ${chatId}` }], isError: true };

    const client = mtprotoService.getClient();
    const entity = chat.username ? chat.username : chat.id;

    if (chat.type === 'channel' || chat.type === 'supergroup') {
      await mtprotoService.withFloodWaitHandling(async () => {
        const inputChannel = await client.getInputEntity(entity);
        await client.invoke(
          new Api.channels.EditPhoto({
            channel: inputChannel as Api.TypeInputChannel,
            photo: new Api.InputChatPhotoEmpty(),
          }),
        );
      });
    } else {
      await mtprotoService.withFloodWaitHandling(async () => {
        await client.invoke(
          new Api.messages.EditChatPhoto({
            chatId: BigInt(chat.id),
            photo: new Api.InputChatPhotoEmpty(),
          }),
        );
      });
    }

    return { content: [{ type: 'text', text: `Chat photo deleted for ${chatId}.` }] };
  } catch (error) {
    return logAndFormatError(
      'delete_chat_photo',
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.GROUP,
    );
  }
}
