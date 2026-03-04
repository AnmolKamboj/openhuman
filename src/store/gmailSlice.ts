import { createSlice, type PayloadAction } from '@reduxjs/toolkit';

export interface GmailEmailSummary {
  id: string;
  threadId: string;
  snippet?: string;
  subject?: string;
  from?: string;
  date?: string;
}

export interface GmailProfile {
  email_address: string;
  messages_total: number;
  threads_total: number;
  history_id: string;
}

interface GmailState {
  /** Emails fetched after OAuth connection (from Gmail skill) */
  emails: GmailEmailSummary[];
  /** Profile of the connected Gmail user (from Gmail skill) */
  profile: GmailProfile | null;
}

const initialState: GmailState = { emails: [], profile: null };

const gmailSlice = createSlice({
  name: 'gmail',
  initialState,
  reducers: {
    setGmailEmails(state, action: PayloadAction<GmailEmailSummary[]>) {
      state.emails = action.payload;
    },
    clearGmailEmails(state) {
      state.emails = [];
    },
    setGmailProfile(state, action: PayloadAction<GmailProfile | null>) {
      state.profile = action.payload;
    },
    clearGmailProfile(state) {
      state.profile = null;
    },
  },
});

export const { setGmailEmails, clearGmailEmails, setGmailProfile, clearGmailProfile } =
  gmailSlice.actions;
export default gmailSlice.reducer;
