/**
 * Route-table coverage for the connections / channels / flows / automation
 * surfaces.
 *
 * WHY THIS FILE EXISTS, given `pages/__tests__/Connections.redirects.test.tsx`
 * already claims to cover two of these redirects:
 *
 * That file declares its **own** local `<TestRoutes>` copy of three routes and
 * renders that, so it asserts React Router's `<Navigate>` works rather than
 * asserting anything about this app's route table. Deleting `/skills` from
 * `AppRoutes.tsx` leaves it green. It also never inspects the landing URL,
 * which is the entire payload of the `/channels` redirect.
 *
 * This file mounts the REAL `AppRoutes` (same mocking pattern as
 * `AppRoutes.auth.test.tsx`) and asserts the landing `pathname + search`, so a
 * change to the route table is what makes it fail.
 */
import { render, screen } from '@testing-library/react';
import type React from 'react';
import { MemoryRouter, useLocation, useParams } from 'react-router-dom';
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

vi.mock('./AppRoutesIOS', () => ({ default: () => <div /> }));
vi.mock('./features/human/HumanPage', () => ({ default: () => <div /> }));
vi.mock('./pages/Accounts', () => ({ default: () => <div /> }));
vi.mock('./pages/Brain', () => ({ default: () => <div /> }));
vi.mock('./pages/dev/AgentInsightsPreview', () => ({ default: () => <div /> }));
vi.mock('./pages/dev/assistant-ui-demo', () => ({ default: () => <div /> }));
vi.mock('./pages/dev/UiGallery', () => ({ default: () => <div /> }));
vi.mock('./pages/Invites', () => ({ default: () => <div /> }));
vi.mock('./pages/Notifications', () => ({ default: () => <div /> }));
vi.mock('./pages/onboarding/Onboarding', () => ({ default: () => <div /> }));
vi.mock('./pages/PttOverlayPage', () => ({ PttOverlayPage: () => <div /> }));
vi.mock('./pages/Rewards', () => ({ default: () => <div /> }));
vi.mock('./pages/Settings', () => ({ default: () => <div /> }));
vi.mock('./pages/Welcome', () => ({ default: () => <div /> }));
vi.mock('./pages/WebCallbackPage', () => ({ default: () => <div /> }));

// The pages this file actually asserts on. Each renders a probe that reports
// the landing location, so an assertion failure names the URL we landed on.
vi.mock('./pages/Skills', () => ({
  default: () => <div data-testid="page">connections</div>,
}));
vi.mock('./pages/FlowsPage', () => ({
  default: () => <div data-testid="page">flows</div>,
}));
vi.mock('./pages/Activity', () => ({
  default: () => <div data-testid="page">activity</div>,
}));
vi.mock('./pages/WorkflowsRun', () => ({
  default: () => <div data-testid="page">workflows-run</div>,
}));
vi.mock('./pages/FlowCanvasPage', () => ({
  default: () => {
    const { id } = useParams();
    return <div data-testid="page">{`flow-canvas:${id ?? ''}`}</div>;
  },
  FlowCanvasDraftPage: () => <div data-testid="page">flow-canvas-draft</div>,
}));

const AppRoutes = (await import('./AppRoutes')).default;

/** Reports the live location so assertions can name the URL we landed on. */
function LocationProbe() {
  const loc = useLocation();
  return <span data-testid="href">{loc.pathname + loc.search}</span>;
}

function renderAt(entry: string) {
  render(
    <MemoryRouter initialEntries={[entry]}>
      <LocationProbe />
      <AppRoutes />
    </MemoryRouter>
  );
  return {
    href: () => screen.getByTestId('href').textContent,
    page: () => screen.queryByTestId('page')?.textContent,
  };
}

describe('connections / channels back-compat redirects (real route table)', () => {
  it('/skills lands on /connections and renders the Connections page', () => {
    const at = renderAt('/skills');
    expect(at.href()).toBe('/connections');
    expect(at.page()).toBe('connections');
  });

  it('/channels lands on /connections?tab=messaging, preserving the tab selector', () => {
    // The whole point of this redirect: `/channels` was an orphaned standalone
    // page, and the messaging tab of Connections replaced it. Landing on bare
    // `/connections` would drop the user on the Welcome tab instead — which is
    // exactly what `Connections.redirects.test.tsx` cannot distinguish, because
    // it only asserts that the page rendered.
    const at = renderAt('/channels');
    expect(at.href()).toBe('/connections?tab=messaging');
    expect(at.page()).toBe('connections');
  });

  it('PINS A KNOWN BUG: /skills drops its ?tab= query on the way to /connections', () => {
    // `AppRoutes.tsx` claims twice — at the block comment above the
    // `/connections` route and again on the `/skills` line itself — that this
    // redirect "preserves ?tab= deep links". It does not: `<Navigate to="…" />`
    // is given an absolute path *string* with no search component, so the
    // incoming query is discarded.
    //
    // The knock-on is that `pages/Skills.tsx`'s legacy alias table
    // (`apps`→`composio`, `messaging`→`channels`, `tools`→`mcp`,
    // `explorer`→`skills`), whose own comment says it exists "so that e.g.
    // `/skills?tab=composio` still works after the redirect", is unreachable
    // from that route — `activeTab` always falls through to 'welcome'.
    //
    // This assertion pins the CURRENT (wrong) behaviour deliberately, so the
    // bug cannot deepen unnoticed. When it is fixed — `<Navigate to={{ pathname:
    // '/connections', search: location.search }} />` or equivalent — this test
    // MUST be flipped to expect '/connections?tab=messaging' and the two source
    // comments left alone, because they will finally be true.
    const at = renderAt('/skills?tab=messaging');
    expect(at.href()).toBe('/connections');
    expect(at.href()).not.toBe('/connections?tab=messaging');
  });
});

describe('automation route slugs', () => {
  it('/routines redirects to /flows', () => {
    const at = renderAt('/routines');
    expect(at.href()).toBe('/flows');
    expect(at.page()).toBe('flows');
  });

  it('/webhooks redirects to the Integrations settings page', () => {
    // Webhooks were retired from the UI; the route survives only to keep old
    // deep links from 404-ing.
    const at = renderAt('/webhooks');
    expect(at.href()).toBe('/settings/integrations');
  });

  it('/workflows is NOT a redirect — it renders the legacy SKILL.md hub', () => {
    // Guards against the stale claim in `AppRoutes.tsx`'s own `/flows` block
    // comment, which says "the bare `/workflows` and `/routines` slugs now
    // redirect here (to `/flows`)". Only `/routines` does. `/workflows` renders
    // Activity, per the comment directly above its own route.
    const at = renderAt('/workflows');
    expect(at.href()).toBe('/workflows');
    expect(at.page()).toBe('activity');
  });

  it('/workflows/run renders the single-purpose Skill runner, not the hub', () => {
    const at = renderAt('/workflows/run');
    expect(at.href()).toBe('/workflows/run');
    expect(at.page()).toBe('workflows-run');
  });
});

describe('/flows canvas route ranking', () => {
  it('/flows renders the flows list hub', () => {
    expect(renderAt('/flows').page()).toBe('flows');
  });

  it('/flows/draft resolves to the draft canvas, not to /flows/:id', () => {
    // `AppRoutes.tsx` warns that if `:id` won this match, the canvas would call
    // `flows_get('draft')` for a flow that does not exist. Pin the resolution
    // so a reorder or a rename of either route is caught here.
    const at = renderAt('/flows/draft');
    expect(at.page()).toBe('flow-canvas-draft');
    expect(at.page()).not.toBe('flow-canvas:draft');
  });

  it('/flows/:id resolves to the canvas and hands it the id', () => {
    expect(renderAt('/flows/flow_abc123').page()).toBe('flow-canvas:flow_abc123');
  });
});
