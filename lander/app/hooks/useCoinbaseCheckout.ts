"use client";

import { useState, useCallback } from "react";
import { createCoinbaseCharge } from "@/lib/payments";
import type { CoinbasePlanType } from "@/lib/types";

interface UseCoinbaseCheckoutReturn {
  handleCheckout: (planType: "basic" | "pro") => Promise<void>;
  loading: boolean;
  error: string | null;
}

/**
 * Hook for handling Coinbase Commerce checkout flow.
 * 
 * Provides a callback function to initiate Coinbase checkout for a given plan type.
 * Manages loading state and error handling.
 * 
 * @returns Object containing checkout handler, loading state, and error state
 */
export function useCoinbaseCheckout(): UseCoinbaseCheckoutReturn {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleCheckout = useCallback(async (planType: "basic" | "pro") => {
    setLoading(true);
    setError(null);

    try {
      // Map UI plan types to API plan names
      const planMap: Record<"basic" | "pro", CoinbasePlanType> = {
        basic: "BASIC",
        pro: "PRO",
      };

      const planName = planMap[planType];

      const response = await createCoinbaseCharge({
        plan: planName,
        currency: "USD",
      });

      if (response.success && response.data?.hostedUrl) {
        window.location.href = response.data.hostedUrl;
      } else {
        throw new Error("No hosted URL received from server");
      }
    } catch (err) {
      const errorMessage =
        err instanceof Error
          ? err.message
          : "Unable to start crypto checkout. Please try again later.";
      setError(errorMessage);
      console.error("Failed to create Coinbase charge:", err);

      // Show user-friendly error message
      alert(errorMessage);
    } finally {
      setLoading(false);
    }
  }, []);

  return {
    handleCheckout,
    loading,
    error,
  };
}
