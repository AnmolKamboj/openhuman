"use client";

import { useEffect, useState } from "react";
import { getCurrentSubscriptionPlan } from "@/lib/payments";
import type { CurrentPlanResponse } from "@/lib/types";

interface UseCurrentSubscriptionPlanReturn {
  plan: CurrentPlanResponse["data"] | null;
  loading: boolean;
  error: string | null;
  refetch: () => Promise<void>;
}

/**
 * Hook to fetch the current subscription plan for the authenticated user.
 * Works for both Stripe and Coinbase subscriptions.
 * 
 * Returns the current plan details including:
 * - Plan type (FREE, BASIC, PRO)
 * - Active subscription status
 * - Plan expiry date
 * - Subscription details (if available)
 */
export function useCurrentSubscriptionPlan(): UseCurrentSubscriptionPlanReturn {
  const [plan, setPlan] = useState<CurrentPlanResponse["data"] | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchPlan = async () => {
    try {
      setLoading(true);
      setError(null);

      const response = await getCurrentSubscriptionPlan();

      if (!response.success || !response.data) {
        throw new Error("Failed to fetch current subscription plan");
      }

      setPlan(response.data);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : "Failed to load subscription plan";
      setError(errorMessage);
      console.error("Error fetching current subscription plan:", err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchPlan();
  }, []);

  return {
    plan,
    loading,
    error,
    refetch: fetchPlan,
  };
}
