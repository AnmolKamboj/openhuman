import { expect, type Page, test } from '@playwright/test';

import { bootAuthenticatedPage, dismissWalkthroughIfPresent, waitForAppReady } from '../helpers/core-rpc';

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
 * Seed the persisted notification slice for `USER`.
 *
 * redux-persist stores each whitelisted field as its own JSON string inside the
 * blob, which is why `items` is double-encoded here.
 */
async function seedFeed(page: Page, items: SeedItem[]): Promise<void> {
  await page.addInitScript(
    ({ user, payload }) => {
      window.localStorage.setItem('OPENHUMAN_ACTIVE_USER_ID', user);
      window.localStorage.setItem(
        `${user}:persist:notifications`,
        JSON.stringify({ items: JSON.stringify(payload), _persist: '{"version":-1,"rehydrated":true}' })
      );
    },
    { user: USER, payload: items }
  );
}

async function openFeed(page: Page): Promise<void> {
  await bootAuthenticatedPage(page, USER, '/notifications');
  await waitForAppReady(page);
  await dismissWalkthroughIfPresent(page);
  await expect(page.getByTestId('system-events-section')).toBeVisible({ timeout: 20_000 });
}

const feed = (page: Page) => page.getByTestId('system-events-section');
const rows = (page: Page) => feed(page).getByTestId('notification-item');

test.describe('Notifications — the system-events feed renders what was stored', () => {
  test('shows every seeded item', async ({ page }) => {
    await seedFeed(page, [
      item('n-agents-1', 'agents', 'Agent finished a task'),
      item('n-system-1', 'system', 'Core restarted'),
    ]);
    await openFeed(page);

    await expect(rows(page)).toHaveCount(2, { timeout: 20_000 });
    await expect(feed(page)).toContainText('Agent finished a task');
    await expect(feed(page)).toContainText('Core restarted');
  });

  test('offers a filter chip only for categories that are present', async ({ page }) => {
    // The chip row is built from the categories actually in the feed. Offering
    // a filter that can only ever show nothing is a dead control.
    await seedFeed(page, [item('n-agents-1', 'agents', 'Agent finished a task')]);
    await openFeed(page);

    await expect(page.getByTestId('notif-filter-chip-all')).toBeVisible({ timeout: 20_000 });
    await expect(page.getByTestId('notif-filter-chip-agents')).toBeVisible();
    await expect(page.getByTestId('notif-filter-chip-system')).toHaveCount(0);
  });
});

test.describe('Notifications — filtering is single-select and actually filters', () => {
  test('narrows the feed to one category and back', async ({ page }) => {
    await seedFeed(page, [
      item('n-agents-1', 'agents', 'Agent finished a task'),
      item('n-system-1', 'system', 'Core restarted'),
    ]);
    await openFeed(page);
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
    await seedFeed(page, [
      item('n-agents-1', 'agents', 'Agent finished a task'),
      item('n-system-1', 'system', 'Core restarted'),
    ]);
    await openFeed(page);

    const all = page.getByTestId('notif-filter-chip-all');
    const system = page.getByTestId('notif-filter-chip-system');

    await expect(all).toHaveAttribute('aria-selected', 'true', { timeout: 20_000 });

    await system.click();
    await expect(system).toHaveAttribute('aria-selected', 'true');
    await expect(all).toHaveAttribute('aria-selected', 'false');
  });
});

test.describe('Notifications — read state', () => {
  test('clicking an unread item marks it read and drops the unread count', async ({ page }) => {
    await seedFeed(page, [
      item('n-agents-1', 'agents', 'Agent finished a task'),
      item('n-system-1', 'system', 'Core restarted'),
    ]);
    await openFeed(page);
    await expect(rows(page)).toHaveCount(2, { timeout: 20_000 });

    // The header reports the unread count; it is the user-visible signal that
    // the click did anything.
    await expect(page.getByText(/2 unread/i)).toBeVisible();

    await rows(page).first().click();

    await expect(page.getByText(/1 unread/i)).toBeVisible({ timeout: 10_000 });
  });

  test('Mark all read clears the count and disables itself', async ({ page }) => {
    await seedFeed(page, [
      item('n-agents-1', 'agents', 'Agent finished a task'),
      item('n-system-1', 'system', 'Core restarted'),
    ]);
    await openFeed(page);

    const markAll = page.getByRole('button', { name: /mark all read/i });
    await expect(markAll).toBeEnabled({ timeout: 20_000 });

    await markAll.click();

    await expect(page.getByText(/unread/i)).toHaveCount(0, { timeout: 10_000 });
    await expect(markAll).toBeDisabled();
  });

  test('Mark all read is already disabled when nothing is unread', async ({ page }) => {
    await seedFeed(page, [item('n-agents-1', 'agents', 'Agent finished a task', true)]);
    await openFeed(page);

    await expect(rows(page)).toHaveCount(1, { timeout: 20_000 });
    await expect(page.getByRole('button', { name: /mark all read/i })).toBeDisabled();
  });

  test('read state survives a reload', async ({ page }) => {
    // The slice is persisted, so marking read must outlive the page. If it does
    // not, the feed re-accuses the user of everything they just dismissed.
    await seedFeed(page, [item('n-agents-1', 'agents', 'Agent finished a task')]);
    await openFeed(page);

    await page.getByRole('button', { name: /mark all read/i }).click();
    await expect(page.getByRole('button', { name: /mark all read/i })).toBeDisabled({
      timeout: 10_000,
    });

    await page.reload();
    await waitForAppReady(page);
    await dismissWalkthroughIfPresent(page);

    await expect(page.getByRole('button', { name: /mark all read/i })).toBeDisabled({
      timeout: 20_000,
    });
  });
});
