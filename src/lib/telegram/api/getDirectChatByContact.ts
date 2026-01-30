/**
 * API: Get direct chat by contact — cache-only.
 */

import { store } from "../../../store";
import { selectOrderedChats } from "../../../store/telegramSelectors";
import type { ApiResult } from "./types";

export interface DirectChat {
  id: string;
  title: string;
  username?: string;
}

export async function getDirectChatByContact(
  userId: string | number
): Promise<ApiResult<DirectChat | undefined>> {
  const state = store.getState();
  const chats = selectOrderedChats(state);

  const dmChat = chats.find(
    (c) => c.type === "private" && String(c.id) === String(userId)
  );

  if (!dmChat) return { data: undefined, fromCache: true };

  return {
    data: {
      id: dmChat.id,
      title: dmChat.title ?? "DM",
      username: dmChat.username,
    },
    fromCache: true,
  };
}
