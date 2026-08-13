import { fireEvent, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { renderWithProviders } from '../../../test/test-utils';
import { AGENT_ACCOUNT_ID } from '../../../utils/accountsFullscreen';
import SidebarNav from './SidebarNav';

// Analytics is fire-and-forget; stub it so the nav renders without a transport.
vi.mock('../../../services/analytics', () => ({ trackEvent: vi.fn() }));

// The Tiny.Place (agent-world) tab is gated on a tiny.place identity (#5424).
// These tests exercise active-route matching with the full nav, so pin identity
// present; the gate itself is covered by useNavTabs.test.ts.
vi.mock('../../../hooks/useTinyPlaceIdentity', () => ({
  useTinyPlaceIdentity: () => ({ status: 'ready', hasIdentity: true }),
}));

/** The rendered button for a nav label (label text lives in a child span). */
function tabButton(label: string): HTMLButtonElement {
  return screen.getByRole('button', { name: new RegExp(label) }) as HTMLButtonElement;
}

describe('SidebarNav active matching', () => {
  it('keeps Tiny.Place active on its redirected /agent-world/explore route', () => {
    // The tab links to /agent-world but the index immediately redirects to
    // /agent-world/explore — an exact match would never light up.
    renderWithProviders(<SidebarNav />, { initialEntries: ['/agent-world/explore'] });

    expect(tabButton('Tiny.Place')).toHaveAttribute('aria-current', 'page');
  });

  it('keeps Tiny.Place active on a nested section route', () => {
    renderWithProviders(<SidebarNav />, { initialEntries: ['/agent-world/messaging'] });

    expect(tabButton('Tiny.Place')).toHaveAttribute('aria-current', 'page');
  });

  it('does not mark Tiny.Place active on an unrelated route', () => {
    renderWithProviders(<SidebarNav />, { initialEntries: ['/chat'] });

    expect(tabButton('Tiny.Place')).not.toHaveAttribute('aria-current');
    expect(tabButton('Chat')).toHaveAttribute('aria-current', 'page');
  });

  it('keeps Workflows active on the /flows list route', () => {
    renderWithProviders(<SidebarNav />, { initialEntries: ['/flows'] });

    expect(tabButton('Workflows')).toHaveAttribute('aria-current', 'page');
  });

  it('keeps Workflows active on a nested /flows/* sub-route', () => {
    renderWithProviders(<SidebarNav />, { initialEntries: ['/flows/some-flow-id'] });

    expect(tabButton('Workflows')).toHaveAttribute('aria-current', 'page');
  });

  it('does not mark Workflows active on an unrelated route', () => {
    renderWithProviders(<SidebarNav />, { initialEntries: ['/chat'] });

    expect(tabButton('Workflows')).not.toHaveAttribute('aria-current');
  });

  it('gives the active tab a neutral fill that lifts off the chrome, not an accent tint', () => {
    renderWithProviders(<SidebarNav />, { initialEntries: ['/chat'] });

    const active = tabButton('Chat');
    // The sidebar sits on the themed chrome layer, which already carries the
    // theme's hue — so selection is a neutral surface lift plus weight. Tinting
    // the pill on top of a tinted chrome stacks two colours and reads as noise.
    expect(active.className).toContain('bg-surface/70');
    expect(active.className).toContain('font-semibold');
    expect(active.className).not.toContain('bg-primary');
    expect(active.className).not.toContain('bg-white');

    // Inactive tabs carry no active fill.
    expect(tabButton('Human').className).not.toContain('bg-surface/70');
  });

  it('clears an active provider selection when clicking the already-active nav item', () => {
    const { store } = renderWithProviders(<SidebarNav />, {
      initialEntries: ['/connections'],
      preloadedState: {
        accounts: {
          accounts: {
            'acct-slack': {
              id: 'acct-slack',
              provider: 'slack',
              label: 'Slack',
              createdAt: '2026-01-01T00:00:00.000Z',
              status: 'open',
            },
          },
          order: ['acct-slack'],
          activeAccountId: 'acct-slack',
          lastActiveAccountId: 'acct-slack',
          messages: {},
          unread: {},
          logs: {},
          overlayOpen: false,
        },
      },
    });

    fireEvent.click(tabButton('Connections'));

    expect(store.getState().accounts.activeAccountId).toBe(AGENT_ACCOUNT_ID);
  });
});
