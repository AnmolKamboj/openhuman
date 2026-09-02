import { expect, test } from '@playwright/test';

import {
  bootAuthenticatedPage,
  dismissWalkthroughIfPresent,
  waitForAppReady,
} from '../helpers/core-rpc';

/**
 * A 401 from the LOCAL core's RPC bearer gate must not sign the user out
 * (PR #5876).
 *
 * `getCoreRpcToken` caches the resolved bearer for the lifetime of the frontend
 * process, so an in-process core restart — which mints a fresh per-launch token
 * — leaves the renderer holding a stale one and every subsequent RPC 401s.
 * Before #5876 `classifyRpcError` mapped any 401 to `auth_expired`, and
 * `classifyAuthExpiredReason` paired it with `confirmed`, which skips
 * corroboration in `CoreStateProvider` and calls `clearSession()` — wiping the
 * TinyHumans auth profile from disk because the *local* core rejected a bearer.
 * The TinyHumans server had said nothing at all.
 *
 * #5876 introduced a distinct `core_auth` kind for exactly this case: it does
 * not dispatch auth-expired, and it drops the token cache and retries once with
 * a freshly-read bearer.
 *
 * Note on what this does NOT cover, deliberately: a 401 from the *backend*
 * (a genuinely revoked session) must still sign the user out. That is the
 * complementary case and `app/test/e2e/specs/auth-access-control.spec.ts`
 * ("Revoked session auto-logout") owns it. The two must not be conflated — the
 * whole point of #5876 is that they are different routes with opposite
 * handling.
 */

/** The RPC the embeddings tab issues on open — a deterministic trigger. */
const TRIGGER_METHOD = 'openhuman.embeddings_get_settings';

test.describe('Core RPC bearer 401 — recovery, not logout', () => {
  test('retries once with a fresh bearer and keeps the user signed in', async ({ page }) => {
    await bootAuthenticatedPage(page, 'pw-core-401', '/connections');
    await waitForAppReady(page);
    await dismissWalkthroughIfPresent(page);

    // Fault injection: reject the bearer for the FIRST occurrence of the
    // trigger method only, exactly as a core that restarted mid-session would,
    // then serve normally so the refreshed-bearer retry can succeed.
    let seen = 0;
    await page.route('**/rpc', async (route, request) => {
      let body: { method?: string } = {};
      try {
        body = JSON.parse(request.postData() || '{}');
      } catch {
        /* not JSON — pass through */
      }
      if (body.method === TRIGGER_METHOD) {
        seen += 1;
        if (seen === 1) {
          return route.fulfill({
            status: 401,
            contentType: 'text/plain',
            body: 'Missing or invalid Authorization header',
          });
        }
      }
      return route.fallback();
    });

    // Trigger it.
    await page.evaluate(() => {
      window.location.hash = '/connections?tab=embeddings';
    });

    // (1) The retry. `core_auth` drops the token cache and reissues the SAME
    // call once. Without #5876 the 401 classifies as `auth_expired`, nothing
    // retries, and this stays at 1.
    await expect.poll(() => seen, { timeout: 20_000 }).toBeGreaterThanOrEqual(2);

    // Bounded to a single extra attempt — a retry loop against a core that is
    // genuinely rejecting us would be worse than the bug.
    expect(seen).toBeLessThanOrEqual(2);

    // (2) The session survives. Without #5876 `clearSession()` wipes the auth
    // profile and the app falls back to the signed-out surface.
    await expect
      .poll(async () => page.evaluate(() => window.location.hash), { timeout: 15_000 })
      .toContain('/connections');
    await expect
      .poll(async () => page.evaluate(() => window.location.hash))
      .not.toContain('/welcome');
    await expect
      .poll(async () =>
        page.evaluate(() => window.localStorage.getItem('openhuman_core_rpc_token'))
      )
      .not.toBeNull();
  });
});
