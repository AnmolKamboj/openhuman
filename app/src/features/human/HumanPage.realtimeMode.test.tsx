/**
 * Unit test for HumanPage's realtime voice overlay (#5399). The realtime
 * controls are now shown unconditionally on the Human tab — the former
 * build-flag + persisted-voice-mode gate was removed so the "Start Voice Chat"
 * control is always available — alongside the classic push-to-talk path.
 * RealtimeVoiceControls is stubbed so the ElevenLabs SDK never loads.
 */
import { configureStore } from '@reduxjs/toolkit';
import { render, screen } from '@testing-library/react';
import { Provider } from 'react-redux';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import chatRuntimeReducer from '../../store/chatRuntimeSlice';
import mascotReducer, { setVoiceMode } from '../../store/mascotSlice';
import threadReducer from '../../store/threadSlice';
import HumanPage from './HumanPage';

// Stub the overlay so the ElevenLabs `ConversationProvider`/SDK never mounts —
// this test only pins the render gate, not the controls (covered separately).
vi.mock('./RealtimeVoiceControls', () => ({
  default: () => <div data-testid="realtime-voice-controls-stub" />,
}));

vi.mock('../conversations/Conversations', () => ({
  default: () => <div data-testid="conversations-stub" />,
}));

vi.mock('./Mascot', async importOriginal => {
  const actual = await importOriginal<typeof import('./Mascot')>();
  return {
    ...actual,
    RiveMascot: () => <div data-testid="mascot-stub" />,
    CustomGifMascot: () => <img data-testid="custom-gif-mascot" alt="" />,
  };
});

vi.mock('./useHumanMascot', () => ({ useHumanMascot: () => ({ face: 'idle', visemes: [] }) }));
vi.mock('./Mascot/manifest/useMascotManifest', () => ({
  useMascotManifest: () => ({ manifest: null, entry: null, loading: false, error: null }),
}));

function renderWithVoiceMode(mode: 'classic' | 'realtime') {
  const store = configureStore({
    reducer: { mascot: mascotReducer, thread: threadReducer, chatRuntime: chatRuntimeReducer },
  });
  store.dispatch(setVoiceMode(mode));
  return render(
    <Provider store={store}>
      <HumanPage />
    </Provider>
  );
}

describe('HumanPage — realtime voice overlay', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('renders the realtime controls regardless of the persisted voice mode', () => {
    // The gate was removed, so the controls appear even when the persisted mode
    // is the classic default — the two paths now coexist on the Human tab.
    renderWithVoiceMode('classic');
    expect(screen.getByTestId('realtime-voice-controls-stub')).toBeInTheDocument();
  });

  it('still renders the realtime controls when the persisted mode is realtime', () => {
    renderWithVoiceMode('realtime');
    expect(screen.getByTestId('realtime-voice-controls-stub')).toBeInTheDocument();
  });
});
