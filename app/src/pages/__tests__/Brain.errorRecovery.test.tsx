/**
 * Brain graph — transient-failure recovery, and the failure that is swallowed.
 *
 * `Brain.test.tsx` already covers "an alert appears when the fetch fails".
 * Neither of the two behaviours here is covered anywhere:
 *
 *   1. A transient failure must CLEAR on the next successful load. This is the
 *      accurate half of the "Couldn't load your brain" report — the panel is
 *      recoverable (`Brain.tsx:97` calls `setError(null)` at the top of every
 *      `load()`, and `MemoryControls` renders above the error branch so Refresh
 *      stays reachable), but nothing pinned that, so deleting that one line
 *      would have turned a recoverable error into a permanent one silently.
 *
 *   2. A refresh that fails AFTER a successful load shows the user nothing.
 *      `Brain.tsx:244-255` only reaches the alert when `graph` is null, and the
 *      catch at `:108-112` never clears `graph`, so a failed refresh leaves the
 *      stale graph on screen with no indication. That is BUG-W4-3, and the
 *      second test characterises it rather than asserting it is correct.
 *
 * Both drive the retry through the `openhuman:memory-tree-completed` window
 * event, which is the in-product refetch trigger (`Brain.tsx:113-117`) and
 * needs no DOM control.
 */
import { act, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { renderWithProviders } from '../../test/test-utils';
import Brain from '../Brain';

const graphExportMock = vi.hoisted(() => vi.fn());
// Controllable authenticated identity so we can simulate a logout→login cycle
// (userId null → set) and assert the graph reloads (#4149).
const coreAuthRef = vi.hoisted(() => ({ current: 'user-A' as string | null }));
const navigateSpy = vi.hoisted(() => vi.fn());

vi.mock('react-router-dom', async importOriginal => {
  const actual = await importOriginal<typeof import('react-router-dom')>();
  return { ...actual, useNavigate: () => navigateSpy };
});

vi.mock('../../utils/tauriCommands', () => ({
  memoryTreeGraphExport: graphExportMock,
  isTauri: () => false,
}));

vi.mock('../../providers/CoreStateProvider', () => ({
  useCoreState: () => ({
    snapshot: {
      auth: { userId: coreAuthRef.current, isAuthenticated: coreAuthRef.current != null },
    },
  }),
}));

vi.mock('../../components/intelligence/MemoryGraph', async () => {
  const React = await import('react');
  return {
    MemoryGraph: ({ nodes }: { nodes: unknown[] }) =>
      React.createElement('div', { 'data-testid': 'memory-graph' }, `nodes:${nodes.length}`),
  };
});

vi.mock('../../lib/i18n/I18nContext', () => ({ useT: () => ({ t: (k: string) => k }) }));

vi.mock('../../components/layout/ChipTabs', async () => {
  const React = await import('react');
  return {
    default: ({ children }: { children?: React.ReactNode }) =>
      React.createElement('div', null, children),
  };
});
vi.mock('../../components/ui/BetaBanner', () => ({ default: () => null }));
vi.mock('../../components/intelligence/MemoryControls', () => ({ MemoryControls: () => null }));
vi.mock('../../components/intelligence/MemoryTreeStatusPanel', async () => {
  const React = await import('react');
  return {
    MemoryTreeStatusPanel: () => React.createElement('div', { 'data-testid': 'brain-sync' }),
  };
});
vi.mock('../../components/intelligence/MemorySourcesRegistry', async () => {
  const React = await import('react');
  return {
    MemorySourcesRegistry: () => React.createElement('div', { 'data-testid': 'brain-sources' }),
  };
});
vi.mock('../../components/intelligence/Toast', () => ({ ToastContainer: () => null }));
vi.mock('../../components/intelligence/SyncAuditPanel', async () => {
  const React = await import('react');
  return {
    SyncAuditPanel: () => React.createElement('div', { 'data-testid': 'brain-sync-audit' }),
  };
});

const makeGraph = (n: number) => ({
  nodes: Array.from({ length: n }, (_, i) => ({ id: `n${i}`, kind: 'summary', label: `N${i}` })),
  edges: [],
  content_root_abs: '/tmp/content',
});

describe('Brain graph — transient failure recovery', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    coreAuthRef.current = 'user-A';
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('clears the error when a retry succeeds after a transient failure', async () => {
    // Fail once, then succeed — a transient blip, not a broken backend.
    graphExportMock
      .mockRejectedValueOnce(new Error('transient blip'))
      .mockResolvedValue(makeGraph(2));

    await act(async () => {
      renderWithProviders(<Brain />, { initialEntries: ['/?tab=graph'] });
    });

    // The failure is visible first — otherwise the clearing below proves nothing.
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
    expect(screen.queryByTestId('memory-graph')).not.toBeInTheDocument();

    // The in-product refetch trigger.
    await act(async () => {
      window.dispatchEvent(new Event('openhuman:memory-tree-completed'));
    });

    // What is actually provable here, stated honestly: the panel RECOVERS —
    // the retry runs and its data renders.
    //
    // What this cannot prove is that `error` state itself was cleared. The
    // render is `graph ? <graph> : error ? <alert> : null`, so once `graph` is
    // truthy a still-set `error` is simply not in the DOM. I verified that by
    // deleting `setError(null)` from `Brain.tsx:97` and re-running: both tests
    // in this file still passed. The alert assertion below therefore documents
    // the recovered UI, not the state reset — and the reason the state reset is
    // unobservable is itself BUG-W4-3, which the next test characterises.
    //
    // Revert-checked by disabling the `openhuman:memory-tree-completed`
    // listener (`Brain.tsx:113-117`): both tests then fail on the call-count
    // assertion below.
    await waitFor(() => {
      expect(screen.getByTestId('memory-graph')).toHaveTextContent('nodes:2');
    });
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(graphExportMock).toHaveBeenCalledTimes(2);
  });

  it('CHARACTERISES BUG-W4-3: a refresh that fails after a good load is silently swallowed', async () => {
    // Succeed first, then fail — the opposite order to the test above.
    graphExportMock
      .mockResolvedValueOnce(makeGraph(3))
      .mockRejectedValue(new Error('refresh blew up'));

    await act(async () => {
      renderWithProviders(<Brain />, { initialEntries: ['/?tab=graph'] });
    });
    await waitFor(() => {
      expect(screen.getByTestId('memory-graph')).toHaveTextContent('nodes:3');
    });

    await act(async () => {
      window.dispatchEvent(new Event('openhuman:memory-tree-completed'));
    });
    await waitFor(() => {
      expect(graphExportMock).toHaveBeenCalledTimes(2);
    });

    // What the user sees: the OLD graph, and no warning that the refresh died.
    //
    // This asserts today's behaviour deliberately. When BUG-W4-3 is fixed —
    // by surfacing the error alongside stale data, or by clearing `graph` on
    // error — this test goes red and must be rewritten to assert whichever
    // was chosen. That is the intent: right now the swallow is invisible.
    expect(screen.getByTestId('memory-graph')).toHaveTextContent('nodes:3');
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });
});
