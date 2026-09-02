import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import BillingPanel from './BillingPanel';

const navigateBack = vi.fn();

vi.mock('../hooks/useSettingsNavigation', () => ({
  useSettingsNavigation: () => ({
    navigateBack,
    navigateToSettings: vi.fn(),
    navigateToTeamManagement: vi.fn(),
    breadcrumbs: [],
  }),
}));

const openUrlMock = vi.fn();
vi.mock('../../../utils/openUrl', () => ({ openUrl: (url: string) => openUrlMock(url) }));

const getCurrentPlanMock = vi.fn();
const purchasePlanMock = vi.fn();
const createCoinbaseChargeMock = vi.fn();

vi.mock('../../../services/api/billingApi', () => ({
  billingApi: {
    getCurrentPlan: (...args: unknown[]) => getCurrentPlanMock(...args),
    purchasePlan: (...args: unknown[]) => purchasePlanMock(...args),
    createCoinbaseCharge: (...args: unknown[]) => createCoinbaseChargeMock(...args),
  },
}));

describe('<BillingPanel />', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    openUrlMock.mockResolvedValue(undefined);
    getCurrentPlanMock.mockResolvedValue({
      plan: 'FREE',
      hasActiveSubscription: false,
      planExpiry: null,
      subscription: null,
      monthlyBudgetUsd: 0,
      weeklyBudgetUsd: 0,
    });
    purchasePlanMock.mockResolvedValue({
      checkoutUrl: 'https://checkout.stripe.com/test',
      sessionId: 'test-session',
    });
    createCoinbaseChargeMock.mockResolvedValue({
      gatewayTransactionId: 'test-gw',
      hostedUrl: 'https://commerce.coinbase.com/test',
      status: 'NEW',
      expiresAt: '2026-01-01T00:00:00Z',
    });
  });

  it('renders the plan selector and the dashboard button without auto-opening the browser', async () => {
    render(<BillingPanel />);

    // SubscriptionPlans renders its own title; billing frequency selection is
    // back in-app so users can change their plan without leaving the desktop app.
    expect(screen.getByText('Choose a Plan')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Open billing dashboard' })).toBeInTheDocument();

    // getCurrentPlan is called on mount but must not trigger a browser open.
    await waitFor(() => expect(getCurrentPlanMock).toHaveBeenCalledTimes(1));
    expect(openUrlMock).not.toHaveBeenCalled();
  });

  it('loads the current plan tier on mount and passes it to SubscriptionPlans', async () => {
    getCurrentPlanMock.mockResolvedValue({
      plan: 'BASIC',
      hasActiveSubscription: true,
      planExpiry: null,
      subscription: null,
      monthlyBudgetUsd: 20,
      weeklyBudgetUsd: 10,
    });

    render(<BillingPanel />);

    await waitFor(() => expect(getCurrentPlanMock).toHaveBeenCalledTimes(1));
    // With BASIC as current tier the BASIC card shows the "Current plan" badge.
    expect(await screen.findByText('Current plan')).toBeInTheDocument();
  });

  it('upgrade with card payment calls purchasePlan and opens the checkout URL', async () => {
    render(<BillingPanel />);

    await waitFor(() => expect(getCurrentPlanMock).toHaveBeenCalledTimes(1));

    // Both BASIC and PRO show upgrade buttons when current tier is FREE.
    const upgradeButtons = screen.getAllByRole('button', { name: 'Upgrade' });
    fireEvent.click(upgradeButtons[0]);

    await waitFor(() => expect(purchasePlanMock).toHaveBeenCalledTimes(1));
    await waitFor(() =>
      expect(openUrlMock).toHaveBeenCalledWith('https://checkout.stripe.com/test')
    );
  });

  it('opens the billing dashboard when the user clicks the secondary button', async () => {
    render(<BillingPanel />);

    fireEvent.click(screen.getByRole('button', { name: 'Open billing dashboard' }));
    await waitFor(() => expect(openUrlMock).toHaveBeenCalledTimes(1));
    expect(openUrlMock).toHaveBeenLastCalledWith('https://tinyhumans.ai/dashboard');
  });

  it('invokes the navigation back handler from both the header and the inline button', async () => {
    render(<BillingPanel />);

    // The SettingsHeader back button (aria-label "Back") and the inline
    // "Back to settings" button both route through navigateBack.
    fireEvent.click(screen.getByRole('button', { name: 'Back' }));
    fireEvent.click(screen.getByRole('button', { name: 'Back to settings' }));
    expect(navigateBack).toHaveBeenCalledTimes(2);
  });
});
