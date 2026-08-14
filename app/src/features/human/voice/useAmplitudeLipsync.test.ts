import { act, renderHook } from '@testing-library/react';
import { createRef } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { type RealtimeVoiceAudio } from './amplitudeLipsync';
import { useAmplitudeLipsync } from './useAmplitudeLipsync';

/** Drive the rAF loop by hand so frames are deterministic. */
let frames: FrameRequestCallback[] = [];

function flushFrames(count: number): void {
  for (let i = 0; i < count; i += 1) {
    const pending = frames;
    frames = [];
    act(() => pending.forEach(cb => cb(performance.now())));
  }
}

function audioRef(overrides: Partial<RealtimeVoiceAudio> = {}) {
  const ref = createRef<RealtimeVoiceAudio>() as { current: RealtimeVoiceAudio };
  ref.current = { getOutputVolume: null, speaking: false, ...overrides };
  return ref;
}

describe('useAmplitudeLipsync', () => {
  beforeEach(() => {
    frames = [];
    vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => {
      frames.push(cb);
      return frames.length;
    });
    vi.stubGlobal('cancelAnimationFrame', () => {});
  });
  afterEach(() => vi.unstubAllGlobals());

  it('stays inactive and rested while nothing is speaking', () => {
    const { result } = renderHook(() => useAmplitudeLipsync(audioRef()));
    flushFrames(3);
    expect(result.current).toEqual({ active: false, visemeCode: 'sil' });
  });

  it('drives the mouth from output loudness while the agent speaks', () => {
    const ref = audioRef({ speaking: true, getOutputVolume: () => 0.9 });
    const { result } = renderHook(() => useAmplitudeLipsync(ref));
    flushFrames(10);
    expect(result.current.active).toBe(true);
    expect(result.current.visemeCode).toBe('aa');
  });

  // The mouth must return to rest when the turn ends, not freeze on its last
  // shape — a stuck-open mascot is worse than one that never moved.
  it('rests the mouth when speaking stops', () => {
    const ref = audioRef({ speaking: true, getOutputVolume: () => 0.9 });
    const { result } = renderHook(() => useAmplitudeLipsync(ref));
    flushFrames(10);
    expect(result.current.visemeCode).toBe('aa');

    ref.current.speaking = false;
    flushFrames(2);
    expect(result.current).toEqual({ active: false, visemeCode: 'sil' });
  });

  // The SDK reads a live analyser; a session torn down mid-frame throws rather
  // than returning 0. An uncaught throw would kill the loop and freeze the mouth.
  it('survives an accessor that throws mid-session', () => {
    const ref = audioRef({
      speaking: true,
      getOutputVolume: () => {
        throw new Error('analyser closed');
      },
    });
    const { result } = renderHook(() => useAmplitudeLipsync(ref));
    expect(() => flushFrames(5)).not.toThrow();
    expect(result.current.visemeCode).toBe('sil');
  });

  // Smoothing toward 0 would hold the mouth open for ~16 frames after the audio
  // is already gone, so a bad reading resets on the very next frame.
  it('rests immediately when the accessor throws after a loud sample', () => {
    let volume = 0.9;
    const ref = audioRef({
      speaking: true,
      getOutputVolume: () => {
        if (volume < 0) throw new Error('analyser closed');
        return volume;
      },
    });
    const { result } = renderHook(() => useAmplitudeLipsync(ref));
    flushFrames(10);
    expect(result.current.visemeCode).toBe('aa');

    volume = -1; // next call throws
    flushFrames(1);
    expect(result.current).toEqual({ active: false, visemeCode: 'sil' });
  });

  // A non-finite sample smoothed into the level poisons it permanently: every
  // later sample stays NaN and the mouth never animates again for the rest of
  // the call. Guarding only at the viseme mapping hides that as a quiet mouth.
  it('recovers on the next valid sample after a non-finite reading', () => {
    let volume = 0.9;
    const ref = audioRef({ speaking: true, getOutputVolume: () => volume });
    const { result } = renderHook(() => useAmplitudeLipsync(ref));
    flushFrames(10);
    expect(result.current.visemeCode).toBe('aa');

    volume = Number.NaN;
    flushFrames(1);
    expect(result.current).toEqual({ active: false, visemeCode: 'sil' });

    // The level must not have been poisoned — real audio animates again.
    volume = 0.9;
    flushFrames(10);
    expect(result.current).toEqual({ active: true, visemeCode: 'aa' });
  });

  it('cancels its frame loop on unmount', () => {
    const cancel = vi.fn();
    vi.stubGlobal('cancelAnimationFrame', cancel);
    const { unmount } = renderHook(() => useAmplitudeLipsync(audioRef()));
    unmount();
    expect(cancel).toHaveBeenCalled();
  });
});
