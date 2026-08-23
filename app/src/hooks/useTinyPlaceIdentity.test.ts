import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { __resetTinyPlaceIdentityForTests, useTinyPlaceIdentity } from './useTinyPlaceIdentity';

const selfIdentity = vi.fn();
vi.mock('../lib/orchestration/orchestrationClient', () => ({
  orchestrationClient: { selfIdentity: () => selfIdentity() },
}));

describe('useTinyPlaceIdentity (#5424)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    __resetTinyPlaceIdentityForTests();
  });

  afterEach(() => {
    __resetTinyPlaceIdentityForTests();
    vi.useRealTimers();
  });

  it('reports an identity when the RPC returns a non-empty agentId', async () => {
    selfIdentity.mockResolvedValue({ agentId: 'agent-123', handles: [], discoverable: true });
    const { result } = renderHook(() => useTinyPlaceIdentity());

    await waitFor(() => expect(result.current.status).toBe('ready'));
    expect(result.current.hasIdentity).toBe(true);
  });

  it('reports no identity when the agentId is blank', async () => {
    selfIdentity.mockResolvedValue({ agentId: '   ', handles: [], discoverable: false });
    const { result } = renderHook(() => useTinyPlaceIdentity());

    await waitFor(() => expect(result.current.status).toBe('ready'));
    expect(result.current.hasIdentity).toBe(false);
  });

  it('stays fail-closed (hidden) while a rejected check is retried', async () => {
    selfIdentity.mockRejectedValue(new Error('wallet locked'));
    const { result } = renderHook(() => useTinyPlaceIdentity());

    await waitFor(() => expect(result.current.status).toBe('ready'));
    expect(result.current.hasIdentity).toBe(false);
  });

  it('fetches once and shares the result across consumers', async () => {
    selfIdentity.mockResolvedValue({ agentId: 'agent-123', handles: [], discoverable: true });
    const first = renderHook(() => useTinyPlaceIdentity());
    const second = renderHook(() => useTinyPlaceIdentity());

    await waitFor(() => expect(first.result.current.status).toBe('ready'));
    expect(second.result.current.hasIdentity).toBe(true);
    // A single resolved RPC backs every consumer for the app session.
    expect(selfIdentity).toHaveBeenCalledTimes(1);
  });

  it('retries after a transient failure and recovers without a restart (#5439 review)', async () => {
    vi.useFakeTimers();
    selfIdentity
      .mockRejectedValueOnce(new Error('wallet locked at startup'))
      .mockResolvedValue({ agentId: 'agent-123', handles: [], discoverable: true });
    const { result } = renderHook(() => useTinyPlaceIdentity());

    // First attempt fails → fail-closed (hidden), but a backoff retry is queued.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(result.current).toEqual({ status: 'ready', hasIdentity: false });

    // The retry fires and succeeds → a real holder is no longer locked out.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(2000);
    });
    expect(result.current.hasIdentity).toBe(true);
  });

  it('re-checks immediately on window focus after a failed startup check (#5439 review)', async () => {
    selfIdentity
      .mockRejectedValueOnce(new Error('wallet locked'))
      .mockResolvedValue({ agentId: 'agent-123', handles: [], discoverable: true });
    const { result } = renderHook(() => useTinyPlaceIdentity());

    await waitFor(() => expect(result.current.status).toBe('ready'));
    expect(result.current.hasIdentity).toBe(false);

    // The user unlocks their wallet and returns to the app.
    await act(async () => {
      window.dispatchEvent(new Event('focus'));
    });
    await waitFor(() => expect(result.current.hasIdentity).toBe(true));
  });
});
