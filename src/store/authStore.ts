import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';

interface AuthState {
  token: string | null;
  setToken: (token: string) => void;
  clearToken: () => void;
  isAuthenticated: () => boolean;
}

const STORAGE_KEY = 'auth-storage';

// Custom storage that syncs with localStorage 'sessionToken' for backward compatibility
const customStorage = {
  getItem: (name: string): string | null => {
    // First check Zustand's persisted storage
    const zustandData = localStorage.getItem(name);
    if (zustandData) {
      try {
        const parsed = JSON.parse(zustandData);
        if (parsed.state?.token) {
          return zustandData;
        }
      } catch {
        // Ignore parse errors
      }
    }
    
    // Fallback to legacy sessionToken for backward compatibility
    const legacyToken = localStorage.getItem('sessionToken');
    if (legacyToken) {
      return JSON.stringify({ state: { token: legacyToken }, version: 0 });
    }
    
    return null;
  },
  setItem: (name: string, value: string): void => {
    localStorage.setItem(name, value);
    // Also sync to sessionToken for backward compatibility
    try {
      const parsed = JSON.parse(value);
      if (parsed.state?.token) {
        localStorage.setItem('sessionToken', parsed.state.token);
      }
    } catch {
      // Ignore parse errors
    }
  },
  removeItem: (name: string): void => {
    localStorage.removeItem(name);
    localStorage.removeItem('sessionToken');
  },
};

export const useAuthStore = create<AuthState>()(
  persist(
    (set, get) => ({
      token: null,

      setToken: (token: string) => {
        set({ token });
      },

      clearToken: () => {
        set({ token: null });
        // Also clear user data
        localStorage.removeItem('user');
      },

      isAuthenticated: () => {
        return get().token !== null;
      },
    }),
    {
      name: STORAGE_KEY,
      storage: createJSONStorage(() => customStorage),
      // Only persist the token
      partialize: (state) => ({ token: state.token }),
    }
  )
);
