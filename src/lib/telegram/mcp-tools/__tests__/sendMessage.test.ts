import { describe, it, expect, vi, beforeEach } from "vitest";
import { sendMessage, tool } from "../sendMessage";
import type { TelegramMCPContext } from "../../types";
import type { TelegramState } from "../../../../store/telegram/types";

vi.mock("../../api/sendMessage");

const { sendMessage: sendMessageApi } = await import("../../api/sendMessage");

describe("sendMessage tool handler", () => {
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
    const result = await sendMessage({ message: "test message" }, mockContext);

    expect(result.isError).toBe(true);
    expect(result.content[0].text).toContain("chat_id");
  });

  it("should return error when message is missing", async () => {
    const result = await sendMessage({ chat_id: "123456789" }, mockContext);

    expect(result.isError).toBe(true);
    expect(result.content[0].text).toBe("Message content is required");
  });

  it("should return error when message is empty string", async () => {
    const result = await sendMessage(
      { chat_id: "123456789", message: "" },
      mockContext
    );

    expect(result.isError).toBe(true);
    expect(result.content[0].text).toBe("Message content is required");
  });

  it("should return error when message is non-string type", async () => {
    const result = await sendMessage(
      { chat_id: "123456789", message: 123 as any },
      mockContext
    );

    expect(result.isError).toBe(true);
    expect(result.content[0].text).toBe("Message content is required");
  });

  it("should successfully send message", async () => {
    vi.mocked(sendMessageApi).mockResolvedValue({
      data: { id: "42" },
      fromCache: false,
    });

    const result = await sendMessage(
      {
        chat_id: "123456789",
        message: "Hello, world!",
      },
      mockContext
    );

    expect(result.isError).toBeFalsy();
    expect(result.content[0].text).toBe("Message sent successfully.");
    // validateId converts string to number
    expect(sendMessageApi).toHaveBeenCalledWith(123456789, "Hello, world!");
  });

  it("should return error when API returns undefined", async () => {
    vi.mocked(sendMessageApi).mockResolvedValue({
      data: undefined,
      fromCache: false,
    });

    const result = await sendMessage(
      {
        chat_id: "123456789",
        message: "test",
      },
      mockContext
    );

    expect(result.isError).toBe(true);
    expect(result.content[0].text).toContain("Failed to send message");
  });

  it("should handle API errors gracefully", async () => {
    vi.mocked(sendMessageApi).mockRejectedValue(new Error("Network error"));

    const result = await sendMessage(
      {
        chat_id: "123456789",
        message: "test",
      },
      mockContext
    );

    expect(result.isError).toBe(true);
    // Error handler formats with error code
    expect(result.content[0].text).toContain("MSG-ERR");
  });

  it("should include tool definition with correct schema", () => {
    expect(tool.name).toBe("send_message");
    expect(tool.description).toContain("Send a message");
    expect(tool.inputSchema.type).toBe("object");
    expect(tool.inputSchema.properties).toHaveProperty("chat_id");
    expect(tool.inputSchema.properties).toHaveProperty("message");
    expect(tool.inputSchema.required).toContain("chat_id");
    expect(tool.inputSchema.required).toContain("message");
  });
});
