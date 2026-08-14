import { useMemo, useRef, useState } from 'react';

import { useT } from '../../lib/i18n/I18nContext';
import { useAppDispatch, useAppSelector } from '../../store/hooks';
import {
  selectCustomMascotGifUrl,
  selectCustomPrimaryColor,
  selectCustomSecondaryColor,
  selectMascotColor,
  selectSpeakReplies,
  setSpeakReplies,
} from '../../store/mascotSlice';
import { HUMAN_VOICE_REALTIME_ENABLED, HUMAN_VOICE_SHOW_BOTH } from '../../utils/config';
import Conversations from '../conversations/Conversations';
import {
  CustomGifMascot,
  getMascotPalette,
  hexToArgbInt,
  ManifestRiveMascot,
  RiveMascot,
} from './Mascot';
import { useMascotManifest } from './Mascot/manifest/useMascotManifest';
import RealtimeVoiceControls from './RealtimeVoiceControls';
import { useHumanMascot } from './useHumanMascot';
import { IDLE_REALTIME_VOICE_AUDIO, type RealtimeVoiceAudio } from './voice/amplitudeLipsync';
import { useAmplitudeLipsync } from './voice/useAmplitudeLipsync';
import { resolveHumanVoiceEntry } from './voiceEntry';

const HumanPage = () => {
  const { t } = useT();
  const dispatch = useAppDispatch();
  // Reads the shared preference rather than the old
  // `localStorage['human.speakReplies']` this page used to own. That key is
  // consumed and deleted by the mascot slice's persist migration, so keeping the
  // local copy would leave this page and the chat mascot disagreeing about the
  // same setting — and would silently drop whatever the user had chosen before.
  const speakReplies = useAppSelector(selectSpeakReplies);

  const { face, visemeCode } = useHumanMascot({ speakReplies });

  // Lip-sync for the realtime voice session. The session lives inside
  // RealtimeVoiceControls (which owns its own ConversationProvider), so it
  // publishes its output-loudness accessor into this ref and the mascot samples
  // it per frame — a 60fps signal must not travel through React state.
  const realtimeAudioRef = useRef<RealtimeVoiceAudio>({ ...IDLE_REALTIME_VOICE_AUDIO });
  // The agent's speaking edge, lifted out of RealtimeVoiceControls so it can gate
  // the lip-sync loop below. Flips a couple of times per turn, so it is cheap as
  // state (the 60fps amplitude stays in the ref). While it is false — an idle
  // realtime session, or the classic voice path that never mounts the control —
  // the loop schedules no frames at all.
  const [realtimeSpeaking, setRealtimeSpeaking] = useState(false);
  const realtimeLipsync = useAmplitudeLipsync(realtimeAudioRef, realtimeSpeaking);

  // While the agent is speaking its own audio drives the mouth; otherwise the
  // classic path keeps ownership, so the two never fight over the same frame.
  const mascotFace = realtimeLipsync.active ? 'speaking' : face;
  const mascotVisemeCode = realtimeLipsync.active ? realtimeLipsync.visemeCode : visemeCode;
  const mascotColor = useAppSelector(selectMascotColor);
  const customPrimary = useAppSelector(selectCustomPrimaryColor);
  const customSecondary = useAppSelector(selectCustomSecondaryColor);
  const customMascotGifUrl = useAppSelector(selectCustomMascotGifUrl);
  // Active mascot resolved from the GitHub manifest (selection + default).
  const { entry: mascotEntry } = useMascotManifest();
  const palette = getMascotPalette(mascotColor);
  const primaryColor = useMemo(
    () => hexToArgbInt(mascotColor === 'custom' ? customPrimary : palette.bodyFill),
    [mascotColor, customPrimary, palette]
  );
  const secondaryColor = useMemo(
    () => hexToArgbInt(mascotColor === 'custom' ? customSecondary : palette.neckShadowColor),
    [mascotColor, customSecondary, palette]
  );

  // Which voice control the tab offers. Build-flag driven (#5399). In the
  // single-control modes the realtime button takes the slot the push-to-talk mic
  // used to own, so the tab has one voice affordance rather than two competing
  // ones. `both` keeps them apart instead of stacking them — the realtime button
  // floats over the mascot stage where it used to live, the card keeps
  // tap-and-speak — so the two paths stay visually distinct while being compared.
  const voiceEntry = resolveHumanVoiceEntry({
    realtimeEnabled: HUMAN_VOICE_REALTIME_ENABLED,
    showBoth: HUMAN_VOICE_SHOW_BOTH,
  });

  // The mascot drives a ~60fps lipsync re-render while the agent is speaking
  // (useHumanMascot forces a frame each rAF tick). Conversations is a heavy
  // subtree, so co-rendering it here would reconcile the whole chat tree every
  // frame and starve the main thread — which is what made tab switching feel
  // locked during TTS playback (#5357). Its props are constant, so hold a stable
  // element: React short-circuits reconciliation of an unchanged child, keeping
  // the per-frame mascot re-render off the chat tree and the UI responsive.
  // `voiceEntry` is build-time constant, so it cannot invalidate this memo at
  // runtime — it is in the deps only to keep the dependency honest.
  const chatPanel = useMemo(
    () => (
      <Conversations
        variant="sidebar"
        composer="mic-cloud"
        voiceChatControl={
          voiceEntry === 'realtime' ? (
            <RealtimeVoiceControls
              audioRef={realtimeAudioRef}
              onSpeakingChange={setRealtimeSpeaking}
            />
          ) : null
        }
        showMicComposer={voiceEntry !== 'realtime'}
        projectThreadList
      />
    ),
    [voiceEntry]
  );

  return (
    <div className="absolute inset-0 bg-surface-subtle dark:bg-surface-canvas overflow-hidden">
      <div
        className="pointer-events-none absolute inset-0"
        style={{
          background: 'radial-gradient(ellipse at 35% 40%, rgba(74,131,221,0.10), transparent 60%)',
        }}
      />

      {/* Mascot stage — fills the area to the left of the reserved chat column. */}
      <div className="absolute inset-y-0 left-0 right-[436px] flex items-center justify-center">
        <div className="relative w-[min(80vh,90%)] aspect-square">
          {customMascotGifUrl ? (
            <CustomGifMascot src={customMascotGifUrl} face={mascotFace} />
          ) : mascotEntry ? (
            <ManifestRiveMascot
              key={mascotEntry.id}
              entry={mascotEntry}
              face={mascotFace}
              primaryColor={primaryColor}
              secondaryColor={secondaryColor}
              visemeCode={mascotVisemeCode}
              idlePoseRotation
            />
          ) : (
            <RiveMascot
              face={mascotFace}
              primaryColor={primaryColor}
              secondaryColor={secondaryColor}
              visemeCode={mascotVisemeCode}
              idlePoseRotation
            />
          )}
        </div>
      </div>

      {/* Comparison mode only: the realtime control keeps its own place over the
          mascot stage, so it reads as a separate path from the card's
          tap-and-speak rather than a second button stacked on it. */}
      {voiceEntry === 'both' && (
        <div className="absolute bottom-8 left-0 right-[436px] z-10 flex justify-center">
          <RealtimeVoiceControls
            audioRef={realtimeAudioRef}
            onSpeakingChange={setRealtimeSpeaking}
          />
        </div>
      )}

      <label className="absolute top-4 left-4 z-10 inline-flex items-center gap-2 px-3 py-1.5 rounded-full bg-surface/80 backdrop-blur-sm border border-line-strong text-xs text-content-secondary shadow-soft cursor-pointer select-none">
        <input
          type="checkbox"
          checked={speakReplies}
          onChange={e => dispatch(setSpeakReplies(e.target.checked))}
          className="cursor-pointer"
        />
        {t('voice.pushToTalk')}
      </label>

      {/* Chat panel — kept on the right (the Human page is intentionally the
          one surface that leaves the root sidebar's dynamic region empty). */}
      <div className="absolute right-4 top-4 bottom-4 z-10 flex items-center">
        <aside className="w-[420px] h-[min(760px,100%)] rounded-2xl border border-line-strong bg-surface shadow-soft flex flex-col overflow-hidden">
          {/* Right-rail chat, but its thread list is surfaced in the (otherwise
              empty) root sidebar so the Human page shows the user's threads.
              Held as a stable element (chatPanel) so mascot lipsync re-renders
              don't reconcile it — see #5357. */}
          {chatPanel}
        </aside>
      </div>
    </div>
  );
};

export default HumanPage;
