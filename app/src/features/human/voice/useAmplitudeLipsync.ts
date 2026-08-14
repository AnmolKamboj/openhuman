import { type RefObject, useEffect, useRef, useState } from 'react';

import {
  amplitudeToVisemeCode,
  type RealtimeVoiceAudio,
  smoothAmplitude,
} from './amplitudeLipsync';

export interface AmplitudeLipsync {
  /** True while the realtime agent is speaking and driving the mouth. */
  active: boolean;
  /** Viseme code for Rive's `mouthVisemeCode` input. */
  visemeCode: string;
}

/**
 * Drive the mascot's mouth from the realtime session's output loudness.
 *
 * Runs an animation-frame loop only while the agent is speaking, so an idle
 * Human tab schedules no frames. Reads the SDK accessor out of a ref rather
 * than props because the session lives inside `RealtimeVoiceControls` (which
 * owns its own `ConversationProvider`) — see `RealtimeVoiceAudio`.
 *
 * State is committed only when the viseme code actually changes. The smoothed
 * level moves every frame, but the code it maps to steps between four values,
 * so re-rendering on the raw level would reconcile the page ~60 times a second
 * to produce the same mouth — the exact cost the chat panel is memoised to
 * avoid (#5357).
 */
export function useAmplitudeLipsync(audio: RefObject<RealtimeVoiceAudio>): AmplitudeLipsync {
  const [state, setState] = useState<AmplitudeLipsync>({ active: false, visemeCode: 'sil' });
  const levelRef = useRef(0);
  // Mirrors `state` for the loop to compare against without re-subscribing the
  // effect on every change.
  const codeRef = useRef('sil');
  const activeRef = useRef(false);

  useEffect(() => {
    let raf = 0;
    let stopped = false;

    const commit = (active: boolean, visemeCode: string): void => {
      if (active === activeRef.current && visemeCode === codeRef.current) return;
      activeRef.current = active;
      codeRef.current = visemeCode;
      setState({ active, visemeCode });
    };

    const tick = (): void => {
      if (stopped) return;
      const { getOutputVolume, speaking } = audio.current;
      if (!speaking || !getOutputVolume) {
        levelRef.current = 0;
        commit(false, 'sil');
      } else {
        // The SDK reads from a live analyser; a session torn down mid-frame
        // makes this throw rather than return 0, which would kill the loop and
        // freeze the mouth open for the rest of the call.
        let sample = 0;
        try {
          sample = getOutputVolume();
        } catch {
          sample = 0;
        }
        levelRef.current = smoothAmplitude(levelRef.current, sample);
        commit(true, amplitudeToVisemeCode(levelRef.current));
      }
      raf = window.requestAnimationFrame(tick);
    };

    raf = window.requestAnimationFrame(tick);
    return () => {
      stopped = true;
      window.cancelAnimationFrame(raf);
    };
  }, [audio]);

  return state;
}
