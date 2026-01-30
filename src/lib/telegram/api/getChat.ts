/**
 * API: Get chat — cache-first with API fallback (hybrid).
 */

import { mtprotoService } from "../services/mtprotoService";
import { enforceRateLimit } from "../../mcp/rateLimiter";
import { getChatById, formatEntity } from "./helpers";
import type { ApiResult } from "./types";
import type { TelegramChat } from "../../../store/telegram/types";
import { updateChatInState } from "../state";

export interface ChatInfo {
  id: string;
  name: string;
  type: string;
  username?: string;
  participantsCount?: number;
  unreadCount?: number;
  phone?: string;
  isBot?: boolean;
  lastMessage?: {
    from: string;
    date: string;
    text: string;
  };
}

export async function getChat(
  chatId: string | number
): Promise<ApiResult<ChatInfo | undefined>> {
  // 1. Try cache
  const chat = getChatById(chatId);
  if (chat) {
    const entity = formatEntity(chat);
    const info: ChatInfo = {
      id: entity.id,
      name: entity.name,
      type: entity.type,
      username: entity.username,
    };
    if ("participantsCount" in chat && chat.participantsCount) {
      info.participantsCount = chat.participantsCount as number;
    }
    if ("unreadCount" in chat) {
      info.unreadCount = (chat.unreadCount as number) ?? 0;
    }
    const lastMsg = (
      chat as {
        lastMessage?: {
          fromName?: string;
          fromId?: string;
          date: number;
          message?: string;
        };
      }
    ).lastMessage;
    if (lastMsg) {
      info.lastMessage = {
        from: lastMsg.fromName ?? lastMsg.fromId ?? "Unknown",
        date: new Date(lastMsg.date * 1000).toISOString(),
        text: lastMsg.message || "[Media/No text]",
      };
    }
    return { data: info, fromCache: true };
  }

  // 2. Cache miss — try Telegram API
  try {
    await enforceRateLimit("__api_fallback_chat");

    const client = mtprotoService.getClient();
    const rawEntity = await mtprotoService.withFloodWaitHandling(async () => {
      return client.getEntity(chatId);
    });

    if (!rawEntity) return { data: undefined, fromCache: false };

    const raw = rawEntity as unknown as Record<string, unknown>;
    const className = raw.className as string | undefined;
    const info: ChatInfo = {
      id: String(raw.id ?? chatId),
      name: "Unknown",
      type: "unknown",
    };

    if (className === "User") {
      info.name =
        [raw.firstName, raw.lastName].filter(Boolean).join(" ") || "Unknown";
      info.type = "user";
      if (raw.username) info.username = String(raw.username);
      if (raw.phone) info.phone = String(raw.phone);
      if (raw.bot) info.isBot = true;
    } else if (className === "Channel") {
      info.name = (raw.title as string) ?? "Unknown";
      info.type = raw.megagroup ? "supergroup" : "channel";
      if (raw.username) info.username = String(raw.username);
      if (raw.participantsCount)
        info.participantsCount = raw.participantsCount as number;
    } else if (className === "Chat") {
      info.name = (raw.title as string) ?? "Unknown";
      info.type = "group";
      if (raw.participantsCount)
        info.participantsCount = raw.participantsCount as number;
    } else {
      info.name =
        (raw.title as string) ?? (raw.firstName as string) ?? "Unknown";
    }

    // Update Redux state with fetched chat
    const chatType: TelegramChat["type"] =
      className === "User"
        ? "private"
        : className === "Channel"
        ? raw.megagroup
          ? "supergroup"
          : "channel"
        : "group";
    updateChatInState({
      id: info.id,
      title: info.name,
      type: chatType,
      username: info.username,
      unreadCount: info.unreadCount ?? 0,
      isPinned: false,
      participantsCount: info.participantsCount,
    });

    return { data: info, fromCache: false };
  } catch {
    return { data: undefined, fromCache: false };
  }
}
