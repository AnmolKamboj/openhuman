import { describe, it, expect, vi, beforeEach } from "vitest";
import { getChats, tool } from "../getChats";
import type { TelegramMCPContext } from "../../types";
import type { TelegramState } from "../../../../store/telegram/types";

vi.mock("../../api/getChats");

const { getChats: getChatsApi } = await import("../../api/getChats");

describe("getChats tool handler", () => {
  let mockContext: TelegramMCPContext;

  beforeEach(() => {
    vi.clearAllMocks();
    mockContext = {
      telegramState: {
        byUser: {},
        currentUserId: null,
      } as unknown as TelegramState,
      transport: {
        emit: vi.fn(),
        on: vi.fn(),
        off: vi.fn(),
      } as any,
    };
  });

  it("should return formatted chat list with IDs and titles", async () => {
    const mockChats = [
      {
        id: "123",
        title: "Tech Group",
        type: "group" as const,
        unreadCount: 0,
        isPinned: false,
      },
      {
        id: "456",
        title: "Trading Chat",
        type: "group" as const,
        unreadCount: 0,
        isPinned: false,
      },
      {
        id: "789",
        title: "John Doe",
        type: "private" as const,
        unreadCount: 0,
        isPinned: false,
      },
    ];

    vi.mocked(getChatsApi).mockResolvedValue({
      data: mockChats,
      fromCache: false,
    });

    const result = await getChats({}, mockContext);

    expect(result.isError).toBeFalsy();
    expect(result.content[0].text).toContain("123");
    expect(result.content[0].text).toContain("Tech Group");
    expect(result.content[0].text).toContain("456");
    expect(result.content[0].text).toContain("Trading Chat");
    expect(result.content[0].text).toContain("789");
    expect(result.content[0].text).toContain("John Doe");
  });

  it("should handle empty chat list", async () => {
    vi.mocked(getChatsApi).mockResolvedValue({
      data: [],
      fromCache: false,
    });

    const result = await getChats({}, mockContext);

    expect(result.isError).toBeFalsy();
    expect(result.content[0].text).toContain("Page out of range");
  });

  it("should return error for empty page", async () => {
    const mockChats = [
      {
        id: "123",
        title: "Chat 1",
        type: "private" as const,
        unreadCount: 0,
        isPinned: false,
      },
      {
        id: "456",
        title: "Chat 2",
        type: "private" as const,
        unreadCount: 0,
        isPinned: false,
      },
    ];

    vi.mocked(getChatsApi).mockResolvedValue({
      data: mockChats,
      fromCache: false,
    });

    const result = await getChats({ page: 5, page_size: 10 }, mockContext);

    expect(result.content[0].text).toContain("Page out of range");
  });

  it("should handle pagination with page 2 and page_size 2", async () => {
    const mockChats = [
      {
        id: "1",
        title: "Chat 1",
        type: "private" as const,
        unreadCount: 0,
        isPinned: false,
      },
      {
        id: "2",
        title: "Chat 2",
        type: "private" as const,
        unreadCount: 0,
        isPinned: false,
      },
      {
        id: "3",
        title: "Chat 3",
        type: "private" as const,
        unreadCount: 0,
        isPinned: false,
      },
      {
        id: "4",
        title: "Chat 4",
        type: "private" as const,
        unreadCount: 0,
        isPinned: false,
      },
      {
        id: "5",
        title: "Chat 5",
        type: "private" as const,
        unreadCount: 0,
        isPinned: false,
      },
    ];

    vi.mocked(getChatsApi).mockResolvedValue({
      data: mockChats,
      fromCache: false,
    });

    const result = await getChats({ page: 2, page_size: 2 }, mockContext);

    expect(result.isError).toBeFalsy();
    const text = result.content[0].text;
    expect(text).toContain("3");
    expect(text).toContain("Chat 3");
    expect(text).toContain("4");
    expect(text).toContain("Chat 4");
    expect(text).not.toContain("Chat 1");
    expect(text).not.toContain("Chat 5");
  });

  it("should use default pagination values", async () => {
    const mockChats = Array.from({ length: 25 }, (_, i) => ({
      id: String(i + 1),
      title: `Chat ${i + 1}`,
      type: "private" as const,
      unreadCount: 0,
      isPinned: false,
    }));

    vi.mocked(getChatsApi).mockResolvedValue({
      data: mockChats,
      fromCache: false,
    });

    const result = await getChats({}, mockContext);

    expect(result.isError).toBeFalsy();
    const text = result.content[0].text;
    // Default page_size should be 20
    expect(text).toContain("Chat 1");
    expect(text).toContain("Chat 20");
    expect(text).not.toContain("Chat 21");
  });

  it("should handle API errors gracefully", async () => {
    vi.mocked(getChatsApi).mockRejectedValue(
      new Error("Failed to fetch chats")
    );

    const result = await getChats({}, mockContext);

    expect(result.isError).toBe(true);
    expect(result.content[0].text).toContain("CHAT-ERR");
  });

  it("should include tool definition with correct schema", () => {
    expect(tool.name).toBe("get_chats");
    expect(tool.description).toContain("chats");
    expect(tool.inputSchema.type).toBe("object");
    expect(tool.inputSchema.properties).toHaveProperty("page");
    expect(tool.inputSchema.properties).toHaveProperty("page_size");
  });
});
