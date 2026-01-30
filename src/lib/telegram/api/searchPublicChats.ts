/**
 * Search Public Chats API - Search for public chats, channels, or bots
 *
 * Uses Telegram's contacts.Search API for server-side discovery.
 * Falls back to filtering cached chats if the API call fails.
 */

import { mtprotoService } from "../../../services/mtprotoService";
import { searchChatsInCache } from "./helpers";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import { updateChatsFromResults, updateUsersFromApiUsers } from "../state";
import type { ApiUser } from "./apiResultTypes";

export interface PublicChatResult {
  id: string;
  name: string;
  type: string;
  username?: string;
}

export async function searchPublicChats(
  query: string
): Promise<ApiResult<PublicChatResult[]>> {
  // Try server-side search via Telegram API
  try {
    const client = mtprotoService.getClient();

    const result = await mtprotoService.withFloodWaitHandling(async () => {
      return client.invoke(new Api.contacts.Search({ q: query, limit: 20 }));
    });

    const entries: PublicChatResult[] = [];

    // Process returned chats
    if ("chats" in result && Array.isArray(result.chats)) {
      for (const chat of result.chats) {
        const c = chat as {
          id: { toString(): string };
          title?: string;
          username?: string;
          megagroup?: boolean;
          broadcast?: boolean;
        };
        const type = c.broadcast ? "channel" : c.megagroup ? "group" : "chat";
        entries.push({
          id: String(c.id),
          name: c.title ?? "Unknown",
          type,
          username: c.username,
        });
      }
    }

    // Process returned users
    if ("users" in result && Array.isArray(result.users)) {
      for (const user of result.users) {
        const u = user as {
          id: { toString(): string };
          firstName?: string;
          lastName?: string;
          username?: string;
          bot?: boolean;
        };
        const name =
          [u.firstName, u.lastName].filter(Boolean).join(" ") || "Unknown";
        entries.push({
          id: String(u.id),
          name,
          type: u.bot ? "bot" : "user",
          username: u.username,
        });
      }
    }

    if (entries.length > 0) {
      // Update Redux state with discovered chats and users
      updateChatsFromResults(entries);
      if ("users" in result && Array.isArray(result.users)) {
        updateUsersFromApiUsers(result.users as unknown as ApiUser[]);
      }
      return { data: entries, fromCache: false };
    }
  } catch {
    // API call failed — fall back to cached search
  }

  // Fallback: search cached chats locally
  const chats = searchChatsInCache(query);
  return {
    data: chats.map((c) => ({
      id: c.id,
      name: c.title ?? "Unknown",
      type: c.type ?? "chat",
      username: c.username,
    })),
    fromCache: true,
  };
}
