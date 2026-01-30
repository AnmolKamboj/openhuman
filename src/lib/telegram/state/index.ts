/**
 * Telegram state update layer.
 *
 * Functions in this module dispatch Redux actions to keep the store
 * in sync when API read functions fetch fresh data from Telegram.
 */

export { getCurrentUserId, resolveUserId } from "./helpers";

export {
  updateChatsInState,
  updateChatInState,
  updateChatFromResult,
  updateChatsFromResults,
  updateChatFieldsInState,
  replaceAllChatsInState,
  addChatsWithUsersInState,
} from "./chatState";

export {
  updateMessagesInState,
  updateMessageReactionsInState,
  addMessageToState,
  updateMessageFieldsInState,
  deleteMessagesFromState,
  findAndDeleteMessagesFromState,
} from "./messageState";

export {
  updateCurrentUserInState,
  updateUsersInState,
  updateUsersFromApiUsers,
  replaceAllUsersInState,
} from "./userState";

export {
  updateSyncStatusInState,
  updateCommonBoxStateInState,
  updateChannelPtsInState,
} from "./syncState";
