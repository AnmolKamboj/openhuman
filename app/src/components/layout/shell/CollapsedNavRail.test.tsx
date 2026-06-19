import { fireEvent, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { renderWithProviders } from '../../../test/test-utils';
import CollapsedNavRail from './CollapsedNavRail';

const mockNavigate = vi.fn();
const mockHome = vi.fn();

vi.mock('react-router-dom', async importOriginal => {
  const actual = await importOriginal<typeof import('react-router-dom')>();
  return { ...actual, useNavigate: () => mockNavigate };
});
vi.mock('./useHomeNav', () => ({ useHomeNav: () => mockHome }));
// Deterministic labels: render the i18n key so queries don't depend on locale.
vi.mock('../../../lib/i18n/I18nContext', () => ({ useT: () => ({ t: (k: string) => k }) }));
vi.mock('../../../services/analytics', () => ({ trackEvent: vi.fn() }));

describe('CollapsedNavRail', () => {
  beforeEach(() => vi.clearAllMocks());

  it('renders Home plus every primary nav destination as an icon button', () => {
    renderWithProviders(<CollapsedNavRail />, { initialEntries: ['/home'] });
    for (const key of [
      'nav.home',
      'nav.chat',
      'nav.human',
      'nav.brain',
      'nav.agentWorld',
      'nav.connections',
    ]) {
      expect(screen.getByRole('button', { name: key })).toBeInTheDocument();
    }
  });

  it('navigates to a destination path when its icon is clicked', () => {
    renderWithProviders(<CollapsedNavRail />, { initialEntries: ['/home'] });
    fireEvent.click(screen.getByRole('button', { name: 'nav.brain' }));
    expect(mockNavigate).toHaveBeenCalledWith('/brain');
  });

  it('runs the shared Home action when Home is clicked', () => {
    renderWithProviders(<CollapsedNavRail />, { initialEntries: ['/home'] });
    fireEvent.click(screen.getByRole('button', { name: 'nav.home' }));
    expect(mockHome).toHaveBeenCalledTimes(1);
    expect(mockNavigate).not.toHaveBeenCalled();
  });

  it('marks the active destination with aria-current', () => {
    renderWithProviders(<CollapsedNavRail />, { initialEntries: ['/agent-world'] });
    expect(screen.getByRole('button', { name: 'nav.agentWorld' })).toHaveAttribute(
      'aria-current',
      'page'
    );
    expect(screen.getByRole('button', { name: 'nav.chat' })).not.toHaveAttribute('aria-current');
  });

  it('treats /chat as the active Home state', () => {
    renderWithProviders(<CollapsedNavRail />, { initialEntries: ['/chat/abc'] });
    expect(screen.getByRole('button', { name: 'nav.home' })).toHaveAttribute(
      'aria-current',
      'page'
    );
  });
});
