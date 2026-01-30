/**
 * Telegram state update layer.
 *
 * Functions in this module dispatch Redux actions to keep the store
 * in sync when API read functions fetch fresh data from Telegram.
 */

export { getCurrentUserId } from "./helpers";

export {
  updateChatsInState,
  updateChatInState,
  updateChatFromResult,
  updateChatsFromResults,
} from "./chatState";

export {
  updateMessagesInState,
  updateMessageReactionsInState,
} from "./messageState";

export {
  updateCurrentUserInState,
  updateUsersInState,
  updateUsersFromApiUsers,
} from "./userState";
