import { describe, it, expect, vi, beforeEach } from "vitest";
import { getCurrentUser } from "../getCurrentUser";
import type { TelegramUser } from "../../../../../store/telegram/types";

// Mock dependencies
const mockClient = {
  invoke: vi.fn(),
  getInputEntity: vi.fn(),
  getMe: vi.fn(),
  sendMessage: vi.fn(),
};

vi.mock("../../../../../services/mtprotoService", () => ({
  mtprotoService: {
    getClient: vi.fn(() => mockClient),
    withFloodWaitHandling: vi.fn(async (fn: any) => await fn()),
    isReady: vi.fn(() => true),
    isClientConnected: vi.fn(() => true),
    sendMessage: vi.fn(),
  },
}));

vi.mock("../../../../../store", () => ({
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
    dispatch: vi.fn(),
  },
}));

vi.mock("../../../../../store/telegramSelectors", () => ({
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

vi.mock("../../../rateLimiter", () => ({
  enforceRateLimit: vi.fn(),
  resetRequestCallCount: vi.fn(),
}));

vi.mock("../helpers", async (importOriginal) => {
  const original = (await importOriginal()) as any;
  return {
    ...original,
    getCurrentUser: vi.fn(() => undefined),
  };
});

describe("getCurrentUser", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should return cached user when cache hit occurs", async () => {
    const mockCachedUser: TelegramUser = {
      id: "self123",
      firstName: "John",
      lastName: "Doe",
      username: "johndoe",
      phoneNumber: "+1234567890",
      isBot: false,
    };

    const { getCurrentUser: getCurrentUserHelper } = await import("../helpers");
    vi.mocked(getCurrentUserHelper).mockReturnValueOnce(mockCachedUser);

    const result = await getCurrentUser();

    expect(result.data).toEqual(mockCachedUser);
    expect(result.fromCache).toBe(true);

    const { mtprotoService } = await import(
      "../../../../../services/mtprotoService"
    );
    expect(mtprotoService.getClient).not.toHaveBeenCalled();
  });

  it("should call API when cache is empty", async () => {
    const { getCurrentUser: getCurrentUserHelper } = await import("../helpers");
    vi.mocked(getCurrentUserHelper).mockReturnValueOnce(undefined);

    const { enforceRateLimit } = await import("../../../rateLimiter");
    vi.mocked(enforceRateLimit).mockResolvedValueOnce(undefined);

    const mockApiUser = {
      id: 123n,
      firstName: "API User",
      lastName: "Test",
      username: "apiuser",
      phone: "+9876543210",
    };

    mockClient.getMe.mockResolvedValueOnce(mockApiUser);

    const result = await getCurrentUser();

    expect(result.fromCache).toBe(false);
    expect(result.data).toBeDefined();
    expect(result.data?.id).toBe("123");
    expect(result.data?.firstName).toBe("API User");
    expect(enforceRateLimit).toHaveBeenCalledWith("__api_fallback_me");
  });

  it("should return undefined when API returns null", async () => {
    const { getCurrentUser: getCurrentUserHelper } = await import("../helpers");
    vi.mocked(getCurrentUserHelper).mockReturnValueOnce(undefined);

    const { enforceRateLimit } = await import("../../../rateLimiter");
    vi.mocked(enforceRateLimit).mockResolvedValueOnce(undefined);

    mockClient.getMe.mockResolvedValueOnce(null);

    const result = await getCurrentUser();

    expect(result.data).toBeUndefined();
    expect(result.fromCache).toBe(false);
  });

  it("should return undefined when API fails", async () => {
    const { getCurrentUser: getCurrentUserHelper } = await import("../helpers");
    vi.mocked(getCurrentUserHelper).mockReturnValueOnce(undefined);

    const { enforceRateLimit } = await import("../../../rateLimiter");
    vi.mocked(enforceRateLimit).mockResolvedValueOnce(undefined);

    mockClient.getMe.mockRejectedValueOnce(new Error("API Error"));

    const result = await getCurrentUser();

    expect(result.data).toBeUndefined();
    expect(result.fromCache).toBe(false);
  });

  it("should return undefined when rate limit is rejected", async () => {
    const { getCurrentUser: getCurrentUserHelper } = await import("../helpers");
    vi.mocked(getCurrentUserHelper).mockReturnValueOnce(undefined);

    const { enforceRateLimit } = await import("../../../rateLimiter");
    vi.mocked(enforceRateLimit).mockRejectedValueOnce(
      new Error("Rate limit exceeded")
    );

    const result = await getCurrentUser();

    expect(result.data).toBeUndefined();
    expect(result.fromCache).toBe(false);

    const { mtprotoService } = await import(
      "../../../../../services/mtprotoService"
    );
    expect(mtprotoService.getClient).not.toHaveBeenCalled();
  });

  it("should handle user with minimal fields", async () => {
    const { getCurrentUser: getCurrentUserHelper } = await import("../helpers");
    vi.mocked(getCurrentUserHelper).mockReturnValueOnce(undefined);

    const { enforceRateLimit } = await import("../../../rateLimiter");
    vi.mocked(enforceRateLimit).mockResolvedValueOnce(undefined);

    const mockApiUser = {
      id: 456n,
      firstName: "MinimalUser",
      // No lastName, username, or phone
    };

    mockClient.getMe.mockResolvedValueOnce(mockApiUser);

    const result = await getCurrentUser();

    expect(result.data).toBeDefined();
    expect(result.data?.id).toBe("456");
    expect(result.data?.firstName).toBe("MinimalUser");
    expect(result.data?.lastName).toBeUndefined();
    expect(result.data?.username).toBeUndefined();
  });
});
