// Payment API Type Definitions
// Based on AlphaHuman Payment Integration API Documentation

// ============================================================================
// Common Types
// ============================================================================

export type PlanType = 'FREE' | 'BASIC' | 'PRO';
export type StripePlanType = 'BASIC_MONTHLY' | 'BASIC_YEARLY' | 'PRO_MONTHLY' | 'PRO_YEARLY';
export type CoinbasePlanType = 'BASIC' | 'PRO';
export type PaymentGateway = 'coinbase' | 'stripe' | 'paypal';
export type ChargeStatus = 'created' | 'pending' | 'confirmed' | 'failed';
export type PriceInterval = 'day' | 'week' | 'month' | 'year';
export type PriceType = 'recurring' | 'one_time';

// ============================================================================
// Stripe Types
// ============================================================================

/**
 * Price information for a Stripe plan
 */
export interface PlanPrice {
  priceId: string;
  amount: number;
  currency: string;
  interval: PriceInterval;
  intervalCount: number;
  type: PriceType;
}

/**
 * Stripe plan with product and pricing information
 */
export interface Plan {
  productId: string;
  productName: string;
  productDescription: string | null;
  prices: PlanPrice[];
}

/**
 * Response for GET /payments/stripe/plans
 */
export interface GetPlansResponse {
  success: true;
  data: {
    plans: Plan[];
    totalPlans: number;
  };
}

/**
 * Request body for POST /payments/stripe/purchasePlan
 */
export interface PurchasePlanRequest {
  plan: StripePlanType;
}

/**
 * Response for POST /payments/stripe/purchasePlan
 */
export interface PurchasePlanResponse {
  success: true;
  data: {
    checkoutUrl: string | null;
    sessionId: string;
  };
}

/**
 * Query parameters for GET /payments/stripe/subscription/success
 */
export interface SubscriptionSuccessQuery {
  session_id: string; // Required: Stripe checkout session ID
}

/**
 * Response for GET /payments/stripe/subscription/success
 */
export interface SubscriptionSuccessResponse {
  success: true;
  data: {
    sessionId: string;
    subscriptionId: string;
    status: string;
    customerId: string | null;
    paymentStatus: string;
  };
}

/**
 * Response for GET /payments/stripe/subscription/cancel
 */
export interface SubscriptionCancelResponse {
  success: true;
  data: {
    canceled: boolean;
    redirectUrl?: string; // Only present if Accept header includes text/html
  };
}

/**
 * Subscription details (nested in CurrentPlanResponse)
 */
export interface SubscriptionDetails {
  id: string;
  status: string;
  currentPeriodEnd: string;
}

/**
 * Response for GET /payments/stripe/currentPlan
 */
export interface CurrentPlanResponse {
  success: true;
  data: {
    plan: PlanType;
    hasActiveSubscription: boolean;
    planExpiry: string | null;
    subscription?: SubscriptionDetails | null;
  };
}

// ============================================================================
// Coinbase Types
// ============================================================================

/**
 * Request body for POST /payments/coinbase/charge
 */
export interface CreateChargeRequest {
  plan: CoinbasePlanType; // Required
  currency?: string; // Optional, defaults to "USD"
  metadata?: Record<string, unknown>; // Optional
}

/**
 * Response for POST /payments/coinbase/charge
 */
export interface CreateChargeResponse {
  success: true;
  data: {
    gatewayTransactionId: string;
    hostedUrl: string;
    status: ChargeStatus;
    expiresAt: string; // ISO 8601 date string
  };
}

/**
 * Query parameters for GET /payments/coinbase/charge/{gatewayTransactionId}
 */
export interface ChargeStatusQuery {
  sync?: boolean; // Optional: Whether to sync with Coinbase Commerce. Default: false
}

/**
 * Response for GET /payments/coinbase/charge/{gatewayTransactionId}
 */
export interface ChargeStatusResponse {
  success: true;
  data: {
    gatewayTransactionId: string;
    paymentGateway: PaymentGateway;
    tgUserId?: string;
    amount: number;
    currency: string;
    status: ChargeStatus;
    hostedUrl?: string;
    expiresAt?: string; // ISO 8601 date string
    isExpired?: boolean;
    metadata?: Record<string, unknown>;
  };
}

// ============================================================================
// Error Response Types
// ============================================================================

/**
 * Standard error response format
 */
export interface ErrorResponse {
  error: string;
}

// ============================================================================
// API Response Wrapper
// ============================================================================

/**
 * Generic API response wrapper
 */
export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}
