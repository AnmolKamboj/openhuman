import { describe, it, expect, vi, beforeEach } from "vitest";
import bigInt from "big-integer";
import {
  formatEntity,
  formatMessage,
  apiMessageToTelegramMessage,
  apiDialogToTelegramChat,
} from "../helpers";
import type {
  TelegramChat,
  TelegramUser,
  TelegramMessage,
} from "../../../../store/telegram/types";

// Mock the store module
vi.mock("../../../../../store", () => ({
  store: {
    getState: vi.fn(() => ({
      telegram: {
        byUser: {},
      },
    })),
  },
}));

// Mock the selectors module
vi.mock("../../../../../store/telegramSelectors", () => ({
  selectChat: vi.fn(),
  selectUser: vi.fn(),
}));

describe("formatEntity", () => {
  describe("with TelegramChat", () => {
    it("should format a channel chat", () => {
      const chat: TelegramChat = {
        id: "123",
        title: "Test Channel",
        type: "channel",
        username: "testchannel",
        unreadCount: 5,
        isPinned: false,
      };

      const result = formatEntity(chat);

      expect(result).toEqual({
        id: "123",
        name: "Test Channel",
        type: "channel",
        username: "testchannel",
      });
    });

    it("should map supergroup to group type", () => {
      const chat: TelegramChat = {
        id: "456",
        title: "Test Group",
        type: "supergroup",
        unreadCount: 0,
        isPinned: true,
      };

      const result = formatEntity(chat);

      expect(result).toEqual({
        id: "456",
        name: "Test Group",
        type: "group",
      });
    });

    it("should handle private chat type", () => {
      const chat: TelegramChat = {
        id: "789",
        title: "John Doe",
        type: "private",
        unreadCount: 2,
        isPinned: false,
      };

      const result = formatEntity(chat);

      expect(result).toEqual({
        id: "789",
        name: "John Doe",
        type: "private",
      });
    });

    it("should use 'Unknown' when title is undefined", () => {
      const chat: TelegramChat = {
        id: "999",
        title: undefined,
        type: "group",
        unreadCount: 0,
        isPinned: false,
      };

      const result = formatEntity(chat);

      expect(result).toEqual({
        id: "999",
        name: "Unknown",
        type: "group",
      });
    });

    it("should include username if present", () => {
      const chat: TelegramChat = {
        id: "111",
        title: "Public Group",
        type: "group",
        username: "publicgroup",
        unreadCount: 0,
        isPinned: false,
      };

      const result = formatEntity(chat);

      expect(result).toEqual({
        id: "111",
        name: "Public Group",
        type: "group",
        username: "publicgroup",
      });
    });
  });

  describe("with TelegramUser", () => {
    it("should format a user with full name", () => {
      const user: TelegramUser = {
        id: "123",
        firstName: "John",
        lastName: "Doe",
        username: "johndoe",
        isBot: false,
      };

      const result = formatEntity(user);

      expect(result).toEqual({
        id: "123",
        name: "John Doe",
        type: "user",
        username: "johndoe",
      });
    });

    it("should format a user with only first name", () => {
      const user: TelegramUser = {
        id: "456",
        firstName: "Alice",
        isBot: false,
      };

      const result = formatEntity(user);

      expect(result).toEqual({
        id: "456",
        name: "Alice",
        type: "user",
      });
    });

    it("should include phone number if present", () => {
      const user: TelegramUser = {
        id: "789",
        firstName: "Bob",
        lastName: "Smith",
        phoneNumber: "+1234567890",
        isBot: false,
      };

      const result = formatEntity(user);

      expect(result).toEqual({
        id: "789",
        name: "Bob Smith",
        type: "user",
        phone: "+1234567890",
      });
    });

    it("should handle bot users", () => {
      const user: TelegramUser = {
        id: "999",
        firstName: "Bot",
        username: "testbot",
        isBot: true,
      };

      const result = formatEntity(user);

      expect(result).toEqual({
        id: "999",
        name: "Bot",
        type: "user",
        username: "testbot",
      });
    });
  });
});

describe("formatMessage", () => {
  it("should format a basic message", () => {
    const message: TelegramMessage = {
      id: "1",
      chatId: "123",
      date: 1640000000,
      message: "Hello, world!",
      isOutgoing: false,
      isEdited: false,
      isForwarded: false,
    };

    const result = formatMessage(message);

    expect(result).toEqual({
      id: "1",
      date: new Date(1640000000 * 1000).toISOString(),
      text: "Hello, world!",
    });
  });

  it("should include fromId when present", () => {
    const message: TelegramMessage = {
      id: "2",
      chatId: "123",
      date: 1640000000,
      message: "Test message",
      fromId: "456",
      isOutgoing: false,
      isEdited: false,
      isForwarded: false,
    };

    const result = formatMessage(message);

    expect(result).toEqual({
      id: "2",
      date: new Date(1640000000 * 1000).toISOString(),
      text: "Test message",
      from_id: "456",
    });
  });

  it("should include media information when media exists", () => {
    const message: TelegramMessage = {
      id: "3",
      chatId: "123",
      date: 1640000000,
      message: "Photo message",
      media: { type: "photo" },
      isOutgoing: false,
      isEdited: false,
      isForwarded: false,
    };

    const result = formatMessage(message);

    expect(result).toEqual({
      id: "3",
      date: new Date(1640000000 * 1000).toISOString(),
      text: "Photo message",
      has_media: true,
      media_type: "photo",
    });
  });

  it("should not include media fields when media is absent", () => {
    const message: TelegramMessage = {
      id: "4",
      chatId: "123",
      date: 1640000000,
      message: "Text only",
      isOutgoing: false,
      isEdited: false,
      isForwarded: false,
    };

    const result = formatMessage(message);

    expect(result.has_media).toBeUndefined();
    expect(result.media_type).toBeUndefined();
  });
});

describe("apiMessageToTelegramMessage", () => {
  it("should convert a basic API message", () => {
    const apiMsg = {
      id: 42,
      date: 1640000000,
      message: "Hello",
    };

    const result = apiMessageToTelegramMessage(apiMsg as any, "123");

    expect(result).toEqual({
      id: "42",
      chatId: "123",
      date: 1640000000,
      message: "Hello",
      isOutgoing: false,
      isEdited: false,
      isForwarded: false,
    });
  });

  it("should extract fromId from userId", () => {
    const apiMsg = {
      id: 42,
      date: 1640000000,
      message: "Test",
      fromId: { userId: bigInt(456) },
    };

    const result = apiMessageToTelegramMessage(apiMsg as any, "123");

    expect(result.fromId).toBe("456");
  });

  it("should include replyTo message ID", () => {
    const apiMsg = {
      id: 42,
      date: 1640000000,
      message: "Reply",
      replyTo: { replyToMsgId: 10 },
    };

    const result = apiMessageToTelegramMessage(apiMsg as any, "123");

    expect(result.replyToMessageId).toBe("10");
  });

  it("should detect media presence", () => {
    const apiMsg = {
      id: 42,
      date: 1640000000,
      message: "Photo",
      media: { className: "MessageMediaPhoto" },
    };

    const result = apiMessageToTelegramMessage(apiMsg as any, "123");

    expect(result.media).toEqual({ type: "MessageMediaPhoto" });
  });

  it("should set isEdited when editDate exists", () => {
    const apiMsg = {
      id: 42,
      date: 1640000000,
      message: "Edited",
      editDate: 1640001000,
    };

    const result = apiMessageToTelegramMessage(apiMsg as any, "123");

    expect(result.isEdited).toBe(true);
  });

  it("should set isForwarded when fwdFrom exists", () => {
    const apiMsg = {
      id: 42,
      date: 1640000000,
      message: "Forwarded",
      fwdFrom: { fromId: bigInt(789) },
    };

    const result = apiMessageToTelegramMessage(apiMsg as any, "123");

    expect(result.isForwarded).toBe(true);
  });

  it("should set isOutgoing when out flag is true", () => {
    const apiMsg = {
      id: 42,
      date: 1640000000,
      message: "Sent",
      out: true,
    };

    const result = apiMessageToTelegramMessage(apiMsg as any, "123");

    expect(result.isOutgoing).toBe(true);
  });

  it("should handle missing optional fields", () => {
    const apiMsg = {
      id: 42,
      date: 1640000000,
      message: "Minimal",
    };

    const result = apiMessageToTelegramMessage(apiMsg as any, "123");

    expect(result.fromId).toBeUndefined();
    expect(result.replyToMessageId).toBeUndefined();
    expect(result.media).toBeUndefined();
  });
});

describe("apiDialogToTelegramChat", () => {
  const mockChatsById = new Map();
  const mockUsersById = new Map();

  beforeEach(() => {
    mockChatsById.clear();
    mockUsersById.clear();
  });

  it("should convert PeerUser dialog", () => {
    const user = {
      id: bigInt(123),
      firstName: "John",
      lastName: "Doe",
      username: "johndoe",
    };
    mockUsersById.set("123", user);

    const dialog = {
      peer: { className: "PeerUser", userId: bigInt(123) },
      unreadCount: 5,
      pinned: true,
    };

    const result = apiDialogToTelegramChat(
      dialog as any,
      mockChatsById,
      mockUsersById
    );

    expect(result).toEqual({
      id: "123",
      title: "John Doe",
      type: "private",
      username: "johndoe",
      unreadCount: 5,
      isPinned: true,
    });
  });

  it("should convert PeerChat dialog", () => {
    const chat = {
      id: bigInt(456),
      title: "Test Group",
    };
    mockChatsById.set("456", chat);

    const dialog = {
      peer: { className: "PeerChat", chatId: bigInt(456) },
      unreadCount: 2,
      pinned: false,
    };

    const result = apiDialogToTelegramChat(
      dialog as any,
      mockChatsById,
      mockUsersById
    );

    expect(result).toEqual({
      id: "456",
      title: "Test Group",
      type: "group",
      unreadCount: 2,
      isPinned: false,
    });
  });

  it("should convert PeerChannel with megagroup flag", () => {
    const channel = {
      id: bigInt(789),
      title: "Supergroup",
      username: "mygroup",
      megagroup: true,
    };
    mockChatsById.set("789", channel);

    const dialog = {
      peer: { className: "PeerChannel", channelId: bigInt(789) },
      unreadCount: 0,
      pinned: false,
    };

    const result = apiDialogToTelegramChat(
      dialog as any,
      mockChatsById,
      mockUsersById
    );

    expect(result).toEqual({
      id: "789",
      title: "Supergroup",
      type: "supergroup",
      username: "mygroup",
      unreadCount: 0,
      isPinned: false,
    });
  });

  it("should convert PeerChannel without megagroup as channel", () => {
    const channel = {
      id: bigInt(999),
      title: "News Channel",
      username: "news",
      megagroup: false,
    };
    mockChatsById.set("999", channel);

    const dialog = {
      peer: { className: "PeerChannel", channelId: bigInt(999) },
      unreadCount: 10,
      pinned: true,
    };

    const result = apiDialogToTelegramChat(
      dialog as any,
      mockChatsById,
      mockUsersById
    );

    expect(result).toEqual({
      id: "999",
      title: "News Channel",
      type: "channel",
      username: "news",
      unreadCount: 10,
      isPinned: true,
    });
  });

  it("should return undefined for unknown peer type", () => {
    const dialog = {
      peer: {}, // No userId, chatId, or channelId
      unreadCount: 0,
      pinned: false,
    };

    const result = apiDialogToTelegramChat(
      dialog as any,
      mockChatsById,
      mockUsersById
    );

    expect(result).toBeUndefined();
  });

  it("should use 'Unknown' title when raw entity is missing for PeerUser", () => {
    const dialog = {
      peer: { className: "PeerUser", userId: bigInt(123) },
      unreadCount: 0,
      pinned: false,
    };
    // User not in mockUsersById

    const result = apiDialogToTelegramChat(
      dialog as any,
      mockChatsById,
      mockUsersById
    );

    expect(result).toEqual({
      id: "123",
      title: "Unknown",
      type: "private",
      unreadCount: 0,
      isPinned: false,
    });
  });

  it("should use 'Unknown' title when raw entity is missing for PeerChat", () => {
    const dialog = {
      peer: { className: "PeerChat", chatId: bigInt(456) },
      unreadCount: 0,
      pinned: false,
    };
    // Chat not in mockChatsById

    const result = apiDialogToTelegramChat(
      dialog as any,
      mockChatsById,
      mockUsersById
    );

    expect(result).toEqual({
      id: "456",
      title: "Unknown",
      type: "group",
      unreadCount: 0,
      isPinned: false,
    });
  });

  it("should use 'Unknown' title when raw entity is missing for PeerChannel", () => {
    const dialog = {
      peer: { className: "PeerChannel", channelId: bigInt(789) },
      unreadCount: 0,
      pinned: false,
    };
    // Channel not in mockChatsById

    const result = apiDialogToTelegramChat(
      dialog as any,
      mockChatsById,
      mockUsersById
    );

    expect(result).toEqual({
      id: "789",
      title: "Unknown",
      type: "channel",
      unreadCount: 0,
      isPinned: false,
    });
  });
});
