import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import { optString } from "../args";
import { addContact as addContactApi } from "../api/addContact";

export const tool: MCPTool = {
  name: "add_contact",
  description: "Add a contact to your Telegram account",
  inputSchema: {
    type: "object",
    properties: {
      phone: { type: "string", description: "Phone number" },
      first_name: { type: "string", description: "First name" },
      last_name: { type: "string", description: "Last name" },
    },
    required: ["phone", "first_name"],
  },
};

export async function addContact(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const phone = typeof args.phone === "string" ? args.phone : "";
    const firstName =
      typeof args.first_name === "string" ? args.first_name : "";
    const lastName = optString(args, "last_name");

    await addContactApi(phone, firstName, lastName);

    return {
      content: [
        {
          type: "text",
          text: `Contact ${firstName} ${lastName ?? ""} (${phone}) added.`,
        },
      ],
    };
  } catch (error) {
    return logAndFormatError(
      "add_contact",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.CONTACT
    );
  }
}
