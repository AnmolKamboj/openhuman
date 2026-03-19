"use client";

import { useEffect } from "react";
import { useSearchParams } from "next/navigation";

const STORAGE_KEY = "global_token";

/**
 * Gets the JWT token from localStorage.
 * Returns null if token is not found or localStorage is unavailable.
 */
export function getAuthToken(): string | null {
  if (typeof window === "undefined") {
    return null;
  }
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    // Ignore storage errors (e.g. disabled cookies, private browsing)
    return null;
  }
}

/**
 * Reads the bearer token from the current URL (?token=...) and
 * persists it to localStorage for reuse across pages.
 *
 * Example URL:
 *   https://localhost:3000/pricing?token=${globalToken}
 */
export function useAuthToken() {
  const searchParams = useSearchParams();

  useEffect(() => {
    const urlToken = searchParams.get("token");

    if (urlToken) {
      try {
        localStorage.setItem(STORAGE_KEY, urlToken);
      } catch {
        // Ignore storage errors (e.g. disabled cookies)
      }
      return;
    }

    // Fallback: use previously stored token
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
      }
    } catch {
      // Ignore storage errors
    }
  }, [searchParams]);

  // Return the token for component use
  return getAuthToken();
}

