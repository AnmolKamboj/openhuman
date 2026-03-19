'use client';

import { useState, Suspense } from 'react';
import Navigation from '../components/Navigation';
import PlanCard from '../components/PlanCard';
import { useAuthToken } from '../hooks/useAuthToken';
import { useStripePlans } from '../hooks/useStripePlans';
import { useStripeCheckout } from '../hooks/useStripeCheckout';
import { useCoinbaseCheckout } from '../hooks/useCoinbaseCheckout';

function PricingContent() {
    const [billingCycle, setBillingCycle] = useState<'monthly' | 'annual'>('monthly');
    const [loadingPlan, setLoadingPlan] = useState<string | null>(null);
    const token = useAuthToken();
    const { plans, loading: plansLoading, error: plansError, refetch: refetchPlans } = useStripePlans();
    const { handleCheckout: handleStripeCheckout, loading: stripeLoading } = useStripeCheckout();
    const { handleCheckout: handleCoinbaseCheckout, loading: coinbaseLoading } = useCoinbaseCheckout();

    const onStripeCheckout = async (planType: 'basic' | 'pro') => {
        if (!token) {
            alert('Missing token in URL. Please access this page via a valid link.');
            return;
        }

        setLoadingPlan(planType);
        try {
            await handleStripeCheckout(planType, billingCycle);
        } finally {
            setLoadingPlan(null);
        }
    };

    const onCoinbaseCheckout = async (planType: 'basic' | 'pro') => {
        if (!token) {
            alert('Missing token in URL. Please access this page via a valid link.');
            return;
        }

        setLoadingPlan(`coinbase-${planType}`);
        try {
            await handleCoinbaseCheckout(planType);
        } finally {
            setLoadingPlan(null);
        }
    };

    return (
        <div className="min-h-screen bg-zinc-950 text-white">
            <Navigation />
            <main className="mx-auto max-w-7xl px-6 pt-24 sm:px-8 sm:pt-32">
                <div className="mx-auto max-w-3xl text-center">
                    <h1 className="text-4xl font-bold tracking-tight sm:text-5xl">
                        Simple, Transparent Pricing
                    </h1>
                    <p className="mt-4 text-lg text-zinc-400">
                        Choose the plan that works best for you. Upgrade or downgrade at
                        any time.
                    </p>
                </div>

                {/* Billing Cycle Tabs */}
                <div className="mx-auto mt-8 flex justify-center">
                    <div className="inline-flex rounded-lg border border-zinc-800 bg-zinc-900/50 p-1">
                        <button
                            onClick={() => setBillingCycle('monthly')}
                            className={`rounded-md px-4 py-2 text-sm font-semibold transition-colors ${billingCycle === 'monthly'
                                ? 'bg-white text-zinc-950'
                                : 'text-zinc-400 hover:text-white'
                                }`}
                        >
                            Monthly
                        </button>
                        <button
                            onClick={() => setBillingCycle('annual')}
                            className={`rounded-md px-4 py-2 text-sm font-semibold transition-colors ${billingCycle === 'annual'
                                ? 'bg-white text-zinc-950'
                                : 'text-zinc-400 hover:text-white'
                                }`}
                        >
                            Annual
                            <span className="ml-2 rounded-full bg-green-600 px-2 py-0.5 text-xs text-white">
                                Save 20%
                            </span>
                        </button>
                    </div>
                </div>

                {plansError && (
                    <div className="mx-auto mt-16 max-w-2xl">
                        <div className="rounded-lg border border-red-500/20 bg-red-500/10 p-8 text-center">
                            <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-red-500/20">
                                <svg
                                    className="h-6 w-6 text-red-400"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    stroke="currentColor"
                                    strokeWidth={2}
                                >
                                    <path
                                        strokeLinecap="round"
                                        strokeLinejoin="round"
                                        d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
                                    />
                                </svg>
                            </div>
                            <h3 className="mb-2 text-lg font-semibold text-white">
                                Unable to Load Pricing Plans
                            </h3>
                            <p className="mb-6 text-sm text-zinc-400">
                                {plansError}
                            </p>
                            <button
                                onClick={() => refetchPlans()}
                                disabled={plansLoading}
                                className="inline-flex items-center gap-2 rounded-md bg-white px-4 py-2 text-sm font-semibold text-zinc-950 transition-colors hover:bg-zinc-100 disabled:cursor-not-allowed disabled:opacity-50"
                            >
                                {plansLoading ? (
                                    <>
                                        <svg
                                            className="h-4 w-4 animate-spin"
                                            xmlns="http://www.w3.org/2000/svg"
                                            fill="none"
                                            viewBox="0 0 24 24"
                                        >
                                            <circle
                                                className="opacity-25"
                                                cx="12"
                                                cy="12"
                                                r="10"
                                                stroke="currentColor"
                                                strokeWidth="4"
                                            ></circle>
                                            <path
                                                className="opacity-75"
                                                fill="currentColor"
                                                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                                            ></path>
                                        </svg>
                                        Retrying...
                                    </>
                                ) : (
                                    <>
                                        <svg
                                            className="h-4 w-4"
                                            fill="none"
                                            viewBox="0 0 24 24"
                                            stroke="currentColor"
                                            strokeWidth={2}
                                        >
                                            <path
                                                strokeLinecap="round"
                                                strokeLinejoin="round"
                                                d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                                            />
                                        </svg>
                                        Try Again
                                    </>
                                )}
                            </button>
                        </div>
                    </div>
                )}

                {plansLoading ? (
                    <div className="mx-auto mt-16 max-w-5xl">
                        <div className="grid gap-8 sm:grid-cols-2 lg:grid-cols-3">
                            {[1, 2, 3].map((i) => (
                                <div
                                    key={i}
                                    className="flex h-96 animate-pulse flex-col rounded-lg border border-zinc-800 bg-zinc-900/50 p-8"
                                >
                                    <div className="h-8 w-24 rounded bg-zinc-700"></div>
                                    <div className="mt-4 h-12 w-32 rounded bg-zinc-700"></div>
                                    <div className="mt-8 space-y-4">
                                        {[1, 2, 3, 4].map((j) => (
                                            <div
                                                key={j}
                                                className="h-4 w-full rounded bg-zinc-700"
                                            ></div>
                                        ))}
                                    </div>
                                </div>
                            ))}
                        </div>
                    </div>
                ) : (
                    <div className="mx-auto mt-16 grid max-w-5xl gap-8 sm:grid-cols-2 lg:grid-cols-3">
                        {plans.map((plan) => {
                            const isPaidPlan = plan.type === 'basic' || plan.type === 'pro';
                            const isStripeLoading = loadingPlan === plan.type && stripeLoading;
                            const isCoinbaseLoading = loadingPlan === `coinbase-${plan.type}` && coinbaseLoading;

                            return (
                                <PlanCard
                                    key={plan.name}
                                    name={plan.name}
                                    monthlyPrice={plan.monthlyPrice}
                                    annualPrice={plan.annualPrice}
                                    description={plan.description}
                                    features={plan.features}
                                    cta={isPaidPlan ? plan.cta : plan.cta}
                                    popular={plan.popular}
                                    billingCycle={billingCycle}
                                    disabled={isPaidPlan && !token}
                                    loading={isStripeLoading || isCoinbaseLoading}
                                    onPrimaryAction={
                                        isPaidPlan
                                            ? () => onStripeCheckout(plan.type as 'basic' | 'pro')
                                            : () => {
                                                window.location.href = '/downloads';
                                            }
                                    }
                                    onSecondaryAction={
                                        isPaidPlan
                                            ? () => onCoinbaseCheckout(plan.type as 'basic' | 'pro')
                                            : undefined
                                    }
                                    secondaryCta={isPaidPlan ? 'Pay with Crypto' : undefined}
                                />
                            );
                        })}
                    </div>
                )}
            </main>
        </div>
    );
}

export default function Pricing() {
    return (
        <Suspense fallback={
            <div className="min-h-screen bg-zinc-950 text-white">
                <Navigation />
                <main className="mx-auto max-w-7xl px-6 pt-24 sm:px-8 sm:pt-32">
                    <div className="mx-auto max-w-3xl text-center">
                        <h1 className="text-4xl font-bold tracking-tight sm:text-5xl">
                            Simple, Transparent Pricing
                        </h1>
                        <p className="mt-4 text-lg text-zinc-400">
                            Loading pricing plans...
                        </p>
                    </div>
                </main>
            </div>
        }>
            <PricingContent />
        </Suspense>
    );
}
