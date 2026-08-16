import { screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { renderWithProviders } from '../../../test/test-utils';
import RootShellLayout from './RootShellLayout';

// Render i18n keys verbatim so assertions don't depend on locale copy.
vi.mock('../../../lib/i18n/I18nContext', () => ({ useT: () => ({ t: (k: string) => k }) }));
// The collapsed rail pulls in routing + nav config; the shell's own geometry is
// the unit under test.
vi.mock('./CollapsedNavRail', () => ({ default: () => null }));
// macOS/Tauri-gated, and covered by its own spec.
vi.mock('./WindowDragBar', () => ({ default: () => null }));

function renderShell(props: { unframed?: boolean } = {}) {
  return renderWithProviders(
    <RootShellLayout sidebar={<nav>sidebar body</nav>} {...props}>
      <main>routed page</main>
    </RootShellLayout>
  );
}

describe('RootShellLayout', () => {
  it('renders the sidebar and the routed content', () => {
    renderShell();
    expect(screen.getByText('sidebar body')).toBeTruthy();
    expect(screen.getByText('routed page')).toBeTruthy();
  });

  it('mounts the routed content inside the content surface', () => {
    renderShell();
    const surface = screen.getByTestId('app-content-surface');
    expect(surface.contains(screen.getByText('routed page'))).toBe(true);
  });

  it('frames the content surface as a card by default', () => {
    renderShell();
    expect(screen.getByTestId('app-content-surface').dataset.unframed).toBeUndefined();
  });

  it('forwards unframed so a live CEF webview gets a square, edge-to-edge pane', () => {
    renderShell({ unframed: true });
    expect(screen.getByTestId('app-content-surface').dataset.unframed).toBe('true');
  });

  it('leaves the resize divider unfilled so the chrome reads as one surface', () => {
    renderShell();
    const divider = screen.getByTestId('root-shell-divider');
    expect(divider.className).toContain('bg-transparent');
    expect(divider.className).not.toContain('bg-surface-strong');
  });

  it('exposes the divider as a keyboard-operable separator', () => {
    renderShell();
    const divider = screen.getByTestId('root-shell-divider');
    expect(divider.getAttribute('role')).toBe('separator');
    expect(divider.getAttribute('aria-orientation')).toBe('vertical');
    expect(divider.tabIndex).toBe(0);
  });
});
