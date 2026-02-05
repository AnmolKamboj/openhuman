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
  const initDone = useRef(false);

  // Initial status fetch + availability check (runs once)
  useEffect(() => {
    if (initDone.current) return;
    initDone.current = true;

    const init = async () => {
      console.log('[ModelProvider] Initializing...');
      try {
        const status = await invoke<ModelStatus>('model_get_status');
        console.log('[ModelProvider] Initial status:', status);
        dispatch(setModelStatus(status));
      } catch (err) {
        console.log('[ModelProvider] model_get_status failed (non-Tauri?):', err);
        return; // Not a Tauri environment, nothing more to do
      }

      try {
        const avail = await invoke<boolean>('model_is_available');
        console.log('[ModelProvider] Model available:', avail);
        if (avail) {
          const status = await invoke<ModelStatus>('model_get_status');
          dispatch(setModelStatus(status));
        }
      } catch (err) {
        console.log('[ModelProvider] model_is_available failed:', err);
      }
    };
    init();
  }, [dispatch]);

  // Auto-trigger download on desktop when model is available but not downloaded
  useEffect(() => {
    if (downloadTriggered) {
      console.log('[ModelProvider] Auto-download: already triggered, skipping');
      return;
    }
    if (!available) {
      console.log('[ModelProvider] Auto-download: not available yet, waiting');
      return;
    }
    if (downloaded) {
      console.log('[ModelProvider] Auto-download: already downloaded, skipping');
      return;
    }
    if (loading) {
      console.log('[ModelProvider] Auto-download: already loading, skipping');
      return;
    }

    const tryAutoDownload = async () => {
      try {
        const currentPlatform = await platform();
        console.log('[ModelProvider] Platform:', currentPlatform);
        if (currentPlatform === 'android' || currentPlatform === 'ios') {
          console.log('[ModelProvider] Mobile platform, skipping auto-download');
          return;
        }
      } catch (err) {
        console.log('[ModelProvider] Platform detection failed (likely web), skipping:', err);
        return;
      }

      console.log('[ModelProvider] Starting auto-download...');
      dispatch(setDownloadTriggered(true));
      dispatch(setModelLoading(true));
      dispatch(setModelError(null));

      try {
        await invoke('model_start_download');
        const status = await invoke<ModelStatus>('model_get_status');
        console.log('[ModelProvider] Download started, status:', status);
        dispatch(setModelStatus(status));
      } catch (err) {
        console.error('[ModelProvider] Auto-download failed:', err);
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
    console.log('[ModelProvider] Polling started (loading=true)');

    const interval = setInterval(async () => {
      if (!pollingRef.current) return;
      try {
        const status = await invoke<ModelStatus>('model_get_status');
        dispatch(setModelStatus(status));
        if (!status.loading) {
          console.log('[ModelProvider] Polling stopped (loading done)', status);
          pollingRef.current = false;
        }
      } catch {
        // ignore polling errors
      }
    }, POLL_INTERVAL);

    return () => clearInterval(interval);
  }, [dispatch, loading]);

  return <>{children}</>;
};

export default ModelProvider;
