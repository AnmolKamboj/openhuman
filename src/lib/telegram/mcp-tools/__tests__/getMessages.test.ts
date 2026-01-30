import { describe, it, expect, vi, beforeEach } from "vitest";
import { getMessages, tool } from "../getMessages";
import type { TelegramMCPContext } from "../../types";
import type { TelegramState } from "../../../../store/telegram/types";

vi.mock("../../api/getMessages");
vi.mock("../../api/helpers", async (importOriginal) => {
  const original = (await importOriginal()) as any;
  return {
    ...original,
    getChatById: vi.fn(),
    formatMessage: original.formatMessage,
  };
});

const { getMessages: getMessagesApi } = await import("../../api/getMessages");
const { getChatById } = await import("../../api/helpers");

describe("getMessages tool handler", () => {
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

  it("should return error when chat_id is missing", async () => {
    const result = await getMessages({}, mockContext);

    expect(result.isError).toBe(true);
    expect(result.content[0].text).toContain("chat_id");
  });

  it("should return error when chat is not found", async () => {
    vi.mocked(getChatById).mockReturnValue(undefined);

    const result = await getMessages({ chat_id: "123456789" }, mockContext);

    expect(result.isError).toBe(true);
    expect(result.content[0].text).toContain("Chat not found: 123456789");
  });

  it("should return formatted messages with ID, sender, and date", async () => {
    const mockChat = {
      id: "123456789",
      title: "Test Chat",
    };

    const mockMessages = [
      {
        id: "1",
        chatId: "123456789",
        fromId: "user1",
        fromName: "User One",
        message: "Hello!",
        date: 1609459200,
        isOutgoing: false,
        isEdited: false,
        isForwarded: false,
      },
      {
        id: "2",
        chatId: "123456789",
        fromId: "user2",
        fromName: "User Two",
        message: "Hi there!",
        date: 1609459260,
        isOutgoing: false,
        isEdited: false,
        isForwarded: false,
      },
    ];

    vi.mocked(getChatById).mockReturnValue(mockChat as any);
    vi.mocked(getMessagesApi).mockResolvedValue({
      data: mockMessages,
      fromCache: false,
    });

    const result = await getMessages({ chat_id: "123456789" }, mockContext);

    expect(result.isError).toBeFalsy();
    const text = result.content[0].text;
    expect(text).toContain("ID: 1");
    expect(text).toContain("User One");
    expect(text).toContain("Hello!");
    expect(text).toContain("ID: 2");
    expect(text).toContain("User Two");
    expect(text).toContain("Hi there!");
  });

  it("should return message when no messages are found", async () => {
    const mockChat = {
      id: "123456789",
      title: "Empty Chat",
    };

    vi.mocked(getChatById).mockReturnValue(mockChat as any);
    vi.mocked(getMessagesApi).mockResolvedValue({
      data: [],
      fromCache: false,
    });

    const result = await getMessages({ chat_id: "123456789" }, mockContext);

    expect(result.isError).toBeFalsy();
    expect(result.content[0].text).toContain("No messages found for this page");
  });

  it("should handle pagination correctly", async () => {
    const mockChat = {
      id: "123456789",
      title: "Test Chat",
    };

    const mockMessages = [
      {
        id: "3",
        chatId: "123456789",
        fromId: "user3",
        message: "Message 3",
        date: 1609459300,
        isOutgoing: false,
        isEdited: false,
        isForwarded: false,
      },
      {
        id: "4",
        chatId: "123456789",
        fromId: "user4",
        message: "Message 4",
        date: 1609459400,
        isOutgoing: false,
        isEdited: false,
        isForwarded: false,
      },
    ];

    vi.mocked(getChatById).mockReturnValue(mockChat as any);
    vi.mocked(getMessagesApi).mockResolvedValue({
      data: mockMessages,
      fromCache: false,
    });

    const result = await getMessages(
      { chat_id: "123456789", page: 2, page_size: 2 },
      mockContext
    );

    expect(result.isError).toBeFalsy();
    // validateId converts string to number
    expect(getMessagesApi).toHaveBeenCalledWith(123456789, 2, 2);
  });

  it("should use default pagination values", async () => {
    const mockChat = {
      id: "123456789",
      title: "Test Chat",
    };

    vi.mocked(getChatById).mockReturnValue(mockChat as any);
    vi.mocked(getMessagesApi).mockResolvedValue({
      data: [],
      fromCache: false,
    });

    await getMessages({ chat_id: "123456789" }, mockContext);

    // validateId converts string to number
    expect(getMessagesApi).toHaveBeenCalledWith(123456789, 20, 0);
  });

  it("should handle messages without sender ID", async () => {
    const mockChat = {
      id: "123456789",
      title: "Test Chat",
    };

    const mockMessages = [
      {
        id: "5",
        chatId: "123456789",
        fromId: undefined,
        fromName: undefined,
        message: "System message",
        date: 1609459500,
        isOutgoing: false,
        isEdited: false,
        isForwarded: false,
      },
    ];

    vi.mocked(getChatById).mockReturnValue(mockChat as any);
    vi.mocked(getMessagesApi).mockResolvedValue({
      data: mockMessages,
      fromCache: false,
    });

    const result = await getMessages({ chat_id: "123456789" }, mockContext);

    expect(result.isError).toBeFalsy();
    const text = result.content[0].text;
    expect(text).toContain("ID: 5");
    expect(text).toContain("Unknown");
    expect(text).toContain("System message");
  });

  it("should handle API errors gracefully", async () => {
    const mockChat = {
      id: "123456789",
      title: "Test Chat",
    };

    vi.mocked(getChatById).mockReturnValue(mockChat as any);
    vi.mocked(getMessagesApi).mockRejectedValue(new Error("Network error"));

    const result = await getMessages({ chat_id: "123456789" }, mockContext);

    expect(result.isError).toBe(true);
    expect(result.content[0].text).toContain("MSG-ERR");
  });

  it("should include tool definition with correct schema", () => {
    expect(tool.name).toBe("get_messages");
    expect(tool.description).toContain("messages");
    expect(tool.inputSchema.type).toBe("object");
    expect(tool.inputSchema.properties).toHaveProperty("chat_id");
    expect(tool.inputSchema.properties).toHaveProperty("page");
    expect(tool.inputSchema.properties).toHaveProperty("page_size");
    expect(tool.inputSchema.required).toContain("chat_id");
  });
});
