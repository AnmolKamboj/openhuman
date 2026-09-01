/**
 * ⚠️ SKIPPED — DOES NOT PASS YET. Do not un-skip without re-running it.
 *
 * Committed for the diagnosis, not as coverage. Three runs against the live
 * lane (ports 18406/17706/4406, clean core) all fail at the same point.
 *
 * What was established and is worth keeping:
 *  - The persist blob format here is CORRECT: redux-persist stores each
 *    whitelisted field as its own JSON string inside the outer object, and a
 *    probe confirmed the app's own blob has exactly this shape.
 *  - The namespace is NOT the bypass id passed to `bootAuthenticatedPage`.
 *    The app resolves its active user from the mock backend and writes
 *    `user-123:persist:notifications`; a probe of `localStorage` showed both
 *    `pw-notif-feed:persist:notifications` (my seed, never read) and
 *    `user-123:persist:notifications` (`items: "[]"`, what the app reads).
 *    `seedFeed` below therefore discovers the id at runtime rather than
 *    assuming it.
 *
 * Where it still stops: after seeding and re-navigating to /#/notifications,
 * `system-events-section` does not become visible within 20s. Not yet diagnosed
 * — the remaining candidates are that redux-persist rehydrates once per page
 * context and ignores a post-boot localStorage write, or that the section only
 * mounts when the feed is non-empty at first paint. If the former, seeding has
 * to happen in `addInitScript` under the real user id, which means learning the
 * id in a throwaway page context first.
 *
 * The other half of this surface — integration notifications, which ARE
 * RPC-driven — is already covered by `notifications.spec.ts`; this file
 * deliberately does not duplicate it.
 */
import { expect, type Page, test } from '@playwright/test';

import {
  bootAuthenticatedPage,
  dismissWalkthroughIfPresent,
  waitForAppReady,
} from '../helpers/core-rpc';

/**
 * The system-events notification feed: filtering, mark-as-read, clear.
 *
 * `notifications.spec.ts` already covers the *integration* half — the core RPCs
 * (`notification_ingest` / `_list` / `_mark_read` / `_stats`) and that the page
 * renders both sections. It never touches the system-events feed's controls,
 * which are a different data path entirely: those items arrive over socket.io
 * from the Rust core and live in a redux-persist slice, so no core RPC can put
 * one on screen.
 *
 * Seeding therefore goes through the app's own persistence, not a mock: the
 * slice is persisted under `${activeUserId}:persist:notifications`
 * (`store/index.ts:144-149`, `store/userScopedStorage.ts:177-180`), and
 * `OPENHUMAN_ACTIVE_USER_ID` selects the namespace. Writing those two keys
 * before navigation is exactly what a returning user's browser already holds —
 * the real reducer rehydrates them, the real page renders them, and every
 * assertion below is on what the user can see and click.
 *
 * What this pins is the part that is invisible to a component test: that the
 * chips are a single-select tablist over the categories actually present, that
 * clicking an item marks it read, and that "Mark all read" empties the unread
 * count rather than only the badge.
 */

const USER = 'pw-notif-feed';

interface SeedItem {
  id: string;
  category: string;
  title: string;
  body: string;
  timestamp: number;
  read: boolean;
}

function item(id: string, category: string, title: string, read = false): SeedItem {
  return { id, category, title, body: `${title} body`, timestamp: Date.now(), read };
}

/**
 * Seed the persisted notification slice for whichever user the app is actually
 * scoped to, then reload so the real reducer rehydrates it.
 *
 * The namespace is discovered at runtime rather than assumed. `activeUserId`
 * comes from the signed-in identity the app resolves — in this lane the mock
 * backend answers `user-123`, NOT the bypass id passed to
 * `bootAuthenticatedPage`. Seeding under the bypass id writes a key nothing ever
 * reads, and every assertion then times out against an empty feed. That is
 * exactly what the first run of this spec did.
 *
 * The blob shape is redux-persist's: each whitelisted field is its own JSON
 * string inside the outer object (`store/index.ts:144-149`).
 */
async function seedFeed(page: Page, items: SeedItem[]): Promise<void> {
  const user = await page.evaluate(() => localStorage.getItem('OPENHUMAN_ACTIVE_USER_ID'));
  if (!user) throw new Error('no active user id — the app has not resolved a session yet');

  await page.evaluate(
    ({ key, payload }) => {
      const raw = window.localStorage.getItem(key);
      const blob: Record<string, string> = raw ? JSON.parse(raw) : {};
      blob.items = JSON.stringify(payload);
      window.localStorage.setItem(key, JSON.stringify(blob));
    },
    { key: `${user}:persist:notifications`, payload: items }
  );

  // Navigate rather than reload: a bare reload can land on the default route,
  // and the seeded feed is only observable on /notifications.
  await page.goto('/#/notifications');
  await waitForAppReady(page);
  await dismissWalkthroughIfPresent(page);
}

async function openFeedWith(page: Page, items: SeedItem[]): Promise<void> {
  await bootAuthenticatedPage(page, USER, '/notifications');
  await waitForAppReady(page);
  await dismissWalkthroughIfPresent(page);
  await seedFeed(page, items);
  await expect(page.getByTestId('system-events-section')).toBeVisible({ timeout: 20_000 });
}

const feed = (page: Page) => page.getByTestId('system-events-section');
const rows = (page: Page) => feed(page).getByTestId('notification-item');

test.describe.skip('Notifications — the system-events feed renders what was stored', () => {
  test('shows every seeded item', async ({ page }) => {
    await openFeedWith(page, [
      item('n-agents-1', 'agents', 'Agent finished a task'),
      item('n-system-1', 'system', 'Core restarted'),
    ]);

    await expect(rows(page)).toHaveCount(2, { timeout: 20_000 });
    await expect(feed(page)).toContainText('Agent finished a task');
    await expect(feed(page)).toContainText('Core restarted');
  });

  test('offers a filter chip only for categories that are present', async ({ page }) => {
    // The chip row is built from the categories actually in the feed. Offering
    // a filter that can only ever show nothing is a dead control.
    await openFeedWith(page, [item('n-agents-1', 'agents', 'Agent finished a task')]);

    await expect(page.getByTestId('notif-filter-chip-all')).toBeVisible({ timeout: 20_000 });
    await expect(page.getByTestId('notif-filter-chip-agents')).toBeVisible();
    await expect(page.getByTestId('notif-filter-chip-system')).toHaveCount(0);
  });
});

test.describe.skip('Notifications — filtering is single-select and actually filters', () => {
  test('narrows the feed to one category and back', async ({ page }) => {
    await openFeedWith(page, [
      item('n-agents-1', 'agents', 'Agent finished a task'),
      item('n-system-1', 'system', 'Core restarted'),
    ]);
    await expect(rows(page)).toHaveCount(2, { timeout: 20_000 });

    await page.getByTestId('notif-filter-chip-system').click();

    await expect(rows(page)).toHaveCount(1);
    await expect(feed(page)).toContainText('Core restarted');
    await expect(feed(page)).not.toContainText('Agent finished a task');

    await page.getByTestId('notif-filter-chip-all').click();
    await expect(rows(page)).toHaveCount(2);
  });

  test('marks the active chip selected and deselects the previous one', async ({ page }) => {
    // A tablist, not a set of toggles: exactly one selected at a time. Two
    // chips reading `aria-selected="true"` is the bug this pins.
    await openFeedWith(page, [
      item('n-agents-1', 'agents', 'Agent finished a task'),
      item('n-system-1', 'system', 'Core restarted'),
    ]);

    const all = page.getByTestId('notif-filter-chip-all');
    const system = page.getByTestId('notif-filter-chip-system');

    await expect(all).toHaveAttribute('aria-selected', 'true', { timeout: 20_000 });

    await system.click();
    await expect(system).toHaveAttribute('aria-selected', 'true');
    await expect(all).toHaveAttribute('aria-selected', 'false');
  });
});

test.describe.skip('Notifications — read state', () => {
  test('clicking an unread item marks it read and drops the unread count', async ({ page }) => {
    await openFeedWith(page, [
      item('n-agents-1', 'agents', 'Agent finished a task'),
      item('n-system-1', 'system', 'Core restarted'),
    ]);
    await expect(rows(page)).toHaveCount(2, { timeout: 20_000 });

    // The header reports the unread count; it is the user-visible signal that
    // the click did anything.
    await expect(page.getByText(/2 unread/i)).toBeVisible();

    await rows(page).first().click();

    await expect(page.getByText(/1 unread/i)).toBeVisible({ timeout: 10_000 });
  });

  test('Mark all read clears the count and disables itself', async ({ page }) => {
    await openFeedWith(page, [
      item('n-agents-1', 'agents', 'Agent finished a task'),
      item('n-system-1', 'system', 'Core restarted'),
    ]);

    const markAll = page.getByRole('button', { name: /mark all read/i });
    await expect(markAll).toBeEnabled({ timeout: 20_000 });

    await markAll.click();

    await expect(page.getByText(/unread/i)).toHaveCount(0, { timeout: 10_000 });
    await expect(markAll).toBeDisabled();
  });

  test('Mark all read is already disabled when nothing is unread', async ({ page }) => {
    await openFeedWith(page, [item('n-agents-1', 'agents', 'Agent finished a task', true)]);

    await expect(rows(page)).toHaveCount(1, { timeout: 20_000 });
    await expect(page.getByRole('button', { name: /mark all read/i })).toBeDisabled();
  });

  test('read state survives a reload', async ({ page }) => {
    // The slice is persisted, so marking read must outlive the page. If it does
    // not, the feed re-accuses the user of everything they just dismissed.
    await openFeedWith(page, [item('n-agents-1', 'agents', 'Agent finished a task')]);

    await page.getByRole('button', { name: /mark all read/i }).click();
    await expect(page.getByRole('button', { name: /mark all read/i })).toBeDisabled({
      timeout: 10_000,
    });

    // `toBeDisabled()` above only proves in-memory state. redux-persist queues
    // its writes through an async `userScopedStorage.setItem`, so reloading
    // immediately can race the write and re-read the pre-mark blob — which
    // would look like "read state does not survive a reload" when in fact the
    // reload simply happened first. Wait for the persisted payload to show the
    // item as read before reloading.
    const user = await page.evaluate(() => localStorage.getItem('OPENHUMAN_ACTIVE_USER_ID'));
    await expect
      .poll(
        async () =>
          page.evaluate(key => {
            const raw = window.localStorage.getItem(key);
            if (!raw) return false;
            try {
              const blob = JSON.parse(raw) as { items?: string };
              const items = JSON.parse(blob.items ?? '[]') as Array<{ read?: boolean }>;
              return items.length > 0 && items.every(item => item.read === true);
            } catch {
              return false;
            }
          }, `${user}:persist:notifications`),
        { timeout: 10_000, message: 'the read state was never persisted' }
      )
      .toBe(true);

    await page.reload();
    await waitForAppReady(page);
    await dismissWalkthroughIfPresent(page);

    await expect(page.getByRole('button', { name: /mark all read/i })).toBeDisabled({
      timeout: 20_000,
    });
  });
});
