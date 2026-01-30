/**
 * Factory builder for TelegramMCPContext (used by tool handlers).
 */

import { vi } from "vitest";
import type { TelegramMCPContext } from "../../lib/telegram/types";
import type { TelegramState } from "../../store/telegram/types";
import { initialState } from "../../store/telegram/types";

function createMockTransport() {
  return {
    emit: vi.fn(),
    on: vi.fn(),
    off: vi.fn(),
    updateSocket: vi.fn(),
    connected: true,
  } as unknown as TelegramMCPContext["transport"];
}

export function createMockContext(
  telegramOverrides: Partial<TelegramState> = {}
): TelegramMCPContext {
  return {
    telegramState: { ...initialState, ...telegramOverrides },
    transport: createMockTransport(),
  };
}
