import { describe, it, expect, vi, beforeEach } from "vitest";
import { sendMessage } from "../sendMessage";

// Mock dependencies
vi.mock("../../services/mtprotoService", () => {
  const mockClient = {
    invoke: vi.fn(),
    getInputEntity: vi.fn(),
    getMe: vi.fn(),
    sendMessage: vi.fn(),
  };

  return {
    mtprotoService: {
      getClient: vi.fn(() => mockClient),
      withFloodWaitHandling: vi.fn(async (fn: any) => await fn()),
      isReady: vi.fn(() => true),
      isClientConnected: vi.fn(() => true),
      sendMessage: vi.fn(),
    },
  };
});

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
    getChatById: vi.fn(),
  };
});

describe("sendMessage", () => {
  let mockMtprotoService: any;
  let mockClient: any;

  beforeEach(async () => {
    vi.clearAllMocks();
    const mtproto = await import("../../services/mtprotoService");
    mockMtprotoService = mtproto.mtprotoService;
    mockClient = mockMtprotoService.getClient();
  });

  it("should return undefined when chat is not found", async () => {
    const { getChatById } = await import("../helpers");
    vi.mocked(getChatById).mockReturnValueOnce(undefined);

    const result = await sendMessage("999", "Hello");

    expect(result.data).toBeUndefined();
    expect(result.fromCache).toBe(false);
    expect(mockMtprotoService.sendMessage).not.toHaveBeenCalled();
  });

  it("should use username when chat has username", async () => {
    const { getChatById } = await import("../helpers");
    vi.mocked(getChatById).mockReturnValueOnce({
      id: "1",
      title: "Test Chat",
      type: "private",
      username: "testuser",
      unreadCount: 0,
      isPinned: false,
    });

    mockMtprotoService.sendMessage.mockResolvedValueOnce({
      id: 100,
      randomId: 123n,
    } as any);

    const result = await sendMessage("1", "Hello with username");

    expect(mockMtprotoService.sendMessage).toHaveBeenCalledWith(
      "@testuser",
      "Hello with username"
    );
    expect(result.data).toBeDefined();
    expect(result.data?.id).toMatch(/^\d+$/); // Timestamp-based ID
    expect(result.fromCache).toBe(false);
  });

  it("should use numeric ID when chat has no username", async () => {
    const { getChatById } = await import("../helpers");
    vi.mocked(getChatById).mockReturnValueOnce({
      id: "123456",
      title: "Test Chat",
      type: "private",
      unreadCount: 0,
      isPinned: false,
    });

    mockMtprotoService.sendMessage.mockResolvedValueOnce({
      id: 200,
      randomId: 456n,
    } as any);

    const result = await sendMessage("123456", "Hello with ID");

    expect(mockMtprotoService.sendMessage).toHaveBeenCalledWith(
      "123456",
      "Hello with ID"
    );
    expect(result.data).toBeDefined();
    expect(result.data?.id).toMatch(/^\d+$/); // Timestamp-based ID
  });

  it("should use client.sendMessage with replyTo when replyToMessageId is provided", async () => {
    const { getChatById } = await import("../helpers");
    vi.mocked(getChatById).mockReturnValueOnce({
      id: "1",
      title: "Test Chat",
      type: "private",
      username: "testuser",
      unreadCount: 0,
      isPinned: false,
    });

    mockClient.sendMessage.mockResolvedValueOnce({
      id: 300,
      randomId: 789n,
    });

    const result = await sendMessage("1", "Reply message", 50);

    expect(mockClient.sendMessage).toHaveBeenCalledWith("@testuser", {
      message: "Reply message",
      replyTo: 50,
    });
    expect(result.data).toBeDefined();
    expect(result.data?.id).toMatch(/^\d+$/); // Timestamp-based ID
  });

  it("should handle API error by propagating exception", async () => {
    const { getChatById } = await import("../helpers");
    vi.mocked(getChatById).mockReturnValueOnce({
      id: "1",
      title: "Test Chat",
      type: "private",
      username: "testuser",
      unreadCount: 0,
      isPinned: false,
    });

    mockMtprotoService.sendMessage.mockRejectedValueOnce(
      new Error("Network error")
    );

    await expect(sendMessage("1", "Failed message")).rejects.toThrow(
      "Network error"
    );
  });

  it("should handle empty message text", async () => {
    const { getChatById } = await import("../helpers");
    vi.mocked(getChatById).mockReturnValueOnce({
      id: "1",
      title: "Test Chat",
      type: "private",
      username: "testuser",
      unreadCount: 0,
      isPinned: false,
    });

    mockMtprotoService.sendMessage.mockResolvedValueOnce({
      id: 400,
      randomId: 999n,
    } as any);

    const result = await sendMessage("1", "");

    expect(mockMtprotoService.sendMessage).toHaveBeenCalledWith(
      "@testuser",
      ""
    );
    expect(result.data).toBeDefined();
  });

  it("should handle long message text", async () => {
    const longMessage = "A".repeat(4096);
    const { getChatById } = await import("../helpers");
    vi.mocked(getChatById).mockReturnValueOnce({
      id: "1",
      title: "Test Chat",
      type: "private",
      unreadCount: 0,
      isPinned: false,
    });

    mockMtprotoService.sendMessage.mockResolvedValueOnce({
      id: 500,
      randomId: 111n,
    } as any);

    const result = await sendMessage("1", longMessage);

    expect(mockMtprotoService.sendMessage).toHaveBeenCalledWith(
      "1",
      longMessage
    );
    expect(result.data).toBeDefined();
  });

  it("should handle group chat without username", async () => {
    const { getChatById } = await import("../helpers");
    vi.mocked(getChatById).mockReturnValueOnce({
      id: "-1001234567890",
      title: "Test Group",
      type: "group",
      unreadCount: 0,
      isPinned: false,
    });

    mockMtprotoService.sendMessage.mockResolvedValueOnce({
      id: 600,
      randomId: 222n,
    } as any);

    const result = await sendMessage("-1001234567890", "Group message");

    expect(mockMtprotoService.sendMessage).toHaveBeenCalledWith(
      "-1001234567890",
      "Group message"
    );
    expect(result.data).toBeDefined();
    expect(result.data?.id).toMatch(/^\d+$/); // Timestamp-based ID
  });
});
