/**
 * Tests for navConfig — verifies the shape, count, and key values of NAV_TABS
 * and AVATAR_MENU_ITEMS so regressions are caught early.
 *
 * Phase 6 update: Human tab removed; Chat tab renamed to "Assistant"
 * (id stays 'chat', labelKey 'nav.assistant', walkthroughAttr 'tab-chat').
 * Nav drops from 6 tabs to 5.
 */
import { describe, expect, it } from 'vitest';

import { AVATAR_MENU_ITEMS, BRAIN_TAB, NAV_TABS } from '../navConfig';

describe('NAV_TABS', () => {
  it('has exactly 5 entries (Phase 6: Human merged into Assistant)', () => {
    expect(NAV_TABS).toHaveLength(5);
  });

  it('has the correct ids in order', () => {
    expect(NAV_TABS.map(t => t.id)).toEqual([
      'home',
      'chat',
      'connections',
      'activity',
      'settings',
    ]);
  });

  it('has the correct paths', () => {
    expect(NAV_TABS.map(t => t.path)).toEqual([
      '/home',
      '/chat',
      '/connections',
      '/activity',
      '/settings',
    ]);
  });

  it('has the correct labelKeys', () => {
    expect(NAV_TABS.map(t => t.labelKey)).toEqual([
      'nav.home',
      'nav.assistant',
      'nav.connections',
      'nav.activity',
      'nav.settings',
    ]);
  });

  it('has the correct walkthroughAttrs', () => {
    expect(NAV_TABS.map(t => t.walkthroughAttr)).toEqual([
      'tab-home',
      'tab-chat',
      'tab-connections',
      'tab-activity',
      'tab-settings',
    ]);
  });

  it('does not contain a Human tab (Phase 6: merged into Assistant)', () => {
    expect(NAV_TABS.find(t => t.id === 'human')).toBeUndefined();
  });

  it('does not contain a rewards tab', () => {
    expect(NAV_TABS.find(t => t.id === 'rewards')).toBeUndefined();
  });

  it('does not contain an intelligence or skills tab id', () => {
    expect(NAV_TABS.find(t => t.id === 'intelligence')).toBeUndefined();
    expect(NAV_TABS.find(t => t.id === 'skills')).toBeUndefined();
  });

  it('does not contain the Brain tab (rendered specially, not in the regular row)', () => {
    expect(NAV_TABS.find(t => t.id === 'brain')).toBeUndefined();
  });

  it('Assistant tab uses nav.assistant label key and tab-chat walkthrough attr', () => {
    const assistantTab = NAV_TABS.find(t => t.id === 'chat');
    expect(assistantTab).toBeDefined();
    expect(assistantTab?.labelKey).toBe('nav.assistant');
    expect(assistantTab?.walkthroughAttr).toBe('tab-chat');
    expect(assistantTab?.path).toBe('/chat');
  });
});

describe('BRAIN_TAB', () => {
  it('has the expected shape (id, path, labelKey, walkthroughAttr)', () => {
    expect(BRAIN_TAB).toEqual({
      id: 'brain',
      labelKey: 'nav.brain',
      path: '/brain',
      walkthroughAttr: 'tab-brain',
    });
  });
});

describe('AVATAR_MENU_ITEMS', () => {
  it('has exactly 5 entries', () => {
    expect(AVATAR_MENU_ITEMS).toHaveLength(5);
  });

  it('has the correct ids in order', () => {
    expect(AVATAR_MENU_ITEMS.map(i => i.id)).toEqual([
      'account',
      'billing',
      'rewards',
      'invites',
      'wallet',
    ]);
  });

  it('has the correct labelKeys', () => {
    expect(AVATAR_MENU_ITEMS.map(i => i.labelKey)).toEqual([
      'nav.avatarMenu.account',
      'nav.avatarMenu.billing',
      'nav.avatarMenu.rewards',
      'nav.avatarMenu.invites',
      'nav.avatarMenu.wallet',
    ]);
  });

  it('billing, rewards, and invites are cloudOnly; account and wallet are not', () => {
    const cloudOnly = AVATAR_MENU_ITEMS.filter(i => i.cloudOnly).map(i => i.id);
    expect(cloudOnly).toEqual(['billing', 'rewards', 'invites']);

    const notCloudOnly = AVATAR_MENU_ITEMS.filter(i => !i.cloudOnly).map(i => i.id);
    expect(notCloudOnly).toEqual(['account', 'wallet']);
  });

  it('billing uses openUrl; all others use navigate', () => {
    const openUrlItems = AVATAR_MENU_ITEMS.filter(i => i.kind === 'openUrl').map(i => i.id);
    expect(openUrlItems).toEqual(['billing']);

    const navigateItems = AVATAR_MENU_ITEMS.filter(i => i.kind === 'navigate').map(i => i.id);
    expect(navigateItems).toEqual(['account', 'rewards', 'invites', 'wallet']);
  });

  it('each item has a non-empty target', () => {
    for (const item of AVATAR_MENU_ITEMS) {
      expect(item.target.length).toBeGreaterThan(0);
    }
  });
});
