/**
 * Tool Action Parser - Converts tool inputs to human-readable descriptions
 */

type ToolArguments = Record<string, unknown>;

function formatId(id: string | number | undefined, prefix = ''): string {
  if (!id) return '';
  const str = String(id);
  return prefix ? `${prefix} ${str}` : str;
}

function truncateText(text: string, maxLength = 50): string {
  if (text.length <= maxLength) return text;
  return `${text.substring(0, maxLength)}...`;
}

export function toHumanReadableAction(toolName: string, args: ToolArguments): string {
  const parser = toolParsers[toolName];
  if (parser) return parser(args);
  return `Executing ${toolName} with provided parameters`;
}

const toolParsers: Record<string, (args: ToolArguments) => string> = {
  send_message: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const message = truncateText((args.message as string) || '', 100);
    return `Send message to ${chatId}: "${message}"`;
  },
  reply_to_message: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const messageId = args.message_id;
    const text = truncateText((args.text as string) || '', 100);
    return `Reply to message ${messageId} in ${chatId}: "${text}"`;
  },
  edit_message: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const messageId = args.message_id;
    const newText = truncateText((args.new_text as string) || '', 100);
    return `Edit message ${messageId} in ${chatId} to: "${newText}"`;
  },
  delete_message: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const messageId = args.message_id;
    return `Delete message ${messageId} from ${chatId}`;
  },
  forward_message: (args) => {
    const fromChat = formatId(args.from_chat_id as string | number, 'chat');
    const toChat = formatId(args.to_chat_id as string | number, 'chat');
    const messageId = args.message_id;
    return `Forward message ${messageId} from ${fromChat} to ${toChat}`;
  },
  pin_message: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const messageId = args.message_id;
    return `Pin message ${messageId} in ${chatId}`;
  },
  unpin_message: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const messageId = args.message_id;
    return `Unpin message ${messageId} in ${chatId}`;
  },
  mark_as_read: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    return `Mark messages as read in ${chatId}`;
  },
  send_reaction: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const messageId = args.message_id;
    const reaction = (args.reaction as string) || '👍';
    return `Add reaction ${reaction} to message ${messageId} in ${chatId}`;
  },
  remove_reaction: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const messageId = args.message_id;
    const reaction = (args.reaction as string) || '';
    return `Remove reaction ${reaction} from message ${messageId} in ${chatId}`;
  },
  save_draft: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const text = truncateText((args.text as string) || '', 50);
    return `Save draft in ${chatId}: "${text}"`;
  },
  clear_draft: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    return `Clear draft in ${chatId}`;
  },
  list_chats: (args) => {
    const chatType = (args.chat_type as string) || 'all';
    const limit = (args.limit as number) || 20;
    return `List ${limit} ${chatType} chats`;
  },
  get_chat: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    return `Get information for ${chatId}`;
  },
  create_group: (args) => {
    const title = (args.title as string) || 'Untitled Group';
    const userCount = Array.isArray(args.user_ids) ? args.user_ids.length : 0;
    return `Create group "${title}" with ${userCount} member${userCount !== 1 ? 's' : ''}`;
  },
  create_channel: (args) => {
    const title = (args.title as string) || 'Untitled Channel';
    const description = args.description
      ? `: ${truncateText((args.description as string) || '', 50)}`
      : '';
    return `Create channel "${title}"${description}`;
  },
  edit_chat_title: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const newTitle = (args.new_title as string) || '';
    return `Change title of ${chatId} to "${newTitle}"`;
  },
  delete_chat_photo: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    return `Remove photo from ${chatId}`;
  },
  edit_chat_photo: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    return `Update photo for ${chatId}`;
  },
  leave_chat: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    return `Leave ${chatId}`;
  },
  mute_chat: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const muteFor = args.mute_for ? ` for ${args.mute_for} seconds` : '';
    return `Mute ${chatId}${muteFor}`;
  },
  unmute_chat: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    return `Unmute ${chatId}`;
  },
  archive_chat: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    return `Archive ${chatId}`;
  },
  unarchive_chat: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    return `Unarchive ${chatId}`;
  },
  invite_to_group: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const userCount = Array.isArray(args.user_ids) ? args.user_ids.length : 0;
    return `Invite ${userCount} user${userCount !== 1 ? 's' : ''} to ${chatId}`;
  },
  ban_user: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const userId = formatId(args.user_id as string | number, 'user');
    return `Ban ${userId} from ${chatId}`;
  },
  unban_user: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const userId = formatId(args.user_id as string | number, 'user');
    return `Unban ${userId} from ${chatId}`;
  },
  promote_admin: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const userId = formatId(args.user_id as string | number, 'user');
    return `Promote ${userId} to admin in ${chatId}`;
  },
  demote_admin: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const userId = formatId(args.user_id as string | number, 'user');
    return `Demote ${userId} from admin in ${chatId}`;
  },
  block_user: (args) => {
    const userId = formatId(args.user_id as string | number, 'user');
    return `Block ${userId}`;
  },
  unblock_user: (args) => {
    const userId = formatId(args.user_id as string | number, 'user');
    return `Unblock ${userId}`;
  },
  add_contact: (args) => {
    const firstName = (args.first_name as string) || '';
    const lastName = (args.last_name as string) || '';
    const phone = (args.phone_number as string) || '';
    const name = [firstName, lastName].filter(Boolean).join(' ') || phone;
    return `Add contact: ${name}`;
  },
  delete_contact: (args) => {
    const userId = formatId(args.user_id as string | number, 'user');
    return `Delete contact ${userId}`;
  },
  list_contacts: (args) => {
    const limit = (args.limit as number) || 20;
    return `List ${limit} contacts`;
  },
  search_contacts: (args) => {
    const query = truncateText((args.query as string) || '', 30);
    return `Search contacts for "${query}"`;
  },
  search_messages: (args) => {
    const chatId = args.chat_id ? formatId(args.chat_id as string | number, 'chat') : 'all chats';
    const query = truncateText((args.query as string) || '', 30);
    const limit = (args.limit as number) || 20;
    return `Search for "${query}" in ${chatId} (limit: ${limit})`;
  },
  search_public_chats: (args) => {
    const query = truncateText((args.query as string) || '', 30);
    return `Search public chats for "${query}"`;
  },
  resolve_username: (args) => {
    const username = (args.username as string) || '';
    return `Resolve username @${username.replace('@', '')}`;
  },
  get_messages: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const limit = (args.limit as number) || 20;
    return `Get ${limit} messages from ${chatId}`;
  },
  list_messages: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const limit = (args.limit as number) || 20;
    return `List ${limit} messages from ${chatId}`;
  },
  get_history: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const limit = (args.limit as number) || 20;
    return `Get message history from ${chatId} (${limit} messages)`;
  },
  get_pinned_messages: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    return `Get pinned messages from ${chatId}`;
  },
  get_message_context: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const messageId = args.message_id;
    const limit = (args.limit as number) || 5;
    return `Get context around message ${messageId} in ${chatId} (±${limit} messages)`;
  },
  list_topics: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    return `List topics in ${chatId}`;
  },
  get_participants: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const limit = (args.limit as number) || 100;
    return `Get ${limit} participants from ${chatId}`;
  },
  get_admins: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    return `Get administrators of ${chatId}`;
  },
  get_banned_users: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    return `Get banned users from ${chatId}`;
  },
  get_blocked_users: () => 'Get list of blocked users',
  get_invite_link: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    return `Get invite link for ${chatId}`;
  },
  export_chat_invite: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    return `Create invite link for ${chatId}`;
  },
  import_chat_invite: (args) => {
    const inviteHash = args.invite_hash ? truncateText((args.invite_hash as string) || '', 20) : '';
    return `Join chat via invite link: ${inviteHash}`;
  },
  join_chat_by_link: (args) => {
    const inviteLink = args.invite_link ? truncateText((args.invite_link as string) || '', 30) : '';
    return `Join chat via link: ${inviteLink}`;
  },
  subscribe_public_channel: (args) => {
    const username = (args.username as string) || '';
    return `Subscribe to public channel @${username.replace('@', '')}`;
  },
  update_profile: (args) => {
    const updates: string[] = [];
    if (args.first_name) updates.push(`first name: "${args.first_name as string}"`);
    if (args.last_name) updates.push(`last name: "${args.last_name as string}"`);
    if (args.bio) updates.push(`bio: "${truncateText((args.bio as string) || '', 30)}"`);
    return `Update profile (${updates.join(', ')})`;
  },
  set_profile_photo: () => 'Set profile photo',
  delete_profile_photo: () => 'Delete profile photo',
  get_user_photos: (args) => {
    const userId = formatId(args.user_id as string | number, 'user');
    const limit = (args.limit as number) || 20;
    return `Get ${limit} photos from ${userId}`;
  },
  get_user_status: (args) => {
    const userId = formatId(args.user_id as string | number, 'user');
    return `Get status of ${userId}`;
  },
  get_me: () => 'Get current user information',
  list_inline_buttons: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const messageId = args.message_id;
    return `Get inline buttons from message ${messageId} in ${chatId}`;
  },
  press_inline_button: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const messageId = args.message_id;
    const buttonText = args.button_text ? ` "${truncateText((args.button_text as string) || '', 30)}"` : '';
    return `Press inline button${buttonText} on message ${messageId} in ${chatId}`;
  },
  create_poll: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const question = truncateText((args.question as string) || '', 50);
    const optionCount = Array.isArray(args.options) ? args.options.length : 0;
    return `Create poll in ${chatId}: "${question}" with ${optionCount} options`;
  },
  get_bot_info: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    return `Get bot information from ${chatId}`;
  },
  set_bot_commands: (args) => {
    const chatId = args.chat_id ? formatId(args.chat_id as string | number, 'chat') : 'all chats';
    const commandCount = Array.isArray(args.commands) ? args.commands.length : 0;
    return `Set ${commandCount} bot commands for ${chatId}`;
  },
  get_privacy_settings: () => 'Get privacy settings',
  set_privacy_settings: (args) => {
    const setting = (args.setting as string) || 'unknown';
    const value = (args.value as string) || '';
    return `Set privacy setting "${setting}" to ${value}`;
  },
  get_media_info: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const messageId = args.message_id;
    return `Get media information from message ${messageId} in ${chatId}`;
  },
  get_recent_actions: (args) => {
    const chatId = formatId(args.chat_id as string | number, 'chat');
    const limit = (args.limit as number) || 20;
    return `Get ${limit} recent admin actions from ${chatId}`;
  },
};
