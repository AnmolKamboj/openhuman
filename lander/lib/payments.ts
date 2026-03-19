import { getAuthToken } from '@/app/hooks/useAuthToken';
import type {
  GetPlansResponse,
  PurchasePlanRequest,
  PurchasePlanResponse,
  SubscriptionSuccessResponse,
  SubscriptionCancelResponse,
  CurrentPlanResponse,
  CreateChargeRequest,
  CreateChargeResponse,
  ChargeStatusResponse,
  ErrorResponse,
} from './types';

const OPENHUMAN_API_BASE_URL = process.env.NEXT_PUBLIC_API_URL;

type HttpMethod = "GET" | "POST";

interface ApiFetchOptions extends RequestInit {
  method?: HttpMethod;
  token?: string;
}

/**
 * Generic API fetch function with error handling
 */
async function apiFetch<T = unknown>(
  path: string,
  { method = "GET", token, headers, body, ...rest }: ApiFetchOptions = {}
): Promise<T> {
  const mergedHeaders: Record<string, string> = {
    ...(headers as Record<string, string> | undefined),
  };

  if (body && !(body instanceof FormData)) {
    mergedHeaders["Content-Type"] = mergedHeaders["Content-Type"] || "application/json";
  }

  if (token) {
    mergedHeaders["Authorization"] = `Bearer ${token}`;
  }

  const response = await fetch(`${OPENHUMAN_API_BASE_URL}${path}`, {
    method,
    headers: mergedHeaders,
    body,
    ...rest,
  });

  if (!response.ok) {
    let errorMessage = `HTTP ${response.status}: ${response.statusText}`;
    try {
      const errorData = (await response.json()) as ErrorResponse;
      errorMessage = errorData.error || errorMessage;
    } catch {
      // If JSON parsing fails, use the text response
      const text = await response.text().catch(() => "");
      if (text) {
        errorMessage = text;
      }
    }
    throw new Error(errorMessage);
  }

  return response.json() as Promise<T>;
}

// ============================================================================
// Stripe Payments
// ============================================================================

/**
 * Gets all available subscription plans from Stripe.
 * Public endpoint (no authentication required).
 *
 * @returns Available plans with pricing information
 *
 * Docs:
 * https://openhuman.readme.io/reference/get_payments-stripe-plans
 */
export async function getStripePlans(): Promise<GetPlansResponse> {
  return apiFetch<GetPlansResponse>("/stripe/plans", {
    method: "GET",
  });
}

/**
 * Creates a Stripe Checkout Session for purchasing a subscription plan.
 * Requires authentication. Token is automatically fetched from localStorage.
 *
 * @param params - Purchase plan request with plan type
 * @returns Checkout session URL and session ID
 * @throws Error if token is not found in localStorage
 *
 * Docs:
 * https://openhuman.readme.io/reference/post_payments-stripe-purchaseplan
 */
export async function createStripeCheckoutSession(
  params: PurchasePlanRequest
): Promise<PurchasePlanResponse> {
  const token = getAuthToken();
  if (!token) {
    throw new Error("Authentication token not found. Please ensure you are logged in.");
  }

  return apiFetch<PurchasePlanResponse>("/stripe/purchasePlan", {
    method: "POST",
    token,
    body: JSON.stringify(params),
  });
}

/**
 * Verifies checkout success after Stripe redirect.
 * Public endpoint (no authentication required).
 *
 * @param sessionId - Stripe checkout session ID from query parameters
 * @returns Subscription activation details
 *
 * Docs:
 * https://openhuman.readme.io/reference/get_payments-stripe-subscription-success
 */
export async function handleStripeSubscriptionSuccess(
  sessionId: string
): Promise<SubscriptionSuccessResponse> {
  return apiFetch<SubscriptionSuccessResponse>(
    `/stripe/subscription/success?session_id=${encodeURIComponent(sessionId)}`,
    {
      method: "GET",
    }
  );
}

/**
 * Handles canceled subscription checkout redirect.
 * Public endpoint (no authentication required).
 *
 * @param acceptHtml - If true, sets Accept header to text/html for redirect
 * @returns Cancel confirmation
 *
 * Docs:
 * https://openhuman.readme.io/reference/get_payments-stripe-subscription-cancel
 */
export async function handleStripeSubscriptionCancel(
  acceptHtml: boolean = false
): Promise<SubscriptionCancelResponse> {
  const headers = acceptHtml ? { Accept: "text/html" } : undefined;
  return apiFetch<SubscriptionCancelResponse>("/stripe/subscription/cancel", {
    method: "GET",
    headers,
  });
}

/**
 * Returns the current subscription plan for the authenticated user.
 * Works for both Stripe and Coinbase subscriptions.
 * Requires authentication. Token is automatically fetched from localStorage.
 *
 * @returns Current plan details including subscription status
 * @throws Error if token is not found in localStorage
 *
 * Docs:
 * https://openhuman.readme.io/reference/get_payments-stripe-currentplan
 */
export async function getCurrentSubscriptionPlan(): Promise<CurrentPlanResponse> {
  const token = getAuthToken();
  if (!token) {
    throw new Error("Authentication token not found. Please ensure you are logged in.");
  }

  return apiFetch<CurrentPlanResponse>("/stripe/currentPlan", {
    method: "GET",
    token,
  });
}

// ============================================================================
// Coinbase Commerce Payments
// ============================================================================

/**
 * Creates a Coinbase Commerce charge for a subscription plan.
 * Requires authentication. Token is automatically fetched from localStorage.
 *
 * @param params - Charge creation request with plan type and optional currency/metadata
 * @returns Charge details including hosted URL and transaction ID
 * @throws Error if token is not found in localStorage
 *
 * Docs:
 * https://openhuman.readme.io/reference/post_payments-coinbase-charge
 */
export async function createCoinbaseCharge(
  params: CreateChargeRequest
): Promise<CreateChargeResponse> {
  const token = getAuthToken();
  if (!token) {
    throw new Error("Authentication token not found. Please ensure you are logged in.");
  }

  return apiFetch<CreateChargeResponse>("/coinbase/charge", {
    method: "POST",
    token,
    body: JSON.stringify(params),
  });
}

/**
 * Retrieves the status of a Coinbase Commerce charge.
 * Optionally syncs with Coinbase Commerce for the latest status.
 * Requires authentication. Token is automatically fetched from localStorage.
 *
 * @param gatewayTransactionId - Coinbase charge UUID
 * @param sync - Whether to sync with Coinbase Commerce for latest status (default: false)
 * @returns Charge status and details
 * @throws Error if token is not found in localStorage
 *
 * Docs:
 * https://openhuman.readme.io/reference/get_payments-coinbase-charge-gatewaytransactionid
 */
export async function getCoinbaseChargeStatus(
  gatewayTransactionId: string,
  sync: boolean = false
): Promise<ChargeStatusResponse> {
  const token = getAuthToken();
  if (!token) {
    throw new Error("Authentication token not found. Please ensure you are logged in.");
  }

  const queryParam = sync ? "?sync=true" : "";
  return apiFetch<ChargeStatusResponse>(
    `/coinbase/charge/${encodeURIComponent(gatewayTransactionId)}${queryParam}`,
    {
      method: "GET",
      token,
    }
  );
}

