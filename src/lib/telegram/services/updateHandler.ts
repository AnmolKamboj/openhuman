/**
 * Update handler — routes processed Telegram updates to state layer functions.
 *
 * Only handles essential update types:
 * - New/edit/delete messages
 * - Read status changes
 * - Chat metadata updates
 */

import { Api } from "telegram/tl";
import createDebug from "debug";

const log = createDebug("app:telegram:sync");
import {
  addMessageToState,
  updateChatFieldsInState,
  updateMessageFieldsInState,
  findAndDeleteMessagesFromState,
  deleteMessagesFromState,
} from "../state";
import { buildMessage, buildPeerId } from "./entityBuilders";


/**
 * Handle a single update from the UpdateManager.
 * Called for both real-time updates and difference-recovery updates.
 */
export function handleUpdate(
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  update: any,
  userId: string,
  _source: "realtime" | "difference" = "realtime"
): void {
  // -------------------------------------------------------------------------
  // Force sync signal from UpdateManager
  // -------------------------------------------------------------------------
  if (update && update._ === "forceSync") {
    log(
      "Force sync requested %s",
      update.channelId ? `for channel ${update.channelId}` : "(full)"
    );
    return;
  }

  // -------------------------------------------------------------------------
  // New messages
  // -------------------------------------------------------------------------
  if (
    update instanceof Api.UpdateNewMessage ||
    update instanceof Api.UpdateNewChannelMessage
  ) {
    const msg = buildMessage(update.message);
    if (msg) {
      addMessageToState(msg, userId);
      updateChatFieldsInState(
        msg.chatId,
        { lastMessage: msg, lastMessageDate: msg.date },
        userId
      );
    }
    return;
  }

  // -------------------------------------------------------------------------
  // Edited messages
  // -------------------------------------------------------------------------
  if (
    update instanceof Api.UpdateEditMessage ||
    update instanceof Api.UpdateEditChannelMessage
  ) {
    const msg = buildMessage(update.message);
    if (msg) {
      updateMessageFieldsInState(msg.chatId, msg.id, msg, userId);
    }
    return;
  }

  // -------------------------------------------------------------------------
  // Deleted messages
  // -------------------------------------------------------------------------
  if (update instanceof Api.UpdateDeleteMessages) {
    const messageIds = update.messages.map(String);
    findAndDeleteMessagesFromState(messageIds, userId);
    return;
  }

  if (update instanceof Api.UpdateDeleteChannelMessages) {
    const channelId = String(update.channelId);
    const messageIds = update.messages.map(String);
    deleteMessagesFromState(channelId, messageIds, userId);
    return;
  }

  // -------------------------------------------------------------------------
  // Read status (inbox)
  // -------------------------------------------------------------------------
  if (update instanceof Api.UpdateReadHistoryInbox) {
    const chatId = buildPeerId(update.peer);
    if (chatId) {
      updateChatFieldsInState(
        chatId,
        { unreadCount: update.stillUnreadCount },
        userId
      );
    }
    return;
  }

  // Read status (outbox) — no direct UI action needed for outbox reads typically
  if (update instanceof Api.UpdateReadHistoryOutbox) {
    return;
  }

  // Channel read
  if (update instanceof Api.UpdateReadChannelInbox) {
    const chatId = String(update.channelId);
    updateChatFieldsInState(
      chatId,
      { unreadCount: update.stillUnreadCount },
      userId
    );
    return;
  }

  // -------------------------------------------------------------------------
  // Chat/channel metadata updates
  // -------------------------------------------------------------------------
  if (update instanceof Api.UpdateChannel) {
    // The UpdateChannel event only gives us the channel ID.
    // Full re-fetch is handled by sync service if needed.
    return;
  }

  // -------------------------------------------------------------------------
  // User status (online/offline) — could extend TelegramUser in future
  // -------------------------------------------------------------------------
  if (update instanceof Api.UpdateUserStatus) {
    return;
  }

  // -------------------------------------------------------------------------
  // Raw messages from difference (not wrapped in Update*)
  // -------------------------------------------------------------------------
  if (update instanceof Api.Message || update instanceof Api.MessageService) {
    const msg = buildMessage(update);
    if (msg) {
      addMessageToState(msg, userId);
      updateChatFieldsInState(
        msg.chatId,
        { lastMessage: msg, lastMessageDate: msg.date },
        userId
      );
    }
    return;
  }
}
