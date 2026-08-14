import { renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { useNavTabs } from './useNavTabs';
import type { TinyPlaceIdentityState } from './useTinyPlaceIdentity';

// Hoisted so the vi.mock factory can legally reference it (the repo convention
// for controllable mock state — see Brain.test.tsx).
const identityRef = vi.hoisted(() => ({
  current: { status: 'ready', hasIdentity: false } as TinyPlaceIdentityState,
}));
vi.mock('./useTinyPlaceIdentity', () => ({ useTinyPlaceIdentity: () => identityRef.current }));

describe('useNavTabs (#5424)', () => {
  beforeEach(() => {
    identityRef.current = { status: 'ready', hasIdentity: false };
  });

  it('hides the agent-world (tiny.place) tab when the user has no identity', () => {
    identityRef.current = { status: 'ready', hasIdentity: false };
    const { result } = renderHook(() => useNavTabs());

    expect(result.current.some(tab => tab.id === 'agent-world')).toBe(false);
    // The other primary tabs are untouched.
    expect(result.current.some(tab => tab.id === 'chat')).toBe(true);
  });

  it('shows the agent-world tab for a user with a tiny.place identity', () => {
    identityRef.current = { status: 'ready', hasIdentity: true };
    const { result } = renderHook(() => useNavTabs());

    expect(result.current.some(tab => tab.id === 'agent-world')).toBe(true);
  });

  it('keeps the tab hidden while the identity check is still loading', () => {
    identityRef.current = { status: 'loading', hasIdentity: false };
    const { result } = renderHook(() => useNavTabs());

    expect(result.current.some(tab => tab.id === 'agent-world')).toBe(false);
  });
});
