import { createSlice, PayloadAction, createAsyncThunk } from "@reduxjs/toolkit";
import { clearUser } from "./userSlice";

export interface AuthState {
  token: string | null;
  /** Onboarding completion per user id */
  isOnboardedByUser: Record<string, boolean>;
}

const initialState: AuthState = {
  token: null,
  isOnboardedByUser: {},
};

const authSlice = createSlice({
  name: "auth",
  initialState,
  reducers: {
    setToken: (state, action: PayloadAction<string>) => {
      state.token = action.payload;
    },
    _clearToken: (state) => {
      state.token = null;
    },
    setOnboardedForUser: (
      state,
      action: PayloadAction<{ userId: string; value: boolean }>,
    ) => {
      const { userId, value } = action.payload;
      state.isOnboardedByUser[userId] = value;
    },
  },
});

// Thunk that clears both token and user data
export const clearToken = createAsyncThunk(
  "auth/clearToken",
  async (_, { dispatch }) => {
    dispatch(authSlice.actions._clearToken());
    dispatch(clearUser());
  },
);

export const { setToken, setOnboardedForUser } = authSlice.actions;
export default authSlice.reducer;
