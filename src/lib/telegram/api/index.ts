/**
 * Barrel export for the Telegram API layer.
 *
 * Each module exposes a single async function returning ApiResult<T>.
 */

// Foundation
export type { ApiResult } from "./types";
export * from "./helpers";
export * from "./apiResultTypes";
export * from "./apiCastHelpers";

// Cache-first with API fallback
export { getChats } from "./getChats";
export { getMessages } from "./getMessages";
export { getCurrentUser } from "./getCurrentUser";

// Hybrid (API-first with cache fallback)
export { getChat } from "./getChat";
export { resolveUsername } from "./resolveUsername";
export { searchPublicChats } from "./searchPublicChats";
export { searchMessages } from "./searchMessages";
export { getPinnedMessages } from "./getPinnedMessages";

// Cache-only
export { getContactChats } from "./getContactChats";
export { getDirectChatByContact } from "./getDirectChatByContact";
export { getLastInteraction } from "./getLastInteraction";
export { listInlineButtons } from "./listInlineButtons";
export { searchChats } from "./searchChats";
export { getMediaInfo } from "./getMediaInfo";

// Write operations — Chat management
export { sendMessage } from "./sendMessage";
export { archiveChat } from "./archiveChat";
export { unarchiveChat } from "./unarchiveChat";
export { createChannel } from "./createChannel";
export { createGroup } from "./createGroup";
export { editChatTitle } from "./editChatTitle";
export { leaveChat } from "./leaveChat";
export { muteChat } from "./muteChat";
export { unmuteChat } from "./unmuteChat";
export { deleteChatPhoto } from "./deleteChatPhoto";
export { editChatPhoto } from "./editChatPhoto";
export { exportChatInvite } from "./exportChatInvite";
export { importChatInvite } from "./importChatInvite";
export { joinChatByLink } from "./joinChatByLink";
export { subscribePublicChannel } from "./subscribePublicChannel";
export { getInviteLink } from "./getInviteLink";

// Write operations — Message ops
export { editMessage } from "./editMessage";
export { deleteMessage } from "./deleteMessage";
export { forwardMessage } from "./forwardMessage";
export { pinMessage } from "./pinMessage";
export { unpinMessage } from "./unpinMessage";
export { sendReaction } from "./sendReaction";
export { removeReaction } from "./removeReaction";
export { getMessageReactions } from "./getMessageReactions";
export { pressInlineButton } from "./pressInlineButton";
export { saveDraft } from "./saveDraft";
export { getDrafts } from "./getDrafts";
export { clearDraft } from "./clearDraft";
export { createPoll } from "./createPoll";
export { markAsRead } from "./markAsRead";

// Write operations — User/admin
export { getParticipants } from "./getParticipants";
export { getAdmins } from "./getAdmins";
export { getBannedUsers } from "./getBannedUsers";
export { promoteAdmin } from "./promoteAdmin";
export { demoteAdmin } from "./demoteAdmin";
export { banUser } from "./banUser";
export { unbanUser } from "./unbanUser";
export { inviteToGroup } from "./inviteToGroup";

// Write operations — Contacts
export { listContacts } from "./listContacts";
export { searchContacts } from "./searchContacts";
export { addContact } from "./addContact";
export { deleteContact } from "./deleteContact";
export { blockUser } from "./blockUser";
export { unblockUser } from "./unblockUser";
export { getBlockedUsers } from "./getBlockedUsers";
export { getContactIds } from "./getContactIds";
export { importContacts } from "./importContacts";
export { exportContacts } from "./exportContacts";

// Write operations — Profile/settings
export { updateProfile } from "./updateProfile";
export { getUserPhotos } from "./getUserPhotos";
export { getUserStatus } from "./getUserStatus";
export { getPrivacySettings } from "./getPrivacySettings";
export { setPrivacySettings } from "./setPrivacySettings";
export { setProfilePhoto } from "./setProfilePhoto";
export { deleteProfilePhoto } from "./deleteProfilePhoto";
export { setBotCommands } from "./setBotCommands";
export { getBotInfo } from "./getBotInfo";

// Read operations — Search/discovery
export { getRecentActions } from "./getRecentActions";
export { getStickerSets } from "./getStickerSets";
export { getGifSearch } from "./getGifSearch";
export { listTopics } from "./listTopics";
