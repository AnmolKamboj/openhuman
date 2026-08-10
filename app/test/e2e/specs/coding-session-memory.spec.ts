import { waitForApp, waitForAuthBootstrap } from '../helpers/app-helpers';
import { triggerAuthDeepLinkBypass } from '../helpers/deep-link-helpers';
import { resetApp } from '../helpers/reset-app';
import { navigateViaHash } from '../helpers/shared-flows';
import { startMockServer, stopMockServer } from '../mock-server';

describe('Coding-agent session memory', () => {
  before(async () => {
    await startMockServer();
    await waitForApp();
    await resetApp('e2e-coding-session-memory');
    await triggerAuthDeepLinkBypass('e2e-coding-session-memory');
    await waitForAuthBootstrap();
    await navigateViaHash('/brain?tab=sources');
  });

  after(async () => {
    await stopMockServer();
  });

  it('surfaces Codex and Claude Code as private local memory sources', async () => {
    const card = await $('[data-testid="coding-sessions-card"]');
    await card.waitForDisplayed({ timeout: 20_000 });
    expect(await card.getText()).toContain('Coding-agent sessions');
    await expect($('[data-testid="coding-session-source-claude_code"]')).toBeDisplayed();
    await expect($('[data-testid="coding-session-source-codex"]')).toBeDisplayed();
    await expect($('[data-testid="coding-sessions-ingest"]')).toBeDisplayed();
  });
});
