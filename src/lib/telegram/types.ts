/**
 * Telegram MCP server types
 */

import type { SocketIOMCPTransportImpl } from "../mcp/transport";
import type { MCPToolResult } from "../mcp/types";
import type { TelegramState } from "../../store/telegram/types";

export interface TelegramMCPContext {
  telegramState: TelegramState;
  transport: SocketIOMCPTransportImpl;
}

export type TelegramMCPToolHandler = (
  args: Record<string, unknown>,
  context: TelegramMCPContext
) => Promise<MCPToolResult>;

export const TELEGRAM_MCP_TOOL_NAMES = [
  "get_chats",
  "list_chats",
  "get_chat",
  "create_group",
  "invite_to_group",
  "create_channel",
  "edit_chat_title",
  "delete_chat_photo",
  "leave_chat",
  "get_participants",
  "get_admins",
  "get_banned_users",
  "promote_admin",
  "demote_admin",
  "ban_user",
  "unban_user",
  "get_invite_link",
  "export_chat_invite",
  "import_chat_invite",
  "join_chat_by_link",
  "subscribe_public_channel",
  "get_messages",
  "list_messages",
  "list_topics",
  "send_message",
  "reply_to_message",
  "edit_message",
  "delete_message",
  "forward_message",
  "pin_message",
  "unpin_message",
  "mark_as_read",
  "get_message_context",
  "get_history",
  "get_pinned_messages",
  "send_reaction",
  "remove_reaction",
  "get_message_reactions",
  "list_contacts",
  "search_contacts",
  "add_contact",
  "delete_contact",
  "block_user",
  "unblock_user",
  "get_blocked_users",
  "get_me",
  "update_profile",
  "get_user_photos",
  "get_user_status",
  "mute_chat",
  "unmute_chat",
  "archive_chat",
  "unarchive_chat",
  "get_privacy_settings",
  "set_privacy_settings",
  "list_inline_buttons",
  "press_inline_button",
  "save_draft",
  "get_drafts",
  "clear_draft",
  "search_public_chats",
  "search_messages",
  "resolve_username",
  "get_media_info",
  "get_recent_actions",
  "create_poll",
  "get_bot_info",
  "set_bot_commands",
  "set_profile_photo",
  "delete_profile_photo",
  "edit_chat_photo",
  "get_sticker_sets",
  "get_gif_search",
  "get_contact_ids",
  "import_contacts",
  "export_contacts",
  "get_direct_chat_by_contact",
  "get_contact_chats",
  "get_last_interaction",
] as const;

export type TelegramMCPToolName = (typeof TELEGRAM_MCP_TOOL_NAMES)[number];
