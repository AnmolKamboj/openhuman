import { renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

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

  it('fails closed (no identity) when the RPC rejects — e.g. a locked wallet', async () => {
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
    // A single RPC backs every consumer for the app session.
    expect(selfIdentity).toHaveBeenCalledTimes(1);
  });
});
