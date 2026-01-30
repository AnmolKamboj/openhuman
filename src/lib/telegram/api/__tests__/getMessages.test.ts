import { describe, it, expect, vi, beforeEach } from "vitest";
import { getMessages } from "../getMessages";
import type { TelegramMessage } from "../../../../store/telegram/types";

// Mock dependencies
const mockClient = {
  invoke: vi.fn(),
  getInputEntity: vi.fn(),
  getMe: vi.fn(),
  sendMessage: vi.fn(),
};

vi.mock("../../../../services/mtprotoService", () => ({
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
    getCachedMessages: vi.fn(() => undefined),
    getChatById: vi.fn(() => ({
      id: "1",
      title: "Test Chat",
      type: "private",
      unreadCount: 0,
      isPinned: false,
    })),
    apiMessageToTelegramMessage: original.apiMessageToTelegramMessage,
  };
});

describe("getMessages", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should return cached messages when cache hit occurs", async () => {
    const mockCachedMessages: TelegramMessage[] = [
      {
        id: "1",
        chatId: "1",
        message: "Cached message 1",
        date: Date.now(),
        fromId: "sender1",
        fromName: "John",
        isOutgoing: false,
        isEdited: false,
        isForwarded: false,
      },
      {
        id: "2",
        chatId: "1",
        message: "Cached message 2",
        date: Date.now(),
        fromId: "sender2",
        fromName: "Jane",
        isOutgoing: true,
        isEdited: false,
        isForwarded: false,
      },
    ];

    const { getCachedMessages } = await import("../helpers");
    vi.mocked(getCachedMessages).mockReturnValueOnce(mockCachedMessages);

    const result = await getMessages("1", 20);

    expect(result.data).toEqual(mockCachedMessages);
    expect(result.fromCache).toBe(true);

    const { mtprotoService } = await import(
      "../../../../services/mtprotoService"
    );
    expect(mtprotoService.getClient).not.toHaveBeenCalled();
  });

  it("should call API when cache is empty and chat is found", async () => {
    const { getCachedMessages, getChatById } = await import("../helpers");
    vi.mocked(getCachedMessages).mockReturnValueOnce(undefined);
    vi.mocked(getChatById).mockReturnValueOnce({
      id: "1",
      title: "Test Chat",
      type: "private",
      unreadCount: 0,
      isPinned: false,
    });

    const { enforceRateLimit } = await import("../../../mcp/rateLimiter");
    vi.mocked(enforceRateLimit).mockResolvedValueOnce(undefined);

    const mockApiMessages = {
      messages: [
        {
          id: 100,
          peerId: { userId: 123n },
          message: "API message",
          date: Math.floor(Date.now() / 1000),
          fromId: { userId: 456n },
          out: false,
        },
      ],
      chats: [],
      users: [
        {
          id: 456n,
          firstName: "API User",
        },
      ],
    };

    mockClient.getInputEntity.mockResolvedValueOnce({
      className: "InputPeerUser",
      userId: 1n,
    });
    mockClient.invoke.mockResolvedValueOnce(mockApiMessages);

    const result = await getMessages("1", 20);

    expect(result.fromCache).toBe(false);
    expect(enforceRateLimit).toHaveBeenCalledWith("__api_fallback_messages");
    expect(mockClient.invoke).toHaveBeenCalledWith(
      expect.objectContaining({
        className: "messages.GetHistory",
      })
    );
  });

  it("should return empty array when chat is not found", async () => {
    const { getCachedMessages, getChatById } = await import("../helpers");
    vi.mocked(getCachedMessages).mockReturnValueOnce(undefined);
    vi.mocked(getChatById).mockReturnValueOnce(undefined);

    const result = await getMessages("999", 20);

    expect(result.data).toEqual([]);
    expect(result.fromCache).toBe(false);

    const { mtprotoService } = await import(
      "../../../../services/mtprotoService"
    );
    expect(mtprotoService.getClient).not.toHaveBeenCalled();
  });

  it("should return empty array when API fails", async () => {
    const { getCachedMessages, getChatById } = await import("../helpers");
    vi.mocked(getCachedMessages).mockReturnValueOnce(undefined);
    vi.mocked(getChatById).mockReturnValueOnce({
      id: "1",
      title: "Test Chat",
      type: "private",
      unreadCount: 0,
      isPinned: false,
    });

    const { enforceRateLimit } = await import("../../../mcp/rateLimiter");
    vi.mocked(enforceRateLimit).mockResolvedValueOnce(undefined);

    mockClient.getInputEntity.mockResolvedValueOnce({
      className: "InputPeerUser",
      userId: 1n,
    });
    mockClient.invoke.mockRejectedValueOnce(new Error("API Error"));

    const result = await getMessages("1", 20);

    expect(result.data).toEqual([]);
    expect(result.fromCache).toBe(false);
  });

  it("should return empty array when rate limit is rejected", async () => {
    const { getCachedMessages, getChatById } = await import("../helpers");
    vi.mocked(getCachedMessages).mockReturnValueOnce(undefined);
    vi.mocked(getChatById).mockReturnValueOnce({
      id: "1",
      title: "Test Chat",
      type: "private",
      unreadCount: 0,
      isPinned: false,
    });

    const { enforceRateLimit } = await import("../../../mcp/rateLimiter");
    vi.mocked(enforceRateLimit).mockRejectedValueOnce(
      new Error("Rate limit exceeded")
    );

    const result = await getMessages("1", 20);

    expect(result.data).toEqual([]);
    expect(result.fromCache).toBe(false);

    const { mtprotoService } = await import(
      "../../../../services/mtprotoService"
    );
    expect(mtprotoService.getClient).not.toHaveBeenCalled();
  });

  it("should respect custom limit parameter", async () => {
    const mockCachedMessages: TelegramMessage[] = Array.from(
      { length: 100 },
      (_, i) => ({
        id: String(i),
        chatId: "1",
        message: `Message ${i}`,
        date: Date.now(),
        fromId: "sender1",
        fromName: "User",
        isOutgoing: false,
        isEdited: false,
        isForwarded: false,
      })
    );

    const { getCachedMessages } = await import("../helpers");
    vi.mocked(getCachedMessages).mockReturnValueOnce(
      mockCachedMessages.slice(0, 100)
    );

    const result = await getMessages("1", 100);

    expect(getCachedMessages).toHaveBeenCalledWith("1", 100, 0);
    expect(result.data).toHaveLength(100);
  });
});
