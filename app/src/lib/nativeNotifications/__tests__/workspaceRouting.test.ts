/**
 * Core notifications are routed by workspace on the client (issue #5966).
 *
 * The core already refuses to broadcast a workspace-bound notification from a
 * workspace the user has switched away from. But it decides that by resolving
 * the active workspace and *then* sending — two steps, so a switch in between
 * still lets one through, and `core_notification` reaches every connected
 * client with no per-client routing behind it. The payload therefore carries
 * the workspace's opaque handle, and this module re-checks it at the moment
 * it renders.
 *
 * The failure this prevents is not cosmetic: the banner prints an MCP server's
 * qualified name and its transport error, so a leaked one shows another
 * account's server and its failure text inside the account the user is in.
 *
 * The two fail-open paths below are the ones most likely to be "tightened"
 * into a bug later, which is why each is pinned with its reason.
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { store } from '../../../store';
import { setPreference } from '../../../store/notificationSlice';
import {
  __handleCoreNotificationForTests,
  __resetForTests,
  __setActiveWorkspaceForTests,
} from '../service';

vi.mock('../tauriBridge', () => ({ showNativeNotification: vi.fn() }));

vi.mock('../../../services/socketService', () => ({
  socketService: { on: vi.fn(), off: vi.fn() },
}));

const WS_A = 'ws_1111111111111111';
const WS_B = 'ws_2222222222222222';

function notify(overrides: { id: string; workspace?: string | null }) {
  __handleCoreNotificationForTests({
    id: overrides.id,
    category: 'system',
    title: 'MCP server parked',
    body: 'ac.inference.sh/mcp is not being retried.',
    timestamp_ms: 1,
    workspace: overrides.workspace,
  });
}

const items = () => store.getState().notifications.items;

describe('core notification workspace routing', () => {
  beforeEach(() => {
    __resetForTests();
    vi.clearAllMocks();
    store.dispatch({ type: 'notifications/clearAll' });
    store.dispatch(setPreference({ category: 'system', enabled: true }));
  });

  it('accepts a notification from the workspace the client is in', () => {
    __setActiveWorkspaceForTests(WS_A);
    notify({ id: 'mcp:mine', workspace: WS_A });
    expect(items()).toHaveLength(1);
    expect(items()[0].id).toBe('mcp:mine');
  });

  it('drops a notification from a workspace the user has switched away from', () => {
    __setActiveWorkspaceForTests(WS_A);
    notify({ id: 'mcp:theirs', workspace: WS_B });
    expect(items()).toHaveLength(0);
  });

  /**
   * Most notifications — cron, webhook, sub-agent, a rejected API key — are
   * not bound to a workspace and apply wherever they land. Treating a missing
   * handle as a mismatch would silence all of them.
   */
  it('accepts a notification that names no workspace', () => {
    __setActiveWorkspaceForTests(WS_A);
    notify({ id: 'cron:daily', workspace: null });
    expect(items()).toHaveLength(1);
  });

  /**
   * Rows persisted before the field existed deserialize without it, so this is
   * the same case as above reached by a different route: an upgrade must not
   * hide a user's stored notification history.
   */
  it('accepts a notification from before the field existed', () => {
    __setActiveWorkspaceForTests(WS_A);
    __handleCoreNotificationForTests({
      id: 'legacy:1',
      category: 'system',
      title: 'Webhook error',
      body: 'skill-x webhook returned HTTP 500',
      timestamp_ms: 1,
    });
    expect(items()).toHaveLength(1);
  });

  /**
   * Fails open, deliberately, and opposite to the core's fail-closed publish
   * gate. Here the notification has already been sent past that gate; dropping
   * it because the seed has not arrived yet would swallow every notification
   * for the rest of the session, with nothing on screen to say why.
   */
  it('accepts a workspace-bound notification while the active workspace is unknown', () => {
    __setActiveWorkspaceForTests(null);
    notify({ id: 'mcp:unknown-active', workspace: WS_B });
    expect(items()).toHaveLength(1);
  });

  it('follows a workspace switch', () => {
    __setActiveWorkspaceForTests(WS_A);
    notify({ id: 'mcp:a', workspace: WS_A });
    expect(items()).toHaveLength(1);

    __setActiveWorkspaceForTests(WS_B);
    notify({ id: 'mcp:a-again', workspace: WS_A });
    expect(items()).toHaveLength(1);

    notify({ id: 'mcp:b', workspace: WS_B });
    expect(items()).toHaveLength(2);
    expect(items().some(item => item.id === 'mcp:b')).toBe(true);
  });
});
