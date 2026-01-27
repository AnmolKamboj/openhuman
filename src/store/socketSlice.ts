import { createSlice, PayloadAction } from '@reduxjs/toolkit';

export type SocketConnectionStatus = 'connected' | 'disconnected' | 'connecting';

interface SocketState {
  status: SocketConnectionStatus;
  socketId: string | null;
}

const initialState: SocketState = {
  status: 'disconnected',
  socketId: null,
};

const socketSlice = createSlice({
  name: 'socket',
  initialState,
  reducers: {
    setStatus: (state, action: PayloadAction<SocketConnectionStatus>) => {
      state.status = action.payload;
    },
    setSocketId: (state, action: PayloadAction<string | null>) => {
      state.socketId = action.payload;
    },
    reset: (state) => {
      state.status = 'disconnected';
      state.socketId = null;
    },
  },
});

export const { setStatus, setSocketId, reset } = socketSlice.actions;
export default socketSlice.reducer;
