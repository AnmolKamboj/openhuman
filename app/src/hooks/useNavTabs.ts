/**
 * The visible primary nav tabs (#5424).
 *
 * Identical to {@link NAV_TABS} except the `agent-world` (tiny.place) tab is
 * hidden from users without a tiny.place identity — the feature is being removed
 * after 31 August 2026 and its entry points must only appear for people who
 * already have one. Both nav renderers (expanded {@link SidebarNav} and the
 * collapsed rail) consume this so the rule lives in one place.
 */
import { useMemo } from 'react';

import { NAV_TABS, type NavTab } from '../config/navConfig';
import { useTinyPlaceIdentity } from './useTinyPlaceIdentity';

export function useNavTabs(): NavTab[] {
  const { hasIdentity } = useTinyPlaceIdentity();
  return useMemo(
    () => NAV_TABS.filter(tab => tab.id !== 'agent-world' || hasIdentity),
    [hasIdentity]
  );
}
