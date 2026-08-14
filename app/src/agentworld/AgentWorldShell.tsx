/**
 * AgentWorldShell — provider wrapper for the Agent World section.
 *
 * Provides:
 *   - The tiny.place API client (injected via ApiProvider so all nested hooks
 *     call through `createInvokeApiClient()` instead of the HTTP SDK).
 *   - Theme bridging (OpenHuman → Agent World colour tokens).
 *
 * Intentionally excludes WalletContext, MoonPay, and E2EAuthBridge that
 * appear in the tiny.place website's provider tree — those live in core or are
 * not needed in the embedded context.
 */
import type { ReactNode } from 'react';
import { Navigate } from 'react-router-dom';

import { useTinyPlaceIdentity } from '../hooks/useTinyPlaceIdentity';
import { createInvokeApiClient } from '../lib/agentworld/invokeApiClient';
import TinyPlaceSunsetNotice from './TinyPlaceSunsetNotice';

interface AgentWorldShellProps {
  children: ReactNode;
}

// One client instance per app lifetime (the underlying HTTP calls go through
// callCoreRpc which manages its own connection lifecycle).
const apiClient = createInvokeApiClient();

export default function AgentWorldShell({ children }: AgentWorldShellProps) {
  // #5424 — tiny.place is being removed from the app after 31 August 2026. A
  // user without an identity has no entry point to it, so a direct link here is
  // sent back to chat once we confirm they have none. Identity-holders keep full
  // access (direct links included) plus the removal notice. While the check is
  // in flight the surface renders optimistically so a holder never sees a flash.
  const { status, hasIdentity } = useTinyPlaceIdentity();
  if (status === 'ready' && !hasIdentity) {
    return <Navigate to="/chat" replace />;
  }

  // NOTE: When the vendored ApiProvider is available (from synced website/src),
  // wrap children with <ApiProvider client={apiClient}>.  For Wave 0 we expose
  // the client via a context (see AgentWorldContext) so the Explore placeholder
  // can demonstrate the end-to-end wiring without requiring the full vendor sync.
  void apiClient; // referenced here to ensure the module is evaluated
  return (
    <>
      <TinyPlaceSunsetNotice />
      {children}
    </>
  );
}

export { apiClient };
