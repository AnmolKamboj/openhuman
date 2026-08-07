/**
 * Whether this user has set up a tiny.place identity (#5424).
 *
 * tiny.place is being removed from the app after 31 August 2026. Its entry
 * points must be shown only to users who already have an identity — everyone
 * else never really started, so there is nothing to preserve and no reason to
 * advertise a feature that is about to disappear.
 *
 * The authoritative signal is the orchestration self-identity RPC: a non-empty
 * `agentId` means the wallet-backed identity exists. The RPC can reject when the
 * wallet is locked or unconfigured — that is exactly the "never set up" case, so
 * a rejection resolves to `hasIdentity: false` (fail-closed → hidden).
 *
 * The result is fetched once per app session and cached module-side, so the
 * several gates that read it (nav tabs, the agent-world route, the Brain
 * orchestration sub-tab, the notice) share a single RPC rather than each firing
 * their own.
 */
import { useEffect, useState } from 'react';

import { orchestrationClient } from '../lib/orchestration/orchestrationClient';

export interface TinyPlaceIdentityState {
  /** `loading` until the one-shot RPC settles; `ready` once resolved. */
  status: 'loading' | 'ready';
  /** True only when a tiny.place identity exists for this user. */
  hasIdentity: boolean;
}

let cache: TinyPlaceIdentityState = { status: 'loading', hasIdentity: false };
let started = false;
const listeners = new Set<() => void>();

function publish(next: TinyPlaceIdentityState) {
  cache = next;
  listeners.forEach(listener => listener());
}

async function load() {
  try {
    const identity = await orchestrationClient.selfIdentity();
    publish({ status: 'ready', hasIdentity: identity.agentId.trim().length > 0 });
  } catch {
    // Locked/unconfigured wallet or a degraded relay: treat as "no identity" so
    // the entry points stay hidden rather than flashing in on a transient error.
    publish({ status: 'ready', hasIdentity: false });
  }
}

/** Test seam — clears the module cache so each test starts from `loading`. */
export function __resetTinyPlaceIdentityForTests() {
  cache = { status: 'loading', hasIdentity: false };
  started = false;
  listeners.clear();
}

export function useTinyPlaceIdentity(): TinyPlaceIdentityState {
  const [state, setState] = useState<TinyPlaceIdentityState>(cache);

  useEffect(() => {
    const sync = () => setState(cache);
    listeners.add(sync);
    if (!started) {
      started = true;
      void load();
    }
    // Adopt the current cache in case it resolved between the initial render and
    // this effect firing (or a sibling hook already loaded it).
    sync();
    return () => {
      listeners.delete(sync);
    };
  }, []);

  return state;
}
