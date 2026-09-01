import { render } from '@testing-library/react';
import type React from 'react';
import { MemoryRouter, useLocation } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';

vi.mock('./lib/platform', () => ({ getIsMobile: () => false }));

vi.mock('./components/PublicRoute', () => ({
  default: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));
vi.mock('./components/ProtectedRoute', () => ({
  default: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));
vi.mock('./components/DefaultRedirect', () => ({
  default: () => <div data-testid="default-redirect" />,
}));

vi.mock('./pages/WebCallbackPage', () => ({
  default: ({ callbackKind }: { callbackKind?: string }) => (
    <div data-testid="web-callback">{callbackKind ?? 'route-param'}</div>
  ),
}));

vi.mock('./AppRoutesIOS', () => ({ default: () => <div /> }));
vi.mock('./features/human/HumanPage', () => ({ default: () => <div /> }));
vi.mock('./pages/Accounts', () => ({ default: () => <div /> }));
vi.mock('./pages/Brain', () => ({ default: () => <div /> }));
vi.mock('./pages/dev/AgentInsightsPreview', () => ({ default: () => <div /> }));
vi.mock('./pages/dev/UiGallery', () => ({ default: () => <div /> }));
vi.mock('./pages/Invites', () => ({ default: () => <div /> }));
vi.mock('./pages/Notifications', () => ({ default: () => <div /> }));
vi.mock('./pages/onboarding/Onboarding', () => ({ default: () => <div /> }));
vi.mock('./pages/PttOverlayPage', () => ({ PttOverlayPage: () => <div /> }));
vi.mock('./pages/Rewards', () => ({ default: () => <div /> }));
vi.mock('./pages/Settings', () => ({ default: () => <div data-testid="settings-page" /> }));
vi.mock('./pages/Skills', () => ({ default: () => <div data-testid="skills-page" /> }));
vi.mock('./pages/Welcome', () => ({ default: () => <div /> }));
vi.mock('./pages/WorkflowsRun', () => ({ default: () => <div /> }));

// Reads the ambient router location and passes it to a callback. Rendered as a
// sibling of AppRoutes so it sees the settled location after any Navigate fires.
function LocationSpy({
  onCapture,
}: {
  onCapture: (loc: { pathname: string; search: string; hash: string }) => void;
}) {
  const { pathname, search, hash } = useLocation();
  onCapture({ pathname, search, hash });
  return null;
}

const AppRoutes = (await import('./AppRoutes')).default;

/**
 * `/webhooks` is a TWO-HOP redirect and the deep link has to survive both.
 *
 *   /webhooks -> /settings/integrations   (AppRoutes.tsx)
 *   /settings/integrations -> /connections (settingsRouteElements.tsx)
 *
 * The two hops are intended — the Integrations settings section was retired and
 * the OAuth grid moved to Connections. What was not intended is that both hops
 * used a bare `<Navigate>`, which discards `search` and `hash`, so
 * `/webhooks?tab=inbound#delivery-3` arrived at a bare `/connections`.
 *
 * Fixing only the first hop would not have been enough: the fragment would have
 * reached `/settings/integrations` and been dropped by the second. These tests
 * assert the end of the chain, so they fail if EITHER hop regresses.
 */
describe('/webhooks two-hop back-compat redirect', () => {
  it('lands on /connections', () => {
    const loc = { pathname: '', search: '', hash: '' };
    render(
      <MemoryRouter initialEntries={['/webhooks']}>
        <AppRoutes />
        <LocationSpy onCapture={l => Object.assign(loc, l)} />
      </MemoryRouter>
    );
    expect(loc.pathname).toBe('/connections');
  });

  it('carries a query string across both hops', () => {
    const loc = { pathname: '', search: '', hash: '' };
    render(
      <MemoryRouter initialEntries={['/webhooks?tab=inbound']}>
        <AppRoutes />
        <LocationSpy onCapture={l => Object.assign(loc, l)} />
      </MemoryRouter>
    );
    expect(loc.pathname).toBe('/connections');
    expect(loc.search).toBe('?tab=inbound');
  });

  it('carries a hash fragment across both hops', () => {
    // The reported defect (#5908). A bare `<Navigate>` on either hop loses this.
    const loc = { pathname: '', search: '', hash: '' };
    render(
      <MemoryRouter initialEntries={['/webhooks#delivery-3']}>
        <AppRoutes />
        <LocationSpy onCapture={l => Object.assign(loc, l)} />
      </MemoryRouter>
    );
    expect(loc.pathname).toBe('/connections');
    expect(loc.hash).toBe('#delivery-3');
  });

  it('carries a query string and a fragment together', () => {
    const loc = { pathname: '', search: '', hash: '' };
    render(
      <MemoryRouter initialEntries={['/webhooks?tab=inbound#delivery-3']}>
        <AppRoutes />
        <LocationSpy onCapture={l => Object.assign(loc, l)} />
      </MemoryRouter>
    );
    expect(loc.pathname).toBe('/connections');
    expect(loc.search).toBe('?tab=inbound');
    expect(loc.hash).toBe('#delivery-3');
  });

  it('produces an empty search and hash when /webhooks carries neither', () => {
    // Guards the other direction: forwarding must not invent a stray `?` or `#`.
    const loc = { pathname: '', search: '(init)', hash: '(init)' };
    render(
      <MemoryRouter initialEntries={['/webhooks']}>
        <AppRoutes />
        <LocationSpy onCapture={l => Object.assign(loc, l)} />
      </MemoryRouter>
    );
    expect(loc.pathname).toBe('/connections');
    expect(loc.search).toBe('');
    expect(loc.hash).toBe('');
  });

  it('forwards the deep link when /settings/integrations is entered directly', () => {
    // The second hop is reachable on its own, not only via /webhooks — anyone
    // holding an old Integrations settings link hits it.
    const loc = { pathname: '', search: '', hash: '' };
    render(
      <MemoryRouter initialEntries={['/settings/integrations?tab=inbound#delivery-3']}>
        <AppRoutes />
        <LocationSpy onCapture={l => Object.assign(loc, l)} />
      </MemoryRouter>
    );
    expect(loc.pathname).toBe('/connections');
    expect(loc.search).toBe('?tab=inbound');
    expect(loc.hash).toBe('#delivery-3');
  });
});
