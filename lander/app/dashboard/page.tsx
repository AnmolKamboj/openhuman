'use client';

import { useState } from 'react';
import Navigation from '../components/Navigation';
import { useCurrentSubscriptionPlan } from '../hooks/useCurrentSubscriptionPlan';
import { useStripePlans } from '../hooks/useStripePlans';
import { useStripeCheckout } from '../hooks/useStripeCheckout';
import { useCoinbaseCheckout } from '../hooks/useCoinbaseCheckout';

// Mock data - in production, this would come from an API
const mockUsage = {
  dailyRecapsUsed: 8,
  dailyRecapsLimit: 15,
  aiTokensUsed: 125000,
  aiTokensLimit: 500000,
  connectionsUsed: 2,
  connectionsLimit: 3,
};

export default function Dashboard() {
  const { plan, loading, error, refetch } = useCurrentSubscriptionPlan();
  const { plans: stripePlans, loading: plansLoading } = useStripePlans();
  const { handleCheckout: handleStripeCheckout, loading: stripeLoading } = useStripeCheckout();
  const { handleCheckout: handleCoinbaseCheckout, loading: coinbaseLoading } = useCoinbaseCheckout();
  const [showCancelModal, setShowCancelModal] = useState(false);
  const [showConvertModal, setShowConvertModal] = useState(false);
  const [showSwitchModal, setShowSwitchModal] = useState(false);
  const [selectedPlan, setSelectedPlan] = useState<string | null>(null);
  const [billingCycle, setBillingCycle] = useState<'monthly' | 'annual'>('monthly');

  const recapsPercentage = (mockUsage.dailyRecapsUsed / mockUsage.dailyRecapsLimit) * 100;
  const tokensPercentage = (mockUsage.aiTokensUsed / mockUsage.aiTokensLimit) * 100;
  const connectionsPercentage = (mockUsage.connectionsUsed / mockUsage.connectionsLimit) * 100;

  // Format plan name from API format (FREE, BASIC, PRO) to display format (Free, Basic, Pro)
  const formatPlanName = (planType: string | null | undefined): string => {
    if (!planType) return 'Free';
    return planType.charAt(0) + planType.slice(1).toLowerCase();
  };

  const currentPlanName = formatPlanName(plan?.plan);
  const isUpgrade = (planName: string) => {
    const planOrder = ['Free', 'Basic', 'Pro'];
    const currentIndex = planOrder.indexOf(currentPlanName);
    const newIndex = planOrder.indexOf(planName);
    return newIndex > currentIndex;
  };

  // Format date for display
  const formatDate = (dateString: string | null | undefined): string => {
    if (!dateString) return 'N/A';
    try {
      const date = new Date(dateString);
      return date.toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'long',
        day: 'numeric',
      });
    } catch {
      return dateString;
    }
  };

  // Get next billing date from subscription
  const getNextBillingDate = (): string => {
    if (plan?.subscription?.currentPeriodEnd) {
      return formatDate(plan.subscription.currentPeriodEnd);
    }
    if (plan?.planExpiry) {
      return formatDate(plan.planExpiry);
    }
    return 'N/A';
  };

  const handleSwitchPlan = (planName: string) => {
    setSelectedPlan(planName);
    setShowSwitchModal(true);
  };

  const handleStripePayment = async () => {
    if (!selectedPlan || selectedPlan === 'Free') return;
    
    const planType = selectedPlan.toLowerCase() as 'basic' | 'pro';
    try {
      await handleStripeCheckout(planType, billingCycle);
      setShowSwitchModal(false);
      setSelectedPlan(null);
    } catch {
      // Error is already handled in the hook
    }
  };

  const handleCoinbasePayment = async () => {
    if (!selectedPlan || selectedPlan === 'Free') return;
    
    const planType = selectedPlan.toLowerCase() as 'basic' | 'pro';
    try {
      await handleCoinbaseCheckout(planType);
      setShowSwitchModal(false);
      setSelectedPlan(null);
    } catch {
      // Error is already handled in the hook
    }
  };

  // Get plan data from Stripe plans
  const getPlanData = (planName: string) => {
    return stripePlans.find((p) => p.name === planName);
  };

  return (
    <div className="min-h-screen bg-zinc-950 text-white">
      <Navigation />
      <main className="mx-auto max-w-7xl px-6 pt-24 sm:px-8 sm:pt-32">
        <div className="mx-auto max-w-5xl">
          <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">
            Dashboard
          </h1>
          <p className="mt-2 text-zinc-400">Manage your subscription and usage</p>

          {/* Subscription Card */}
          <div className="mt-8 rounded-lg border border-zinc-800 bg-zinc-900/50 p-6">
            {loading ? (
              <div className="flex items-center justify-center py-8">
                <p className="text-zinc-400">Loading subscription information...</p>
              </div>
            ) : error ? (
              <div className="flex flex-col items-center justify-center py-8">
                <p className="text-red-400">{error}</p>
                <button
                  onClick={() => refetch()}
                  className="mt-4 rounded-lg border border-zinc-800 px-4 py-2 text-sm font-semibold text-white transition-colors hover:border-zinc-700"
                >
                  Retry
                </button>
              </div>
            ) : plan ? (
              <div className="flex items-center justify-between">
                <div>
                  <h2 className="text-xl font-semibold text-white">
                    Current Plan: {currentPlanName}
                  </h2>
                  <p className="mt-1 text-sm text-zinc-400">
                    Status: {plan.hasActiveSubscription ? (
                      <span className="text-green-400">Active</span>
                    ) : (
                      <span className="text-zinc-500">Inactive</span>
                    )}
                  </p>
                  {plan.planExpiry && (
                    <p className="mt-1 text-sm text-zinc-400">
                      Plan expires: {formatDate(plan.planExpiry)}
                    </p>
                  )}
                  {plan.subscription?.currentPeriodEnd && (
                    <p className="mt-1 text-sm text-zinc-400">
                      Next billing date: {formatDate(plan.subscription.currentPeriodEnd)}
                    </p>
                  )}
                  {plan.subscription?.status && (
                    <p className="mt-1 text-sm text-zinc-400">
                      Subscription status: <span className="capitalize">{plan.subscription.status}</span>
                    </p>
                  )}
                </div>
                <div className="flex gap-3">
                  {plan.hasActiveSubscription && (
                    <>
                      <button
                        onClick={() => setShowConvertModal(true)}
                        className="rounded-lg border border-zinc-800 px-4 py-2 text-sm font-semibold text-white transition-colors hover:border-zinc-700"
                      >
                        Convert to Annual
                      </button>
                      <button
                        onClick={() => setShowCancelModal(true)}
                        className="rounded-lg border border-red-800 px-4 py-2 text-sm font-semibold text-red-400 transition-colors hover:border-red-700"
                      >
                        Cancel Subscription
                      </button>
                    </>
                  )}
                </div>
              </div>
            ) : (
              <div className="flex items-center justify-center py-8">
                <p className="text-zinc-400">No subscription information available</p>
              </div>
            )}
          </div>

          {/* Switch Plan Section */}
          <div className="mt-8">
            <div className="flex items-center justify-between">
              <div>
                <h2 className="text-xl font-semibold text-white">Switch Plan</h2>
                <p className="mt-1 text-sm text-zinc-400">
                  Upgrade or downgrade your subscription plan
                </p>
              </div>
              {/* Billing Cycle Toggle */}
              <div className="inline-flex rounded-lg border border-zinc-800 bg-zinc-900/50 p-1">
                <button
                  onClick={() => setBillingCycle('monthly')}
                  className={`rounded-md px-4 py-2 text-sm font-semibold transition-colors ${
                    billingCycle === 'monthly'
                      ? 'bg-white text-zinc-950'
                      : 'text-zinc-400 hover:text-white'
                  }`}
                >
                  Monthly
                </button>
                <button
                  onClick={() => setBillingCycle('annual')}
                  className={`rounded-md px-4 py-2 text-sm font-semibold transition-colors ${
                    billingCycle === 'annual'
                      ? 'bg-white text-zinc-950'
                      : 'text-zinc-400 hover:text-white'
                  }`}
                >
                  Annual
                  {billingCycle === 'annual' && (
                    <span className="ml-2 rounded-full bg-green-600 px-2 py-0.5 text-xs text-white">
                      Save 20%
                    </span>
                  )}
                </button>
              </div>
            </div>
            
            <div className="mt-4 grid gap-4 sm:grid-cols-3">
              {plansLoading ? (
                <div className="col-span-3 text-center text-zinc-400">Loading plans...</div>
              ) : (
                stripePlans.map((planData) => {
                  const isCurrentPlan = planData.name === currentPlanName;
                  const isUpgradePlan = isUpgrade(planData.name);
                  const price = billingCycle === 'monthly' ? planData.monthlyPrice : planData.annualPrice;
                  const monthlyEquivalent = billingCycle === 'annual' && price > 0 
                    ? Math.round(price / 12) 
                    : price;
                  const savings = billingCycle === 'annual' && planData.monthlyPrice > 0
                    ? (planData.monthlyPrice * 12) - price
                    : 0;

                  return (
                    <div
                      key={planData.name}
                      className={`relative rounded-lg border p-6 ${
                        isCurrentPlan
                          ? 'border-white bg-zinc-900'
                          : 'border-zinc-800 bg-zinc-900/50'
                      }`}
                    >
                      {isCurrentPlan && (
                        <div className="absolute -top-3 left-4">
                          <span className="rounded-full bg-white px-2 py-0.5 text-xs font-semibold text-zinc-950">
                            Current
                          </span>
                        </div>
                      )}
                      <div className="text-center">
                        <h3 className="text-lg font-semibold text-white">{planData.name}</h3>
                        <div className="mt-2 flex items-baseline justify-center gap-1">
                          <span className="text-3xl font-bold text-white">
                            ${price}
                          </span>
                          {price > 0 && (
                            <span className="text-sm text-zinc-400">
                              /{billingCycle === 'monthly' ? 'month' : 'year'}
                            </span>
                          )}
                        </div>
                        {billingCycle === 'annual' && price > 0 && (
                          <div className="mt-2">
                            <p className="text-xs text-zinc-400">
                              ${monthlyEquivalent}/month
                            </p>
                            {savings > 0 && (
                              <p className="mt-1 text-xs font-semibold text-green-400">
                                Save ${savings}/year
                              </p>
                            )}
                          </div>
                        )}
                      </div>
                      <button
                        onClick={() => handleSwitchPlan(planData.name)}
                        disabled={isCurrentPlan}
                        className={`mt-6 w-full rounded-lg px-4 py-2 text-sm font-semibold transition-colors ${
                          isCurrentPlan
                            ? 'cursor-not-allowed border border-zinc-700 bg-zinc-800 text-zinc-500'
                            : isUpgradePlan
                            ? 'bg-white text-zinc-950 hover:bg-zinc-200'
                            : 'border border-zinc-800 text-white hover:border-zinc-700'
                        }`}
                      >
                        {isCurrentPlan
                          ? 'Current Plan'
                          : isUpgradePlan
                          ? 'Upgrade'
                          : 'Downgrade'}
                      </button>
                    </div>
                  );
                })
              )}
            </div>
          </div>

          {/* Usage Stats */}
          <div className="mt-8">
            <h2 className="text-xl font-semibold text-white">Usage Statistics</h2>
            <div className="mt-4 grid gap-6 sm:grid-cols-3">
              <div className="rounded-lg border border-zinc-800 bg-zinc-900/50 p-6">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-sm text-zinc-400">Daily Recaps Used</p>
                    <p className="mt-1 text-2xl font-semibold text-white">
                      {mockUsage.dailyRecapsUsed}
                    </p>
                    <p className="mt-1 text-xs text-zinc-500">
                      of {mockUsage.dailyRecapsLimit}
                    </p>
                  </div>
                </div>
                <div className="mt-4 h-2 w-full overflow-hidden rounded-full bg-zinc-800">
                  <div
                    className="h-full bg-white transition-all"
                    style={{ width: `${recapsPercentage}%` }}
                  />
                </div>
              </div>

              <div className="rounded-lg border border-zinc-800 bg-zinc-900/50 p-6">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-sm text-zinc-400">AI Tokens Used</p>
                    <p className="mt-1 text-2xl font-semibold text-white">
                      {mockUsage.aiTokensUsed.toLocaleString()}
                    </p>
                    <p className="mt-1 text-xs text-zinc-500">
                      of {mockUsage.aiTokensLimit.toLocaleString()}
                    </p>
                  </div>
                </div>
                <div className="mt-4 h-2 w-full overflow-hidden rounded-full bg-zinc-800">
                  <div
                    className="h-full bg-white transition-all"
                    style={{ width: `${tokensPercentage}%` }}
                  />
                </div>
              </div>

              <div className="rounded-lg border border-zinc-800 bg-zinc-900/50 p-6">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-sm text-zinc-400">Connections Used</p>
                    <p className="mt-1 text-2xl font-semibold text-white">
                      {mockUsage.connectionsUsed}
                    </p>
                    <p className="mt-1 text-xs text-zinc-500">
                      of {mockUsage.connectionsLimit}
                    </p>
                  </div>
                </div>
                <div className="mt-4 h-2 w-full overflow-hidden rounded-full bg-zinc-800">
                  <div
                    className="h-full bg-white transition-all"
                    style={{ width: `${connectionsPercentage}%` }}
                  />
                </div>
              </div>
            </div>
          </div>
        </div>
      </main>

      {/* Cancel Modal */}
      {showCancelModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
          <div className="relative w-full max-w-md rounded-lg border border-zinc-800 bg-zinc-900 p-6">
            <button
              onClick={() => setShowCancelModal(false)}
              className="absolute right-4 top-4 text-zinc-400 hover:text-white transition-colors"
              aria-label="Close"
            >
              <svg
                className="h-6 w-6"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </button>
            <h3 className="text-xl font-semibold text-white pr-8">
              Cancel Subscription
            </h3>
            <p className="mt-2 text-sm text-zinc-400">
              Are you sure you want to cancel your subscription? You&apos;ll lose
              access to all premium features at the end of your billing period.
            </p>
            <div className="mt-6 flex gap-3">
              <button
                onClick={() => setShowCancelModal(false)}
                className="flex-1 rounded-lg border border-zinc-800 px-4 py-2 text-sm font-semibold text-white transition-colors hover:border-zinc-700"
              >
                Keep Subscription
              </button>
              <button
                onClick={() => {
                  // Handle cancellation
                  setShowCancelModal(false);
                  alert('Subscription cancelled');
                }}
                className="flex-1 rounded-lg bg-red-600 px-4 py-2 text-sm font-semibold text-white transition-colors hover:bg-red-700"
              >
                Cancel Subscription
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Convert to Annual Modal */}
      {showConvertModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
          <div className="relative w-full max-w-md rounded-lg border border-zinc-800 bg-zinc-900 p-6">
            <button
              onClick={() => setShowConvertModal(false)}
              className="absolute right-4 top-4 text-zinc-400 hover:text-white transition-colors"
              aria-label="Close"
            >
              <svg
                className="h-6 w-6"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </button>
            <h3 className="text-xl font-semibold text-white pr-8">
              Convert to Annual Plan
            </h3>
            <p className="mt-2 text-sm text-zinc-400">
              Save 20% by switching to an annual plan. Your subscription will
              be billed annually at $278.40 (equivalent to $23.20/month).
            </p>
            <div className="mt-6 flex gap-3">
              <button
                onClick={() => setShowConvertModal(false)}
                className="flex-1 rounded-lg border border-zinc-800 px-4 py-2 text-sm font-semibold text-white transition-colors hover:border-zinc-700"
              >
                Cancel
              </button>
              <button
                onClick={() => {
                  // Handle conversion
                  setShowConvertModal(false);
                  alert('Converted to annual plan');
                }}
                className="flex-1 rounded-lg bg-white px-4 py-2 text-sm font-semibold text-zinc-950 transition-colors hover:bg-zinc-200"
              >
                Convert Now
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Switch Plan Modal */}
      {showSwitchModal && selectedPlan && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
          <div className="relative w-full max-w-md rounded-lg border border-zinc-800 bg-zinc-900 p-6">
            {/* Close Button */}
            <button
              onClick={() => {
                setShowSwitchModal(false);
                setSelectedPlan(null);
              }}
              className="absolute right-4 top-4 text-zinc-400 hover:text-white transition-colors"
              aria-label="Close"
            >
              <svg
                className="h-6 w-6"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </button>

            <h3 className="text-xl font-semibold text-white pr-8">
              Switch to {selectedPlan} Plan
            </h3>
            <p className="mt-2 text-sm text-zinc-400">
              {isUpgrade(selectedPlan) ? (
                <>
                  You&apos;re upgrading from <strong>{currentPlanName}</strong> to{' '}
                  <strong>{selectedPlan}</strong>. Changes will take effect
                  immediately. You&apos;ll be charged a prorated amount for the
                  remainder of your billing cycle.
                </>
              ) : (
                <>
                  You&apos;re downgrading from <strong>{currentPlanName}</strong> to{' '}
                  <strong>{selectedPlan}</strong>. Changes will take effect at
                  the end of your current billing period on{' '}
                  {getNextBillingDate()}.
                </>
              )}
            </p>
            
            {selectedPlan !== 'Free' && (
              <>
                {/* Price Display */}
                <div className="mt-4 rounded-lg border border-zinc-800 bg-zinc-950/50 p-4">
                  {(() => {
                    const planData = getPlanData(selectedPlan);
                    if (!planData) return null;
                    
                    const price = billingCycle === 'monthly' 
                      ? planData.monthlyPrice 
                      : planData.annualPrice;
                    const monthlyEquivalent = billingCycle === 'annual' && price > 0
                      ? Math.round(price / 12)
                      : price;
                    
                    return (
                      <>
                        <p className="text-sm text-zinc-300">
                          {billingCycle === 'monthly' ? 'Monthly' : 'Annual'} charge: ${price}
                          {billingCycle === 'annual' && price > 0 && (
                            <span className="ml-2 text-zinc-400">
                              (${monthlyEquivalent}/month)
                            </span>
                          )}
                        </p>
                      </>
                    );
                  })()}
                </div>

                {/* Payment Buttons */}
                <div className="mt-6 space-y-3">
                  <button
                    onClick={handleStripePayment}
                    disabled={stripeLoading || coinbaseLoading}
                    className="w-full rounded-lg bg-white px-4 py-3 text-sm font-semibold text-zinc-950 transition-colors hover:bg-zinc-200 disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    {stripeLoading ? 'Processing...' : 'Pay with Card'}
                  </button>
                  <button
                    onClick={handleCoinbasePayment}
                    disabled={stripeLoading || coinbaseLoading}
                    className="w-full rounded-lg border border-zinc-800 px-4 py-3 text-sm font-semibold text-white transition-colors hover:border-zinc-700 disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    {coinbaseLoading ? 'Processing...' : 'Pay with Crypto'}
                  </button>
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
