import { describe, it, expect, vi, beforeEach } from "vitest";
import { getChats } from "../getChats";
import type { TelegramChat } from "../../../../store/telegram/types";

// Mock dependencies
const mockClient = {
  invoke: vi.fn(),
  getInputEntity: vi.fn(),
  getMe: vi.fn(),
  sendMessage: vi.fn(),
};

vi.mock("../../services/mtprotoService", () => ({
  mtprotoService: {
    getClient: vi.fn(() => mockClient),
    withFloodWaitHandling: vi.fn(async (fn: any) => await fn()),
    isReady: vi.fn(() => true),
    isClientConnected: vi.fn(() => true),
    sendMessage: vi.fn(),
  },
}));

vi.mock("../../../../store", () => ({
  store: {
    getState: vi.fn(() => ({
      user: { user: { _id: "u1" } },
      telegram: {
        byUser: {
          u1: {
            chats: {},
            chatsOrder: [],
            messages: {},
            messagesOrder: {},
            currentUser: null,
          },
        },
      },
    })),
  },
}));

vi.mock("../../../../store/telegramSelectors", () => ({
  selectTelegramUserState: vi.fn(() => ({
    chats: {},
    chatsOrder: [],
    messages: {},
    messagesOrder: {},
    currentUser: null,
  })),
  selectCurrentUser: vi.fn(() => null),
  selectOrderedChats: vi.fn(() => []),
}));

vi.mock("../../../mcp/rateLimiter", () => ({
  enforceRateLimit: vi.fn(),
  resetRequestCallCount: vi.fn(),
}));

vi.mock("../helpers", async (importOriginal) => {
  const original = (await importOriginal()) as any;
  return {
    ...original,
    getOrderedChats: vi.fn(() => []),
    apiDialogToTelegramChat: original.apiDialogToTelegramChat,
  };
});

describe("getChats", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should return cached data when cache hit occurs", async () => {
    const mockCachedChats: TelegramChat[] = [
      {
        id: "1",
        title: "Cached Chat 1",
        type: "private",
        unreadCount: 0,
        isPinned: false,
      },
      {
        id: "2",
        title: "Cached Chat 2",
        type: "group",
        unreadCount: 5,
        isPinned: true,
      },
    ];

    const { getOrderedChats } = await import("../helpers");
    vi.mocked(getOrderedChats).mockReturnValueOnce(mockCachedChats);

    const result = await getChats(20);

    expect(result.data).toEqual(mockCachedChats);
    expect(result.fromCache).toBe(true);

    const { mtprotoService } = await import(
      "../../services/mtprotoService"
    );
    expect(mtprotoService.getClient).not.toHaveBeenCalled();
  });

  it("should call API when cache is empty", async () => {
    const { getOrderedChats } = await import("../helpers");
    vi.mocked(getOrderedChats).mockReturnValueOnce([]);

    const { enforceRateLimit } = await import("../../../mcp/rateLimiter");
    vi.mocked(enforceRateLimit).mockResolvedValueOnce(undefined);

    const mockApiDialogs = {
      dialogs: [
        {
          peer: { className: "PeerChannel", channelId: 123n },
          unreadCount: 5,
          pinned: false,
        },
      ],
      chats: [
        {
          id: 123n,
          title: "API Chat",
          accessHash: 456n,
          megagroup: false,
        },
      ],
      users: [],
    };

    mockClient.invoke.mockResolvedValueOnce(mockApiDialogs);

    const result = await getChats(20);

    expect(result.fromCache).toBe(false);
    expect(enforceRateLimit).toHaveBeenCalledWith("__api_fallback_chats");
    expect(mockClient.invoke).toHaveBeenCalledWith(
      expect.objectContaining({
        className: "messages.GetDialogs",
      })
    );
  });

  it("should return empty array when API fails", async () => {
    const { getOrderedChats } = await import("../helpers");
    vi.mocked(getOrderedChats).mockReturnValueOnce([]);

    const { enforceRateLimit } = await import("../../../mcp/rateLimiter");
    vi.mocked(enforceRateLimit).mockResolvedValueOnce(undefined);

    mockClient.invoke.mockRejectedValueOnce(new Error("API Error"));

    const result = await getChats(20);

    expect(result.data).toEqual([]);
    expect(result.fromCache).toBe(false);
  });

  it("should return empty array when rate limit is rejected", async () => {
    const { getOrderedChats } = await import("../helpers");
    vi.mocked(getOrderedChats).mockReturnValueOnce([]);

    const { enforceRateLimit } = await import("../../../mcp/rateLimiter");
    vi.mocked(enforceRateLimit).mockRejectedValueOnce(
      new Error("Rate limit exceeded")
    );

    const result = await getChats(20);

    expect(result.data).toEqual([]);
    expect(result.fromCache).toBe(false);

    const { mtprotoService } = await import(
      "../../services/mtprotoService"
    );
    expect(mtprotoService.getClient).not.toHaveBeenCalled();
  });

  it("should respect custom limit parameter", async () => {
    const mockCachedChats: TelegramChat[] = Array.from(
      { length: 50 },
      (_, i) => ({
        id: String(i),
        title: `Chat ${i}`,
        type: "private",
        unreadCount: 0,
        isPinned: false,
      })
    );

    const { getOrderedChats } = await import("../helpers");
    vi.mocked(getOrderedChats).mockReturnValueOnce(
      mockCachedChats.slice(0, 50)
    );

    const result = await getChats(50);

    expect(getOrderedChats).toHaveBeenCalledWith(50);
    expect(result.data).toHaveLength(50);
  });
});
