/**
 * API: Resolve username — API-first with cache fallback (hybrid).
 */

import { mtprotoService } from "../../../../services/mtprotoService";
import { Api } from "telegram";
import { getChatById, formatEntity } from "./helpers";
import type { ApiResult } from "./types";
import { updateChatFromResult, updateUsersFromApiUsers } from "../state";
import type { ApiUser } from "./apiResultTypes";

export interface ResolvedEntity {
  id: string;
  name: string;
  type: string;
  username: string;
}

export async function resolveUsername(
  username: string,
): Promise<ApiResult<ResolvedEntity | undefined>> {
  const clean = username.startsWith("@") ? username.slice(1) : username;

  // 1. Try server-side resolution via Telegram API
  try {
    const client = mtprotoService.getClient();

    const result = await mtprotoService.withFloodWaitHandling(async () => {
      return client.invoke(
        new Api.contacts.ResolveUsername({ username: clean }),
      );
    });

    if (result && "peer" in result) {
      const peer = result.peer;
      const peerType =
        peer.className === "PeerUser"
          ? "user"
          : peer.className === "PeerChannel"
            ? "channel"
            : peer.className === "PeerChat"
              ? "chat"
              : "unknown";

      const peerId =
        "userId" in peer
          ? String(peer.userId)
          : "channelId" in peer
            ? String(peer.channelId)
            : "chatId" in peer
              ? String(peer.chatId)
              : "unknown";

      let name = clean;
      if ("users" in result && Array.isArray(result.users)) {
        const user = result.users[0] as {
          firstName?: string;
          lastName?: string;
        };
        if (user) {
          name =
            [user.firstName, user.lastName].filter(Boolean).join(" ") || clean;
        }
      }
      if ("chats" in result && Array.isArray(result.chats)) {
        const chat = result.chats[0] as { title?: string };
        if (chat?.title) {
          name = chat.title;
        }
      }

      const resolved = { id: peerId, name, type: peerType, username: clean };

      // Update Redux state with resolved entity
      updateChatFromResult(resolved);
      if ("users" in result && Array.isArray(result.users)) {
        updateUsersFromApiUsers(result.users as unknown as ApiUser[]);
      }

      return {
        data: resolved,
        fromCache: false,
      };
    }
  } catch {
    // API call failed — fall back to cached lookup below
  }

  // 2. Fallback: look up from cached state
  const lookupKey = `@${clean}`;
  const chat = getChatById(lookupKey);
  if (!chat) {
    return { data: undefined, fromCache: false };
  }
  const entity = formatEntity(chat);
  return {
    data: {
      id: entity.id,
      name: entity.name,
      type: entity.type,
      username: entity.username ?? clean,
    },
    fromCache: true,
  };
}
