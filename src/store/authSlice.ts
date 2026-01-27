import { createSlice, PayloadAction } from '@reduxjs/toolkit';

interface AuthState {
  token: string | null;
}

const initialState: AuthState = {
  token: null,
};

// Initialize from localStorage for backward compatibility
if (typeof window !== 'undefined') {
  const legacyToken = localStorage.getItem('sessionToken');
  if (legacyToken) {
    initialState.token = legacyToken;
  }
}

const authSlice = createSlice({
  name: 'auth',
  initialState,
  reducers: {
    setToken: (state, action: PayloadAction<string>) => {
      state.token = action.payload;
      // Also sync to localStorage for backward compatibility
      localStorage.setItem('sessionToken', action.payload);
    },
    clearToken: (state) => {
      state.token = null;
      // Also clear from localStorage for backward compatibility
      localStorage.removeItem('sessionToken');
      localStorage.removeItem('user');
    },
  },
});

export const { setToken, clearToken } = authSlice.actions;
export default authSlice.reducer;
