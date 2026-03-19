"use client";

import { useState, useCallback } from "react";
import { createStripeCheckoutSession } from "@/lib/payments";
import type { StripePlanType } from "@/lib/types";

interface UseStripeCheckoutReturn {
  handleCheckout: (planType: "basic" | "pro", billingCycle: "monthly" | "annual") => Promise<void>;
  loading: boolean;
  error: string | null;
}

/**
 * Hook for handling Stripe checkout flow.
 * 
 * Provides a callback function to initiate Stripe checkout for a given plan type and billing cycle.
 * Manages loading state and error handling.
 * 
 * @returns Object containing checkout handler, loading state, and error state
 */
export function useStripeCheckout(): UseStripeCheckoutReturn {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleCheckout = useCallback(
    async (planType: "basic" | "pro", billingCycle: "monthly" | "annual") => {
      setLoading(true);
      setError(null);

      try {
        // Map plan type and billing cycle to API plan format
        const planMap: Record<
          "basic" | "pro",
          { monthly: StripePlanType; yearly: StripePlanType }
        > = {
          basic: { monthly: "BASIC_MONTHLY", yearly: "BASIC_YEARLY" },
          pro: { monthly: "PRO_MONTHLY", yearly: "PRO_YEARLY" },
        };

        const plan = billingCycle === "monthly" ? planMap[planType].monthly : planMap[planType].yearly;

        const response = await createStripeCheckoutSession({
          plan,
        });

        const redirectUrl = response.data?.checkoutUrl;

        if (redirectUrl) {
          window.location.href = redirectUrl;
        } else {
          throw new Error("No checkout URL received from server");
        }
      } catch (err) {
        const errorMessage =
          err instanceof Error ? err.message : "Unable to start checkout. Please try again later.";
        setError(errorMessage);
        console.error("Failed to create Stripe Checkout Session:", err);
        
        // Show user-friendly error message
        alert(errorMessage);
      } finally {
        setLoading(false);
      }
    },
    []
  );

  return {
    handleCheckout,
    loading,
    error,
  };
}
