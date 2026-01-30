/**
 * API: Get current user — cache-first with API fallback.
 */

import type { TelegramUser } from "../../../store/telegram/types";
import { mtprotoService } from "../../../services/mtprotoService";
import { enforceRateLimit } from "../../mcp/rateLimiter";
import { getCurrentUser as getCachedCurrentUser } from "./helpers";
import type { ApiResult } from "./types";
import { updateCurrentUserInState } from "../state";

export async function getCurrentUser(): Promise<
  ApiResult<TelegramUser | undefined>
> {
  // 1. Try cache
  const cached = getCachedCurrentUser();
  if (cached) return { data: cached, fromCache: true };

  // 2. Rate limit before API call
  try {
    await enforceRateLimit("__api_fallback_me");
  } catch {
    return { data: undefined, fromCache: false };
  }

  // 3. Fetch from Telegram API
  try {
    const client = mtprotoService.getClient();
    const me = await mtprotoService.withFloodWaitHandling(async () => {
      return client.getMe();
    });

    if (!me) return { data: undefined, fromCache: false };

    const raw = me as unknown as {
      id?: unknown;
      firstName?: string;
      lastName?: string;
      username?: string;
      phone?: string;
      bot?: boolean;
    };

    const userData = {
      id: String(raw.id ?? ""),
      firstName: raw.firstName ?? "",
      lastName: raw.lastName,
      username: raw.username,
      phoneNumber: raw.phone,
      isBot: Boolean(raw.bot),
    };

    // Update Redux state with fetched current user
    updateCurrentUserInState(userData);

    return {
      data: userData,
      fromCache: false,
    };
  } catch {
    return { data: undefined, fromCache: false };
  }
}
