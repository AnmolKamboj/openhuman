import { fireEvent, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { renderWithProviders } from '../../../test/test-utils';
import { SidebarProvider } from '../../ui';
import AppSidebar from './AppSidebar';

/** `AppSidebar` reads `useSidebar()` — it must render inside a `<SidebarProvider>`. */
function renderAppSidebar(
  options?: Parameters<typeof renderWithProviders>[1],
  providerProps?: { open?: boolean }
) {
  return renderWithProviders(
    <SidebarProvider open={providerProps?.open ?? true}>
      <AppSidebar />
    </SidebarProvider>,
    options
  );
}

const mockNavigate = vi.fn();
const mockTrackEvent = vi.fn();
// Mutable so each test can pick the session kind. `isReady` sits alongside
// `snapshot` on the core-state value (not inside the snapshot). Must be
// `mock`-prefixed so the hoisted vi.mock factory below may close over it.
let mockCoreState: { snapshot: { sessionToken: string | null }; isReady: boolean } = {
  snapshot: { sessionToken: 'cloud.session.token' },
  isReady: true,
};

vi.mock('react-router-dom', async importOriginal => {
  const actual = await importOriginal<typeof import('react-router-dom')>();
  return { ...actual, useNavigate: () => mockNavigate };
});
// Render i18n keys verbatim so assertions don't depend on locale copy.
vi.mock('../../../lib/i18n/I18nContext', () => ({ useT: () => ({ t: (k: string) => k }) }));
vi.mock('../../../services/analytics', () => ({
  trackEvent: (...args: unknown[]) => mockTrackEvent(...args),
}));
vi.mock('../../../providers/CoreStateProvider', () => ({ useCoreState: () => mockCoreState }));
// Keep the mount light: the footer affordance rows are the unit under test, not
// the header/nav/rail children (SidebarHeader in particular needs the
// RootShellLayout context the test harness doesn't provide). SidebarSlot is left
// real on purpose — the harness itself imports SidebarSlotProvider from it.
vi.mock('./SidebarHeader', () => ({ default: () => null }));
vi.mock('./SidebarNav', () => ({ default: () => null }));
vi.mock('./SidebarAppRail', () => ({ default: () => null }));
vi.mock('../../ConnectionIndicator', () => ({ default: () => null }));

describe('AppSidebar — Rewards footer entry', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockCoreState = { snapshot: { sessionToken: 'cloud.session.token' }, isReady: true };
  });

  it('shows the Rewards row for a resolved cloud session and navigates + tracks on click', () => {
    renderAppSidebar({ initialEntries: ['/chat'] });

    const rewards = screen.getByTitle('nav.rewards');
    expect(rewards).toBeInTheDocument();

    fireEvent.click(rewards);

    expect(mockNavigate).toHaveBeenCalledWith('/rewards');
    expect(mockTrackEvent).toHaveBeenCalledWith(
      'tab_bar_change',
      expect.objectContaining({ to_tab: 'rewards', to_path: '/rewards' })
    );
  });

  it('normalizes from_path to a route template so entity IDs never reach analytics', () => {
    renderAppSidebar({ initialEntries: ['/chat/thread-abc123'] });

    fireEvent.click(screen.getByTitle('nav.rewards'));

    expect(mockTrackEvent).toHaveBeenCalledWith(
      'tab_bar_change',
      expect.objectContaining({ from_path: '/chat/:threadId', to_path: '/rewards' })
    );
  });

  it('hides the Rewards row for a local session but keeps Feedback', () => {
    mockCoreState = { snapshot: { sessionToken: 'header.payload.local' }, isReady: true };
    renderAppSidebar({ initialEntries: ['/chat'] });

    expect(screen.queryByTitle('nav.rewards')).not.toBeInTheDocument();
    expect(screen.getByTitle('nav.feedback')).toBeInTheDocument();
  });

  it('hides the Rewards row until core state has bootstrapped (no flash)', () => {
    // Initial snapshot before the first refresh: not ready, null token.
    // isLocalSessionToken(null) is false, so gating on the token alone would
    // briefly show Rewards here — the isReady guard prevents that flash.
    mockCoreState = { snapshot: { sessionToken: null }, isReady: false };
    renderAppSidebar({ initialEntries: ['/chat'] });

    expect(screen.queryByTitle('nav.rewards')).not.toBeInTheDocument();
    expect(screen.getByTitle('nav.feedback')).toBeInTheDocument();
  });

  it('marks the Rewards row active on the /rewards route', () => {
    renderAppSidebar({ initialEntries: ['/rewards'] });

    expect(screen.getByTitle('nav.rewards')).toHaveAttribute('aria-current', 'page');
  });
});

// The `Sidebar` column stays mounted while collapsed (`collapsible="icon"`),
// so `AppSidebar` — not `RootShellLayout` — is what switches to the compact
// rail body. These render inside a collapsed `SidebarProvider` directly,
// bypassing the mocked SidebarHeader/SidebarNav above so the real collapsed
// branch (drag strip, reopen trigger, CollapsedNavRail) is under test.
describe('AppSidebar — collapsed rail', () => {
  beforeEach(() => vi.clearAllMocks());

  it('renders the reopen trigger and collapsed nav rail instead of the header/nav', () => {
    renderAppSidebar({ initialEntries: ['/chat'] }, { open: false });

    expect(screen.getByTestId('root-shell-reopen')).toBeInTheDocument();
    // The primary nav destinations still resolve via CollapsedNavRail.
    expect(screen.getByRole('button', { name: 'nav.chat' })).toBeInTheDocument();
  });

  it('reserves a draggable strip above the reopen trigger for the macOS traffic lights', () => {
    const { container } = renderAppSidebar({ initialEntries: ['/chat'] }, { open: false });
    expect(container.querySelector('[data-tauri-drag-region]')).toBeInTheDocument();
  });

  it('gives the reopen trigger the expected analytics id and label', () => {
    renderAppSidebar({ initialEntries: ['/chat'] }, { open: false });
    const reopen = screen.getByTestId('root-shell-reopen');
    expect(reopen).toHaveAttribute('data-analytics-id', 'root-shell-reopen-sidebar');
    expect(reopen).toHaveAttribute('aria-label', 'layout.showSidebar');
  });

  it('does not render the reopen trigger while expanded', () => {
    renderAppSidebar({ initialEntries: ['/chat'] }, { open: true });
    expect(screen.queryByTestId('root-shell-reopen')).not.toBeInTheDocument();
  });
});
