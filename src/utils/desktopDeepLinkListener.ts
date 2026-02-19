import { isTauri as coreIsTauri, invoke } from '@tauri-apps/api/core';
import { getCurrent, onOpenUrl } from '@tauri-apps/plugin-deep-link';

import { skillManager } from '../lib/skills/manager';
import { consumeLoginToken, fetchIntegrationTokens } from '../services/api/authApi';
import { store } from '../store';
import { setToken } from '../store/authSlice';
import { setSkillState } from '../store/skillsSlice';

type IntegrationTokensPayload = {
  accessToken: string;
  refreshToken: string;
  /** ISO timestamp string */
  expiresAt: string;
};

function getCurrentUserId(): string | null {
  const state = store.getState();
  const explicitId = state.user.user?._id;
  if (explicitId) return explicitId;

  const token = state.auth.token;
  if (!token) return null;

  try {
    const parts = token.split('.');
    if (parts.length !== 3) return null;
    const payloadBase64 = parts[1].replace(/-/g, '+').replace(/_/g, '/');
    const payloadJson = atob(payloadBase64);
    const payload = JSON.parse(payloadJson);
    return payload.tgUserId || payload.userId || payload.sub || null;
  } catch {
    return null;
  }
}

function hexToBase64(hex: string): string {
  const bytes = hexToBytes(hex);
  if (bytes.length === 0) return '';
  let binary = '';
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

function hexToBytes(hex: string): Uint8Array {
  const cleanHex = hex.trim().replace(/^0x/i, '');
  if (!cleanHex) return new Uint8Array();
  const bytes = new Uint8Array(cleanHex.length / 2);
  for (let i = 0; i < cleanHex.length; i += 2) {
    bytes[i / 2] = parseInt(cleanHex.slice(i, i + 2), 16);
  }
  return bytes;
}

function base64ToBytes(b64: string): Uint8Array {
  // Normalize potential URL-safe base64 and missing padding
  let normalized = b64.replace(/-/g, '+').replace(/_/g, '/');
  const pad = normalized.length % 4;
  if (pad === 2) normalized += '==';
  else if (pad === 3) normalized += '=';

  const binary = atob(normalized);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

async function decryptIntegrationTokensWithKey(
  encryptedPayload: string,
  keyHex: string
): Promise<string> {
  if (typeof crypto === 'undefined' || !crypto.subtle) {
    throw new Error('Web Crypto API is not available for decryption');
  }

  const keyBytes = hexToBytes(keyHex);
  if (keyBytes.length === 0) {
    throw new Error('Invalid encryption key');
  }

  const combined = base64ToBytes(encryptedPayload);
  // Backend format: IV (16 bytes) + AuthTag (16 bytes) + EncryptedData (rest)
  if (combined.length <= 32) {
    throw new Error('Encrypted payload too short');
  }

  const iv = combined.slice(0, 16);
  const authTag = combined.slice(16, 32);
  const encryptedData = combined.slice(32);
  const ciphertextWithTag = new Uint8Array(encryptedData.length + authTag.length);
  ciphertextWithTag.set(encryptedData, 0);
  ciphertextWithTag.set(authTag, encryptedData.length);

  const cryptoKey = await crypto.subtle.importKey(
    'raw',
    keyBytes as unknown as BufferSource,
    { name: 'AES-GCM' },
    false,
    ['decrypt']
  );

  const decrypted = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv, tagLength: 128 },
    cryptoKey,
    ciphertextWithTag as unknown as BufferSource
  );

  return new TextDecoder().decode(decrypted);
}


/**
 * Handle an `alphahuman://auth?token=...` deep link for login.
 */
const handleAuthDeepLink = async (parsed: URL) => {
  const token = parsed.searchParams.get('token');
  if (!token) {
    console.warn('[DeepLink] URL did not contain a token query parameter');
    return;
  }

  console.log('[DeepLink] Received auth token');

  try {
    await invoke('show_window');
  } catch (err) {
    console.warn('[DeepLink] Failed to show window:', err);
  }

  const jwtToken = await consumeLoginToken(token);
  store.dispatch(setToken(jwtToken));
  window.location.hash = '/onboarding';
};

/**
 * Handle `alphahuman://oauth/success?integrationId=...&skillId=...`
 * and `alphahuman://oauth/error?error=...&provider=...` deep links.
 */
const handleOAuthDeepLink = async (parsed: URL) => {
  // pathname is "/success" or "/error" (hostname is "oauth")
  const path = parsed.pathname.replace(/^\/+/, '');

  try {
    await invoke('show_window');
  } catch {
    // Not fatal
  }

  if (path === 'success') {
    const integrationId = parsed.searchParams.get('integrationId');
    const skillId = parsed.searchParams.get('skillId');

    if (!integrationId || !skillId) {
      console.error('[DeepLink] OAuth success missing integrationId or skillId', parsed.href);
      return;
    }

    console.log(`[DeepLink] OAuth success for skill=${skillId} integration=${integrationId}`);

    try {

      const state = store.getState();
      const userId = getCurrentUserId();
      if (!userId) {
        console.warn('[DeepLink] Cannot fetch integration tokens: no current user id');
        return;
      }

      const encryptionKeyHex = state.auth.encryptionKeyByUser[userId];
      if (!encryptionKeyHex) {
        console.warn(
          '[DeepLink] Cannot fetch integration tokens: no encryption key found for user',
          userId
        );
        return;
      }

      const keyForBackend = hexToBase64(encryptionKeyHex);
      const response = await fetchIntegrationTokens(integrationId, keyForBackend || encryptionKeyHex);
      if (!response.success || !response.data?.encrypted) {
        console.warn(
          '[DeepLink] Integration tokens response missing encrypted payload for integration',
          integrationId
        );
        return;
      }

      let decryptedTokens: IntegrationTokensPayload;
      try {
        const plaintext = await decryptIntegrationTokensWithKey(
          response.data.encrypted,
          encryptionKeyHex
        );
        decryptedTokens = JSON.parse(plaintext) as IntegrationTokensPayload;
      } catch (err) {
        console.error('[DeepLink] Failed to decrypt integration tokens:', err);
        return;
      }

      const existingState = state.skills.skillStates[skillId] ?? {};
      store.dispatch(
        setSkillState({
          skillId,
          state: {
            ...existingState,
            oauthTokens: {
              ...(existingState.oauthTokens as Record<string, unknown> | undefined),
              [integrationId]: decryptedTokens,
            },
          },
        })
      );

      await skillManager.notifyOAuthComplete(skillId, integrationId);
    } catch (err) {
      console.error('[DeepLink] Failed to notify OAuth complete:', err);
    }
  } else if (path === 'error') {
    const error = parsed.searchParams.get('error') ?? 'Unknown error';
    const provider = parsed.searchParams.get('provider') ?? 'unknown';
    console.error(`[DeepLink] OAuth error for provider=${provider}: ${error}`);
  } else {
    console.warn('[DeepLink] Unknown OAuth path:', path);
  }
};

/**
 * Handle a list of deep link URLs delivered by the Tauri deep-link plugin.
 * Routes to the appropriate handler based on the URL hostname:
 *   - `alphahuman://auth?token=...` → login flow
 *   - `alphahuman://oauth/success?...` → OAuth completion
 *   - `alphahuman://oauth/error?...` → OAuth failure
 */
const handleDeepLinkUrls = async (urls: string[] | null | undefined) => {
  if (!urls || urls.length === 0) {
    return;
  }

  const url = urls[0];

  try {
    const parsed = new URL(url);
    if (parsed.protocol !== 'alphahuman:') {
      return;
    }

    switch (parsed.hostname) {
      case 'auth':
        await handleAuthDeepLink(parsed);
        break;
      case 'oauth':
        await handleOAuthDeepLink(parsed);
        break;
      default:
        console.warn('[DeepLink] Unknown deep link hostname:', parsed.hostname);
        break;
    }
  } catch (error) {
    console.error('[DeepLink] Failed to handle deep link URL:', url, error);
  }
};

/**
 * Set up listeners for deep links so that when the desktop app is opened
 * via a URL like `alphahuman://auth?token=...`, we can react to it.
 * Only works in Tauri desktop app environment.
 */
export const setupDesktopDeepLinkListener = async () => {
  // Only set up deep link listener in Tauri environment
  if (!coreIsTauri()) {
    return;
  }

  try {
    const startUrls = await getCurrent();
    if (startUrls) {
      await handleDeepLinkUrls(startUrls);
    }

    await onOpenUrl(urls => {
      void handleDeepLinkUrls(urls);
    });

    if (typeof window !== 'undefined') {
      // window.__simulateDeepLink('alphahuman://auth?token=1234567890')
      // window.__simulateDeepLink('alphahuman://oauth/success?integrationId=6989ef9c8e8bf1b6d991a08c&skillId=notion')
      (
        window as Window & { __simulateDeepLink?: (url: string) => Promise<void> }
      ).__simulateDeepLink = (url: string) => handleDeepLinkUrls([url]);
    }
  } catch (err) {
    console.error('[DeepLink] Setup failed:', err);
  }
};
