import { createSlice, PayloadAction, createAsyncThunk } from '@reduxjs/toolkit';
import { clearUser } from './userSlice';

interface AuthState {
  token: string | null;
  isOnboarded: boolean;
}

const initialState: AuthState = {
  token: null,
  isOnboarded: false,
};

const authSlice = createSlice({
  name: 'auth',
  initialState,
  reducers: {
    setToken: (state, action: PayloadAction<string>) => {
      state.token = action.payload;
    },
    _clearToken: (state) => {
      state.token = null;
      state.isOnboarded = false;
    },
    setOnboarded: (state, action: PayloadAction<boolean>) => {
      state.isOnboarded = action.payload;
    },
  },
});

// Thunk that clears both token and user data
export const clearToken = createAsyncThunk('auth/clearToken', async (_, { dispatch }) => {
  dispatch(authSlice.actions._clearToken());
  dispatch(clearUser());
});

export const { setToken, setOnboarded } = authSlice.actions;
export default authSlice.reducer;
