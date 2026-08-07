import { renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { useNavTabs } from './useNavTabs';
import type { TinyPlaceIdentityState } from './useTinyPlaceIdentity';

let identity: TinyPlaceIdentityState = { status: 'ready', hasIdentity: false };
vi.mock('./useTinyPlaceIdentity', () => ({ useTinyPlaceIdentity: () => identity }));

describe('useNavTabs (#5424)', () => {
  beforeEach(() => {
    identity = { status: 'ready', hasIdentity: false };
  });

  it('hides the agent-world (tiny.place) tab when the user has no identity', () => {
    identity = { status: 'ready', hasIdentity: false };
    const { result } = renderHook(() => useNavTabs());

    expect(result.current.some(tab => tab.id === 'agent-world')).toBe(false);
    // The other primary tabs are untouched.
    expect(result.current.some(tab => tab.id === 'chat')).toBe(true);
  });

  it('shows the agent-world tab for a user with a tiny.place identity', () => {
    identity = { status: 'ready', hasIdentity: true };
    const { result } = renderHook(() => useNavTabs());

    expect(result.current.some(tab => tab.id === 'agent-world')).toBe(true);
  });

  it('keeps the tab hidden while the identity check is still loading', () => {
    identity = { status: 'loading', hasIdentity: false };
    const { result } = renderHook(() => useNavTabs());

    expect(result.current.some(tab => tab.id === 'agent-world')).toBe(false);
  });
});
