import type { MCPTool, MCPToolResult } from "../../mcp/types";
import type { TelegramMCPContext } from "../types";
import { ErrorCategory, logAndFormatError } from "../../mcp/errorHandler";
import type { ContactInput } from "../api/apiResultTypes";
import { importContacts as importContactsApi } from "../api/importContacts";

export const tool: MCPTool = {
  name: "import_contacts",
  description: "Import contacts to Telegram",
  inputSchema: {
    type: "object",
    properties: {
      contacts: {
        type: "array",
        description: "Array of contacts: [{phone, first_name, last_name?}]",
        items: {
          type: "object",
          properties: {
            phone: { type: "string" },
            first_name: { type: "string" },
            last_name: { type: "string" },
          },
          required: ["phone", "first_name"],
        },
      },
    },
    required: ["contacts"],
  },
};

export async function importContacts(
  args: Record<string, unknown>,
  _context: TelegramMCPContext
): Promise<MCPToolResult> {
  try {
    const contactsArg = args.contacts;
    if (!Array.isArray(contactsArg)) {
      throw new Error("contacts must be an array");
    }

    const { data } = await importContactsApi(contactsArg as ContactInput[]);

    return {
      content: [
        {
          type: "text",
          text: `Imported ${data.imported} of ${data.total} contacts.`,
        },
      ],
    };
  } catch (error) {
    return logAndFormatError(
      "import_contacts",
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.CONTACT
    );
  }
}
