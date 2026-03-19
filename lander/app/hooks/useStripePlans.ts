"use client";

import { useEffect, useState } from "react";
import { getStripePlans } from "@/lib/payments";
import type { Plan } from "@/lib/types";

interface TransformedPlan {
  name: string;
  monthlyPrice: number;
  annualPrice: number;
  description: string;
  features: string[];
  cta: string;
  popular: boolean;
  type: "free" | "basic" | "pro";
  monthlyPriceId?: string;
  annualPriceId?: string;
}

interface UseStripePlansReturn {
  plans: TransformedPlan[];
  loading: boolean;
  error: string | null;
  refetch: () => Promise<void>;
}

/**
 * Hook to fetch and transform Stripe plans for display in the pricing page.
 * 
 * Transforms the API response into a format suitable for the UI:
 * - Groups plans by type (Basic/Pro)
 * - Separates monthly and yearly prices
 * - Converts amounts from cents to dollars
 * - Includes a hardcoded Free plan
 */
export function useStripePlans(): UseStripePlansReturn {
  const [plans, setPlans] = useState<TransformedPlan[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchPlans = async () => {
    try {
      setLoading(true);
      setError(null);

      const response = await getStripePlans();

      if (!response.success || !response.data) {
        throw new Error("Failed to fetch plans");
      }

      // Transform API plans into UI format
      const transformedPlans: TransformedPlan[] = [];

      // Group plans by type (Basic/Pro) and separate monthly/yearly
      const planMap = new Map<"basic" | "pro", { monthly?: Plan; yearly?: Plan }>();

      response.data.plans.forEach((plan) => {
        const planName = plan.productName.toLowerCase();
        let planType: "basic" | "pro" | null = null;

        if (planName.includes("basic")) {
          planType = "basic";
        } else if (planName.includes("pro")) {
          planType = "pro";
        }

        if (!planType) return;

        // Find monthly and yearly prices
        const monthlyPrice = plan.prices.find((p) => p.interval === "month");
        const yearlyPrice = plan.prices.find((p) => p.interval === "year");

        if (!planMap.has(planType)) {
          planMap.set(planType, {});
        }

        const existing = planMap.get(planType)!;
        if (monthlyPrice) {
          existing.monthly = plan;
        }
        if (yearlyPrice) {
          existing.yearly = plan;
        }
      });

      // Convert to transformed format
      planMap.forEach((planData, planType) => {
        const monthlyPlan = planData.monthly;
        const yearlyPlan = planData.yearly;

        const monthlyPriceObj = monthlyPlan?.prices.find((p) => p.interval === "month");
        const yearlyPriceObj = yearlyPlan?.prices.find((p) => p.interval === "year");

        const monthlyPrice = monthlyPriceObj ? monthlyPriceObj.amount / 100 : 0; // Convert cents to dollars
        const annualPrice = yearlyPriceObj ? yearlyPriceObj.amount / 100 : 0;

        // Default features based on plan type
        const features =
          planType === "basic"
            ? [
                "Extended limits on Intelligence",
                "Image Processing",
                "3 External Connections",
                "15 Recaps/Month",
                "One Device",
              ]
            : [
                "Highest Limits on Intelligence",
                "Unlimited External Connections",
                "Fastest Models Available",
                "Unlimited Devices",
                "Unlimited Recaps",
              ];

        transformedPlans.push({
          name: planType === "basic" ? "Basic" : "Pro",
          monthlyPrice,
          annualPrice,
          description:
            planType === "basic"
              ? "For starters. Everything in Free, plus:"
              : "For power users. Everything in Basic, plus:",
          features,
          cta: "Subscribe with Card",
          popular: planType === "pro",
          type: planType as "basic" | "pro",
          monthlyPriceId: monthlyPriceObj?.priceId,
          annualPriceId: yearlyPriceObj?.priceId,
        });
      });

      // Add Free plan at the beginning
      transformedPlans.unshift({
        name: "Free",
        monthlyPrice: 0,
        annualPrice: 0,
        description: "Perfect for getting started",
        features: [
          "Limited access to Intelligence",
          "Limited and Slower Models",
          "1 External Connection",
          "3 Recaps/Month",
          "One Device",
        ],
        cta: "Get Started",
        popular: false,
        type: "free",
      });

      setPlans(transformedPlans);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : "Failed to load plans";
      setError(errorMessage);
      console.error("Error fetching Stripe plans:", err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchPlans();
  }, []);

  return {
    plans,
    loading,
    error,
    refetch: fetchPlans,
  };
}
