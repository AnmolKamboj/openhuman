import { invoke } from '@tauri-apps/api/core';
import { platform } from '@tauri-apps/plugin-os';
import { useEffect, useRef } from 'react';

import { useAppDispatch, useAppSelector } from '../store/hooks';
import {
  setDownloadTriggered,
  setModelError,
  setModelLoading,
  setModelStatus,
  type ModelStatus,
} from '../store/modelSlice';

const POLL_INTERVAL = 1000;

/**
 * App-level provider that auto-starts model download on desktop
 * and keeps Redux model state in sync with the Rust backend.
 */
const ModelProvider = ({ children }: { children: React.ReactNode }) => {
  const dispatch = useAppDispatch();
  const { loading, downloaded, available, downloadTriggered } = useAppSelector(
    state => state.model
  );
  const pollingRef = useRef(false);

  // Initial status fetch + availability check
  useEffect(() => {
    const init = async () => {
      try {
        const status = await invoke<ModelStatus>('model_get_status');
        dispatch(setModelStatus(status));
      } catch {
        // Non-Tauri environment
      }

      try {
        const avail = await invoke<boolean>('model_is_available');
        if (avail) {
          dispatch(setModelStatus({ ...(await invoke<ModelStatus>('model_get_status')) }));
        }
      } catch {
        // Non-Tauri environment
      }
    };
    init();
  }, [dispatch]);

  // Auto-trigger download on desktop when model is available but not downloaded
  useEffect(() => {
    if (downloadTriggered || !available || downloaded || loading) return;

    const tryAutoDownload = async () => {
      try {
        const currentPlatform = await platform();
        if (currentPlatform === 'android' || currentPlatform === 'ios') return;
      } catch {
        // Can't detect platform — skip auto-download (likely web)
        return;
      }

      dispatch(setDownloadTriggered(true));
      dispatch(setModelLoading(true));
      dispatch(setModelError(null));

      try {
        await invoke('model_start_download');
        const status = await invoke<ModelStatus>('model_get_status');
        dispatch(setModelStatus(status));
      } catch (err) {
        dispatch(setModelError(err instanceof Error ? err.message : 'Failed to download model'));
      }
    };

    tryAutoDownload();
  }, [dispatch, downloadTriggered, available, downloaded, loading]);

  // Poll status while loading/downloading
  useEffect(() => {
    if (!loading) {
      pollingRef.current = false;
      return;
    }

    pollingRef.current = true;

    const interval = setInterval(async () => {
      if (!pollingRef.current) return;
      try {
        const status = await invoke<ModelStatus>('model_get_status');
        dispatch(setModelStatus(status));
        if (!status.loading) {
          pollingRef.current = false;
        }
      } catch {
        // ignore
      }
    }, POLL_INTERVAL);

    return () => clearInterval(interval);
  }, [dispatch, loading]);

  return <>{children}</>;
};

export default ModelProvider;
