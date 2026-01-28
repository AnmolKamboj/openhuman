/**
 * MCP Tools for Telegram
 * Each tool exports a 'tool' definition and a handler function
 */

export { getChats, tool as getChatsTool } from './getChats';
export { listChats, tool as listChatsTool } from './listChats';
export { getChat, tool as getChatTool } from './getChat';
export { createGroup, tool as createGroupTool } from './createGroup';
export { inviteToGroup, tool as inviteToGroupTool } from './inviteToGroup';
export { createChannel, tool as createChannelTool } from './createChannel';
export { editChatTitle, tool as editChatTitleTool } from './editChatTitle';
export { deleteChatPhoto, tool as deleteChatPhotoTool } from './deleteChatPhoto';
export { leaveChat, tool as leaveChatTool } from './leaveChat';
export { getParticipants, tool as getParticipantsTool } from './getParticipants';
export { getAdmins, tool as getAdminsTool } from './getAdmins';
export { getBannedUsers, tool as getBannedUsersTool } from './getBannedUsers';
export { promoteAdmin, tool as promoteAdminTool } from './promoteAdmin';
export { demoteAdmin, tool as demoteAdminTool } from './demoteAdmin';
export { banUser, tool as banUserTool } from './banUser';
export { unbanUser, tool as unbanUserTool } from './unbanUser';
export { getInviteLink, tool as getInviteLinkTool } from './getInviteLink';
export { exportChatInvite, tool as exportChatInviteTool } from './exportChatInvite';
export { importChatInvite, tool as importChatInviteTool } from './importChatInvite';
export { joinChatByLink, tool as joinChatByLinkTool } from './joinChatByLink';
export { subscribePublicChannel, tool as subscribePublicChannelTool } from './subscribePublicChannel';
export { muteChat, tool as muteChatTool } from './muteChat';
export { unmuteChat, tool as unmuteChatTool } from './unmuteChat';
export { archiveChat, tool as archiveChatTool } from './archiveChat';
export { unarchiveChat, tool as unarchiveChatTool } from './unarchiveChat';

export { getMessages, tool as getMessagesTool } from './getMessages';
export { listMessages, tool as listMessagesTool } from './listMessages';
export { listTopics, tool as listTopicsTool } from './listTopics';
export { sendMessage, tool as sendMessageTool } from './sendMessage';
export { replyToMessage, tool as replyToMessageTool } from './replyToMessage';
export { editMessage, tool as editMessageTool } from './editMessage';
export { deleteMessage, tool as deleteMessageTool } from './deleteMessage';
export { forwardMessage, tool as forwardMessageTool } from './forwardMessage';
export { pinMessage, tool as pinMessageTool } from './pinMessage';
export { unpinMessage, tool as unpinMessageTool } from './unpinMessage';
export { markAsRead, tool as markAsReadTool } from './markAsRead';
export { getMessageContext, tool as getMessageContextTool } from './getMessageContext';
export { getHistory, tool as getHistoryTool } from './getHistory';
export { getPinnedMessages, tool as getPinnedMessagesTool } from './getPinnedMessages';
export { sendReaction, tool as sendReactionTool } from './sendReaction';
export { removeReaction, tool as removeReactionTool } from './removeReaction';
export { getMessageReactions, tool as getMessageReactionsTool } from './getMessageReactions';
export { listInlineButtons, tool as listInlineButtonsTool } from './listInlineButtons';
export { pressInlineButton, tool as pressInlineButtonTool } from './pressInlineButton';
export { saveDraft, tool as saveDraftTool } from './saveDraft';
export { getDrafts, tool as getDraftsTool } from './getDrafts';
export { clearDraft, tool as clearDraftTool } from './clearDraft';

export { listContacts, tool as listContactsTool } from './listContacts';
export { searchContacts, tool as searchContactsTool } from './searchContacts';
export { addContact, tool as addContactTool } from './addContact';
export { deleteContact, tool as deleteContactTool } from './deleteContact';
export { blockUser, tool as blockUserTool } from './blockUser';
export { unblockUser, tool as unblockUserTool } from './unblockUser';
export { getBlockedUsers, tool as getBlockedUsersTool } from './getBlockedUsers';

export { getMe, tool as getMeTool } from './getMe';
export { updateProfile, tool as updateProfileTool } from './updateProfile';
export { getUserPhotos, tool as getUserPhotosTool } from './getUserPhotos';
export { getUserStatus, tool as getUserStatusTool } from './getUserStatus';

export { searchPublicChats, tool as searchPublicChatsTool } from './searchPublicChats';
export { searchMessages, tool as searchMessagesTool } from './searchMessages';
export { resolveUsername, tool as resolveUsernameTool } from './resolveUsername';

export { getMediaInfo, tool as getMediaInfoTool } from './getMediaInfo';
export { getRecentActions, tool as getRecentActionsTool } from './getRecentActions';
export { createPoll, tool as createPollTool } from './createPoll';
export { getBotInfo, tool as getBotInfoTool } from './getBotInfo';
export { setBotCommands, tool as setBotCommandsTool } from './setBotCommands';

export { getPrivacySettings, tool as getPrivacySettingsTool } from './getPrivacySettings';
export { setPrivacySettings, tool as setPrivacySettingsTool } from './setPrivacySettings';

export { setProfilePhoto, tool as setProfilePhotoTool } from './setProfilePhoto';
export { deleteProfilePhoto, tool as deleteProfilePhotoTool } from './deleteProfilePhoto';
export { editChatPhoto, tool as editChatPhotoTool } from './editChatPhoto';

export { getStickerSets, tool as getStickerSetsTool } from './getStickerSets';
export { getGifSearch, tool as getGifSearchTool } from './getGifSearch';

export { getContactIds, tool as getContactIdsTool } from './getContactIds';
export { importContacts, tool as importContactsTool } from './importContacts';
export { exportContacts, tool as exportContactsTool } from './exportContacts';
export { getDirectChatByContact, tool as getDirectChatByContactTool } from './getDirectChatByContact';
export { getContactChats, tool as getContactChatsTool } from './getContactChats';
export { getLastInteraction, tool as getLastInteractionTool } from './getLastInteraction';
