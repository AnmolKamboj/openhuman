/**
 * API: List inline buttons — cache-only.
 */

import type { ApiResult } from "./types";
import { getChatById, getCachedMessages } from "./helpers";
import { narrow } from "./apiCastHelpers";
import type {
  MessageWithReplyMarkup,
  ReplyMarkupRow,
} from "./apiResultTypes";

export interface InlineButton {
  row: number;
  button: number;
  text: string;
}

export async function listInlineButtons(
  chatId: string | number,
  messageId: number,
): Promise<ApiResult<InlineButton[]>> {
  const chat = getChatById(chatId);
  if (!chat) return { data: [], fromCache: true };

  const messages = getCachedMessages(chatId, 200, 0);
  if (!messages) return { data: [], fromCache: true };

  const found = messages.find((m) => String(m.id) === String(messageId));
  const msg = found ? narrow<MessageWithReplyMarkup>(found) : undefined;
  if (!msg || !msg.replyMarkup || !msg.replyMarkup.rows) {
    return { data: [], fromCache: true };
  }

  const buttons: InlineButton[] = [];
  msg.replyMarkup.rows.forEach((row: ReplyMarkupRow, ri: number) => {
    if (row.buttons) {
      row.buttons.forEach((btn, bi: number) => {
        buttons.push({ row: ri, button: bi, text: btn.text ?? "?" });
      });
    }
  });

  return { data: buttons, fromCache: true };
}
