/**
 * API: Get contact chats — cache-only.
 */

import { store } from "../../../store";
import { selectOrderedChats } from "../../../store/telegramSelectors";
import type { ApiResult } from "./types";

export interface ContactChat {
  id: string;
  title: string;
  username?: string;
}

export async function getContactChats(): Promise<ApiResult<ContactChat[]>> {
  const state = store.getState();
  const chats = selectOrderedChats(state);
  const dmChats = chats
    .filter((c) => c.type === "private")
    .map((c) => ({
      id: c.id,
      title: c.title ?? "DM",
      username: c.username,
    }));

  return { data: dmChats, fromCache: true };
}
