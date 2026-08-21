import { act, screen, waitFor } from '@testing-library/react';
import { useLocation } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { renderWithProviders } from '../../test/test-utils';
import Brain from '../Brain';

/** Renders the live router pathname so redirect tests can assert the final route. */
function LocationProbe() {
  return <div data-testid="location-pathname">{useLocation().pathname}</div>;
}

const graphExportMock = vi.hoisted(() => vi.fn());
// Controllable authenticated identity so we can simulate a logout→login cycle
// (userId null → set) and assert the graph reloads (#4149).
const coreAuthRef = vi.hoisted(() => ({ current: 'user-A' as string | null }));
// Captures navigate() calls so we can assert the legacy TinyPlace-orchestration
// deep link bounces to the folded-in Orchestration sub-tab.
const navigateSpy = vi.hoisted(() => vi.fn());
// Fires on every OrchestrationView render, so a redirect test can prove the
// gated surface never mounted (not merely that it left the DOM afterwards).
const orchestrationRenderSpy = vi.hoisted(() => vi.fn());
// Controllable tiny.place identity so we can render Brain as a holder (default)
// or a confirmed non-holder hitting the gated `?tab=orchestration` deep link.
const tinyplaceIdentityRef = vi.hoisted(() => ({
  current: { status: 'ready', hasIdentity: true } as {
    status: 'loading' | 'ready';
    hasIdentity: boolean;
  },
}));

vi.mock('react-router-dom', async importOriginal => {
  const actual = await importOriginal<typeof import('react-router-dom')>();
  return { ...actual, useNavigate: () => navigateSpy };
});

vi.mock('../../hooks/useTinyPlaceIdentity', () => ({
  useTinyPlaceIdentity: () => tinyplaceIdentityRef.current,
}));

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

vi.mock('../../hooks/useSubconscious', () => ({
  useSubconscious: () => ({
    status: null,
    mode: 'off',
    refresh: vi.fn(),
    triggerTick: vi.fn(),
    setMode: vi.fn(),
  }),
}));

vi.mock('../../components/intelligence/IntelligenceSubconsciousTab', async () => {
  const React = await import('react');
  return { default: () => React.createElement('div', { 'data-testid': 'brain-subconscious' }) };
});
vi.mock('../../components/layout/ChipTabs', async () => {
  const React = await import('react');
  return {
    default: ({ children }: { children?: React.ReactNode }) =>
      React.createElement('div', null, children),
  };
});
vi.mock('../../components/ui/BetaBanner', () => ({ default: () => null }));
vi.mock('../../components/orchestration/OrchestrationView', async () => {
  const React = await import('react');
  return {
    default: () => {
      orchestrationRenderSpy();
      return React.createElement('div', { 'data-testid': 'brain-orchestration' });
    },
  };
});

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

describe('Brain page', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    coreAuthRef.current = 'user-A';
    tinyplaceIdentityRef.current = { status: 'ready', hasIdentity: true };
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the graph once data is fetched', async () => {
    graphExportMock.mockResolvedValue(makeGraph(3));
    await act(async () => {
      renderWithProviders(<Brain />, { initialEntries: ['/?tab=graph'] });
    });
    await waitFor(() => {
      expect(screen.getByTestId('memory-graph')).toHaveTextContent('nodes:3');
    });
  });

  it('renders empty-state graph when there are no nodes', async () => {
    graphExportMock.mockResolvedValue(makeGraph(0));
    await act(async () => {
      renderWithProviders(<Brain />, { initialEntries: ['/?tab=graph'] });
    });
    await waitFor(() => {
      expect(screen.getByTestId('memory-graph')).toHaveTextContent('nodes:0');
    });
  });

  it('reloads the memory graph from the store when the user re-authenticates (#4149)', async () => {
    // Start signed-out / mid identity-flip: the first fetch resolves empty.
    coreAuthRef.current = null;
    graphExportMock.mockResolvedValue(makeGraph(0));
    let view!: ReturnType<typeof renderWithProviders>;
    await act(async () => {
      view = renderWithProviders(<Brain />, { initialEntries: ['/?tab=graph'] });
    });
    await waitFor(() => expect(graphExportMock).toHaveBeenCalledTimes(1));
    expect(screen.getByTestId('memory-graph')).toHaveTextContent('nodes:0');

    // Re-login: identity becomes available — the graph must re-pull from the
    // persistent store rather than keep the signed-out empty state.
    coreAuthRef.current = 'user-A';
    graphExportMock.mockResolvedValue(makeGraph(5));
    await act(async () => {
      view.rerender(<Brain />);
    });
    await waitFor(() => expect(graphExportMock).toHaveBeenCalledTimes(2));
    await waitFor(() => {
      expect(screen.getByTestId('memory-graph')).toHaveTextContent('nodes:5');
    });
  });

  it('surfaces an error alert when the fetch fails', async () => {
    graphExportMock.mockRejectedValue(new Error('boom'));
    await act(async () => {
      renderWithProviders(<Brain />, { initialEntries: ['/?tab=graph'] });
    });
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
  });

  // All tabs share the standard scaffold. Drive each via the `?tab=` query
  // param so every per-tab branch is exercised.
  it.each([
    ['sources', 'brain-sources'],
    ['sync', 'brain-sync'],
    ['subconscious', 'brain-subconscious'],
  ])('renders the %s tab', async (tab, testId) => {
    graphExportMock.mockResolvedValue(makeGraph(0));
    await act(async () => {
      renderWithProviders(<Brain />, { initialEntries: [`/?tab=${tab}`] });
    });
    await waitFor(() => {
      expect(screen.getByTestId(testId)).toBeInTheDocument();
    });
  });

  it('shows the sync history panel on the Sync tab', async () => {
    graphExportMock.mockResolvedValue(makeGraph(0));
    await act(async () => {
      renderWithProviders(<Brain />, { initialEntries: ['/?tab=sync'] });
    });
    await waitFor(() => {
      expect(screen.getByTestId('brain-sync-history')).toBeInTheDocument();
      expect(screen.getByTestId('brain-sync-audit')).toBeInTheDocument();
    });
  });

  it('renders the folded-in Orchestration view on the orchestration tab', async () => {
    graphExportMock.mockResolvedValue(makeGraph(0));
    await act(async () => {
      renderWithProviders(<Brain />, { initialEntries: ['/?tab=orchestration'] });
    });
    await waitFor(() => {
      expect(screen.getByTestId('brain-orchestration')).toBeInTheDocument();
    });
  });

  it('does not fetch the memory graph on the orchestration tab', async () => {
    graphExportMock.mockResolvedValue(makeGraph(0));
    await act(async () => {
      renderWithProviders(<Brain />, { initialEntries: ['/?tab=orchestration'] });
    });
    await waitFor(() => {
      expect(screen.getByTestId('brain-orchestration')).toBeInTheDocument();
    });
    // Folding Orchestration under Brain must not trigger the unrelated
    // memoryTreeGraphExport RPC that the standalone page never issued.
    expect(graphExportMock).not.toHaveBeenCalled();
  });

  it('redirects a confirmed non-holder away from ?tab=orchestration without mounting OrchestrationView', async () => {
    // #5424 — the redirect is render-phase, so the gated tiny.place surface must
    // never mount (not even for a single commit) once the identity check
    // resolves to a non-holder.
    tinyplaceIdentityRef.current = { status: 'ready', hasIdentity: false };
    graphExportMock.mockResolvedValue(makeGraph(0));
    await act(async () => {
      renderWithProviders(
        <>
          <Brain />
          <LocationProbe />
        </>,
        { initialEntries: ['/?tab=orchestration'] }
      );
    });
    // Render-phase guard: OrchestrationView must never mount — asserting on the
    // spy (not just final DOM) catches a regression that mounts then redirects
    // from an effect.
    expect(orchestrationRenderSpy).not.toHaveBeenCalled();
    expect(screen.queryByTestId('brain-orchestration')).not.toBeInTheDocument();
    // Router landed on the Brain welcome tab.
    expect(screen.getByTestId('location-pathname')).toHaveTextContent('/brain');
    expect(screen.getByTestId('brain-welcome')).toBeInTheDocument();
  });

  it('keeps rendering OrchestrationView while the identity check is still loading', async () => {
    // The in-flight window must not redirect — only a *confirmed* non-holder does.
    tinyplaceIdentityRef.current = { status: 'loading', hasIdentity: false };
    graphExportMock.mockResolvedValue(makeGraph(0));
    await act(async () => {
      renderWithProviders(<Brain />, { initialEntries: ['/?tab=orchestration'] });
    });
    await waitFor(() => {
      expect(screen.getByTestId('brain-orchestration')).toBeInTheDocument();
    });
  });

  it('redirects the legacy tinyplace-orchestration deep link to the orchestration tab', async () => {
    graphExportMock.mockResolvedValue(makeGraph(0));
    await act(async () => {
      renderWithProviders(<Brain />, { initialEntries: ['/?tab=tinyplace-orchestration'] });
    });
    await waitFor(() => {
      expect(navigateSpy).toHaveBeenCalledWith('/brain?tab=orchestration', { replace: true });
    });
  });
});
