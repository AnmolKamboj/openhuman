/**
 * State update functions for Telegram sync infrastructure.
 *
 * Wraps Redux actions for sync status, common box state (pts/seq/qts/date),
 * and per-channel PTS tracking.
 */

import { store } from "../../../store";
import {
  setSyncStatus,
  setCommonBoxState,
  setChannelPts,
} from "../../../store/telegram";
import { resolveUserId } from "./helpers";

/**
 * Update the sync status flags in Redux state.
 */
export function updateSyncStatusInState(
  isSyncing: boolean,
  isSynced?: boolean,
  userId?: string
): void {
  const uid = resolveUserId(userId);
  if (!uid) return;

  const payload: { userId: string; isSyncing: boolean; isSynced?: boolean } = {
    userId: uid,
    isSyncing,
  };
  if (isSynced !== undefined) {
    payload.isSynced = isSynced;
  }
  store.dispatch(setSyncStatus(payload));
}

/**
 * Update the common update box state (seq/pts/qts/date) in Redux state.
 */
export function updateCommonBoxStateInState(
  commonBoxState: { seq: number; date: number; pts: number; qts: number },
  userId?: string
): void {
  const uid = resolveUserId(userId);
  if (!uid) return;
  store.dispatch(setCommonBoxState({ userId: uid, commonBoxState }));
}

/**
 * Update the PTS value for a specific channel in Redux state.
 */
export function updateChannelPtsInState(
  channelId: string,
  pts: number,
  userId?: string
): void {
  const uid = resolveUserId(userId);
  if (!uid) return;
  store.dispatch(setChannelPts({ userId: uid, channelId, pts }));
}
