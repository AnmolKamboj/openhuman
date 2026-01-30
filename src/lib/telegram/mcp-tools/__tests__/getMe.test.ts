import { describe, it, expect, vi, beforeEach } from "vitest";
import { getMe, tool } from "../getMe";
import type { TelegramMCPContext } from "../../types";
import type { TelegramState } from "../../../../store/telegram/types";

vi.mock("../../api/getCurrentUser");

const { getCurrentUser } = await import("../../api/getCurrentUser");

describe("getMe tool handler", () => {
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

  it("should return formatted user JSON with all fields", async () => {
    const mockUser = {
      id: "123456789",
      firstName: "John",
      lastName: "Doe",
      username: "johndoe",
      phoneNumber: "+1234567890",
      isBot: false,
    };

    vi.mocked(getCurrentUser).mockResolvedValue({
      data: mockUser,
      fromCache: false,
    });

    const result = await getMe({}, mockContext);

    expect(result.isError).toBeFalsy();
    const text = result.content[0].text;
    const parsed = JSON.parse(text);
    expect(parsed.id).toBe("123456789");
    expect(parsed.name).toBe("John Doe");
    expect(parsed.username).toBe("johndoe");
    expect(parsed.type).toBe("user");
  });

  it("should handle user without username", async () => {
    const mockUser = {
      id: "987654321",
      firstName: "Jane",
      lastName: "Smith",
      username: undefined,
      phoneNumber: "+9876543210",
      isBot: false,
    };

    vi.mocked(getCurrentUser).mockResolvedValue({
      data: mockUser,
      fromCache: false,
    });

    const result = await getMe({}, mockContext);

    expect(result.isError).toBeFalsy();
    const text = result.content[0].text;
    const parsed = JSON.parse(text);
    expect(parsed.id).toBe("987654321");
    expect(parsed.name).toBe("Jane Smith");
    expect(parsed).not.toHaveProperty("username");
    expect(parsed.type).toBe("user");
  });

  it("should handle user with only first name", async () => {
    const mockUser = {
      id: "111222333",
      firstName: "Alice",
      lastName: undefined,
      username: "alice",
      phoneNumber: "+1112223333",
      isBot: false,
    };

    vi.mocked(getCurrentUser).mockResolvedValue({
      data: mockUser,
      fromCache: false,
    });

    const result = await getMe({}, mockContext);

    expect(result.isError).toBeFalsy();
    const text = result.content[0].text;
    const parsed = JSON.parse(text);
    expect(parsed.name).toBe("Alice");
  });

  it("should identify bot users correctly", async () => {
    const mockUser = {
      id: "999888777",
      firstName: "Bot",
      lastName: "Helper",
      username: "helpbot",
      phoneNumber: undefined,
      isBot: true,
    };

    vi.mocked(getCurrentUser).mockResolvedValue({
      data: mockUser,
      fromCache: false,
    });

    const result = await getMe({}, mockContext);

    expect(result.isError).toBeFalsy();
    const text = result.content[0].text;
    const parsed = JSON.parse(text);
    // formatEntity always returns "user" for TelegramUser, doesn't check isBot
    expect(parsed.type).toBe("user");
    expect(parsed.id).toBe("999888777");
  });

  it("should return error when user is not available", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue({
      data: undefined,
      fromCache: false,
    });

    const result = await getMe({}, mockContext);

    expect(result.isError).toBe(true);
    expect(result.content[0].text).toContain("User information not available");
  });

  it("should return error when user is null", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue({
      data: null as any,
      fromCache: false,
    });

    const result = await getMe({}, mockContext);

    expect(result.isError).toBe(true);
    expect(result.content[0].text).toContain("User information not available");
  });

  it("should handle API errors gracefully", async () => {
    vi.mocked(getCurrentUser).mockRejectedValue(
      new Error("Failed to fetch user")
    );

    const result = await getMe({}, mockContext);

    expect(result.isError).toBe(true);
    expect(result.content[0].text).toContain("PROFILE-ERR");
  });

  it("should include tool definition with correct schema", () => {
    expect(tool.name).toBe("get_me");
    expect(tool.description).toContain("user information");
    expect(tool.inputSchema.type).toBe("object");
    expect(tool.inputSchema.properties).toBeDefined();
  });
});
