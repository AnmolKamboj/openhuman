//! Calling the `tinyvoice` module: the voice primitives, over the bus.
//!
//! Each function here is the host half of one method on
//! `ai.tinyhumans.tinyvoice.Voice`. They exist so the voice domain does not
//! have to know about proxies, base64 framing, or wire error names — a caller
//! asks a question about a transcript or a buffer and gets an answer.
//!
//! # A call costs about 15 microseconds
//!
//! Measured on the real loaded module (`bench_call` in the `tinyvoice` repo):
//! ~15 µs per round trip, against a 20 ms audio frame. A TinyBus module shares
//! this address space — a call is a channel send and a JSON hop, not IPC.
//!
//! So there is no per-call budget to protect and nothing here is too hot to go
//! over the bus, including the VAD, which runs as a
//! [`VadSession`] driven from the always-on capture loop.
//!
//! **What stays on this side is decided by the audio callback, not by cost.**
//! `cpal` delivers on a realtime thread where blocking is a dropout, so the
//! callback converts the sample format and forwards raw interleaved samples;
//! every transform happens in an async worker that calls this module. That is
//! *less* work on the audio thread than the in-process version did, not more.
//!
//! # Failure is not fatal here
//!
//! Every function returns a [`VoiceCallError`] the caller can fall back from,
//! and the callers do. A module that will not load must degrade voice to its
//! pre-module behaviour — deferring to the agent, or skipping a filter — rather
//! than taking dictation down with it. The one thing none of them may do is
//! guess: see [`is_hallucinated`].

use serde::Deserialize;

use super::{host, ops, registry};
use crate::openhuman::config::Config;

/// Registry id of the module these calls go to.
const MODULE_ID: &str = "tinyvoice";

/// Why a voice call did not produce an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoiceCallError {
    /// The module is not loaded and cannot be: unsupported host, downloads off,
    /// disabled in config, or a load that already failed in this process.
    Unavailable(String),
    /// The call itself failed — a malformed payload, or a refused argument.
    Failed(String),
}

impl std::fmt::Display for VoiceCallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(message) | Self::Failed(message) => f.write_str(message),
        }
    }
}

/// Which hallucination list applies, mirroring `tinyvoice::transcript::Mode`.
///
/// Redeclared here rather than imported because this crate does not depend on
/// `tinyvoice` — the module is the only link, and its interface speaks strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HallucinationMode {
    /// Push-to-talk dictation. Aggressive.
    Dictation,
    /// Chat voice input. Conservative.
    Conversation,
}

impl HallucinationMode {
    /// The wire value the module expects.
    fn as_wire(self) -> &'static str {
        match self {
            Self::Dictation => "dictation",
            Self::Conversation => "conversation",
        }
    }
}

/// A recognised fast-path voice command, or `Unknown`.
///
/// Deserialized from the module's tagged JSON. The variants and their payload
/// names are the wire contract — renaming one here silently turns it into
/// `Unknown`, which is why [`VoiceIntent::Unknown`] carries the catch-all
/// `#[serde(other)]` and the tests below pin every tag.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "intent", rename_all = "snake_case")]
pub enum VoiceIntent {
    /// "play <song/artist>".
    Play {
        /// The cleaned search query.
        query: String,
    },
    /// Pause playback.
    Pause,
    /// Resume playback.
    Resume,
    /// Skip to the next track.
    Next,
    /// Go back to the previous track.
    Previous,
    /// "open/launch/start <app>".
    OpenApp {
        /// The cleaned application name.
        app: String,
    },
    /// "set volume to N", absolute `0..=100`.
    SetVolume {
        /// Target volume percentage.
        percent: u8,
    },
    /// Raise the volume.
    VolumeUp,
    /// Lower the volume.
    VolumeDown,
    /// Mute audio output.
    Mute,
    /// Unmute audio output.
    Unmute,
    /// Not a confident fast command — defer to the agent.
    #[serde(other)]
    Unknown,
}

impl VoiceIntent {
    /// A stable, **non-PII** variant name, for logs and metrics.
    ///
    /// Never includes the `query` / `app` payloads. This path is fed by an
    /// always-on microphone, so those fields can hold anything said in the
    /// room: a log line naming the variant is diagnostics, and one naming the
    /// query is a recording.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Play { .. } => "play",
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::Next => "next",
            Self::Previous => "previous",
            Self::OpenApp { .. } => "open_app",
            Self::SetVolume { .. } => "set_volume",
            Self::VolumeUp => "volume_up",
            Self::VolumeDown => "volume_down",
            Self::Mute => "mute",
            Self::Unmute => "unmute",
            Self::Unknown => "unknown",
        }
    }
}

/// Classify a command transcript into a fast-path intent.
///
/// The transcript should already have had its wake word removed by
/// [`extract_command`].
///
/// # Errors
///
/// [`VoiceCallError`] when the module is unavailable or the call fails. A
/// caller should treat that as [`VoiceIntent::Unknown`] and hand the transcript
/// to the agent — the fast path is an optimisation, and losing it costs a round
/// trip rather than the request.
pub async fn route(config: &Config, transcript: &str) -> Result<VoiceIntent, VoiceCallError> {
    let json: String = call(config, "Route", (transcript,)).await?;
    let intent: VoiceIntent = serde_json::from_str(&json)
        .map_err(|e| VoiceCallError::Failed(format!("could not decode intent: {e}")))?;
    Ok(intent.clamped())
}

impl VoiceIntent {
    /// Bring payloads back inside the range the executors assume.
    ///
    /// The module already clamps a spoken volume to `0..=100`, so in practice
    /// this changes nothing. It runs anyway because *this* type is decoded from
    /// a wire payload, and `percent` is interpolated straight into an
    /// `osascript` command by `voice::always_on::execute_intent`. A value the
    /// host never checked reaching a shell command is the shape of bug worth
    /// spending three lines to make impossible, rather than one that depends on
    /// a remote clamp staying correct.
    #[must_use]
    fn clamped(self) -> Self {
        match self {
            Self::SetVolume { percent } if percent > 100 => Self::SetVolume { percent: 100 },
            other => other,
        }
    }
}

/// Apply the wake-word gate, returning the command that followed it.
///
/// `None` means the utterance was not addressed to the agent, or the wake word
/// arrived with nothing after it. Those are the same outcome for a caller, and
/// the module represents both as an empty string.
///
/// # Errors
///
/// [`VoiceCallError`] when the module is unavailable or the call fails.
pub async fn extract_command(
    config: &Config,
    transcript: &str,
    wake_word: &str,
) -> Result<Option<String>, VoiceCallError> {
    let command: String = call(config, "ExtractCommand", (transcript, wake_word)).await?;
    Ok(if command.is_empty() {
        None
    } else {
        Some(command)
    })
}

/// Whether the wake word appears near the start of a transcript.
///
/// Distinguished from [`extract_command`] so a caller can acknowledge a bare
/// "Hey Tiny", which otherwise reads to the user as a dead microphone.
///
/// # Errors
///
/// [`VoiceCallError`] when the module is unavailable or the call fails.
pub async fn wake_word_present(
    config: &Config,
    transcript: &str,
    wake_word: &str,
) -> Result<bool, VoiceCallError> {
    call(config, "WakeWordPresent", (transcript, wake_word)).await
}

/// Whether an STT transcript looks like a hallucination rather than speech.
///
/// # Errors
///
/// [`VoiceCallError`] when the module is unavailable or the call fails.
///
/// **A caller that cannot reach the module must not guess.** Treating an error
/// as "hallucinated" silently deletes real speech; treating it as "clean" lets
/// `[BLANK_AUDIO]` reach the agent as an instruction. Of the two, passing the
/// text through is recoverable and losing it is not, so callers here fall open
/// — and say so at the call site rather than burying it in a default.
pub async fn is_hallucinated(
    config: &Config,
    text: &str,
    mode: HallucinationMode,
) -> Result<bool, VoiceCallError> {
    call(config, "IsHallucinated", (text, mode.as_wire())).await
}

/// Downmix, resample to 16 kHz, optionally silence-gate, and frame as WAV.
///
/// This is the whole capture-side pipeline in one call. Three separate calls
/// would ship the same audio across the bus three times to do work that is
/// microseconds of arithmetic.
///
/// `samples` are interleaved `f32`; `gate_threshold` of zero disables the
/// silence gate.
///
/// # Errors
///
/// [`VoiceCallError`], including a `Failed` when `samples` is not a whole
/// number of frames for `channels`.
pub async fn prepare_capture(
    config: &Config,
    samples: &[f32],
    source_rate: u32,
    channels: u16,
    gate_threshold: f32,
) -> Result<Vec<u8>, VoiceCallError> {
    let encoded = encode_samples(samples);
    let wav: String = call(
        config,
        "PrepareCapture",
        (encoded, source_rate, channels, gate_threshold),
    )
    .await?;
    decode_audio(&wav)
}

/// Frame mono `f32` samples as a 16-bit PCM WAV file.
///
/// # Errors
///
/// [`VoiceCallError`] when the module is unavailable or the call fails.
pub async fn encode_wav(
    config: &Config,
    samples: &[f32],
    sample_rate: u32,
) -> Result<Vec<u8>, VoiceCallError> {
    let encoded = encode_samples(samples);
    let wav: String = call(config, "EncodeWav", (encoded, sample_rate)).await?;
    decode_audio(&wav)
}

/// Tuning for a VAD session, mirroring `tinyvoice::vad::VadConfig`.
///
/// Built from `voice_server` config by [`VadConfig::from_server_config`]. The
/// module has no such constructor on purpose — it does not know what OpenHuman
/// persists — so the mapping lives here.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct VadConfig {
    /// Peak-RMS energy above which a frame counts as speech.
    pub onset_threshold: f32,
    /// How long energy must stay below `onset_threshold` before the utterance
    /// closes. Bridges natural mid-sentence pauses.
    pub hangover_ms: u32,
    /// Minimum voiced duration for a segment to be emitted.
    pub min_speech_ms: u32,
    /// Hard ceiling on a single utterance.
    pub max_utterance_ms: u32,
}

impl VadConfig {
    /// Build VAD tuning from the persisted voice-server config.
    #[must_use]
    pub fn from_server_config(c: &crate::openhuman::config::VoiceServerConfig) -> Self {
        Self {
            onset_threshold: c.vad_onset_threshold,
            hangover_ms: c.vad_hangover_ms,
            min_speech_ms: c.vad_min_speech_ms,
            // Config stores seconds; the module speaks milliseconds. Clamped to
            // at least 1ms so a zero or negative setting cannot make every
            // utterance close on its first frame.
            max_utterance_ms: (c.vad_max_utterance_secs * 1000.0).round().max(1.0) as u32,
        }
    }
}

/// What the segmenter reported at one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VadEvent {
    /// Energy crossed the onset threshold — an utterance has begun.
    SpeechStart {
        /// Index of the frame, within the batch that was pushed.
        frame: usize,
    },
    /// An utterance closed.
    SpeechEnd {
        /// Index of the frame, within the batch that was pushed.
        frame: usize,
        /// Accumulated speech duration, excluding the trailing silence.
        voiced_ms: u32,
        /// False when the segment was too short to be worth transcribing.
        emit: bool,
        /// True when the close was forced by the utterance ceiling.
        forced: bool,
    },
}

/// A live VAD session held by the module.
///
/// Not `Drop`-based: releasing it needs an async bus call, and a `Drop` impl
/// cannot await. Call [`close`](Self::close) when the capture loop stops. A
/// leaked session costs one map entry in the module until the process exits,
/// and the module caps how many can accumulate.
#[derive(Debug, Clone, Copy)]
pub struct VadSession {
    id: u64,
}

impl VadSession {
    /// Open a session with the given tuning.
    ///
    /// # Errors
    ///
    /// [`VoiceCallError`] when the module is unavailable or the config is
    /// rejected.
    pub async fn open(config: &Config, vad: VadConfig) -> Result<Self, VoiceCallError> {
        let json = serde_json::to_string(&vad)
            .map_err(|e| VoiceCallError::Failed(format!("could not encode VAD config: {e}")))?;
        let id: u64 = call(config, "VadOpen", (json,)).await?;
        Ok(Self { id })
    }

    /// Push a batch of frame energies and collect whatever the segmenter says.
    ///
    /// Frame indices in the returned events are relative to `energies`.
    ///
    /// # Errors
    ///
    /// [`VoiceCallError`] when the module is unavailable, the session is not
    /// open, or `frame_ms` is zero.
    pub async fn push(
        &self,
        config: &Config,
        frame_ms: u32,
        energies: &[f32],
    ) -> Result<Vec<VadEvent>, VoiceCallError> {
        let json: String = call(config, "VadPush", (self.id, frame_ms, energies)).await?;
        serde_json::from_str(&json)
            .map_err(|e| VoiceCallError::Failed(format!("could not decode VAD events: {e}")))
    }

    /// Whether the session is currently inside an utterance.
    ///
    /// # Errors
    ///
    /// [`VoiceCallError`] when the module is unavailable or the session is not
    /// open.
    pub async fn is_speaking(&self, config: &Config) -> Result<bool, VoiceCallError> {
        call(config, "VadIsSpeaking", (self.id,)).await
    }

    /// Abort any in-flight utterance without emitting an event.
    ///
    /// The privacy hook: called when the screen locks or capture is revoked, so
    /// a partial utterance is dropped rather than completed and transcribed.
    ///
    /// # Errors
    ///
    /// [`VoiceCallError`] when the module is unavailable or the session is not
    /// open.
    pub async fn reset(&self, config: &Config) -> Result<(), VoiceCallError> {
        call(config, "VadReset", (self.id,)).await
    }

    /// Release the session. Closing one that is already gone is not an error.
    ///
    /// # Errors
    ///
    /// [`VoiceCallError`] only when the module itself is unreachable.
    pub async fn close(&self, config: &Config) -> Result<(), VoiceCallError> {
        call(config, "VadClose", (self.id,)).await
    }
}

/// Downmix and resample a raw capture buffer to 16 kHz mono samples.
///
/// The sibling of [`prepare_capture`] for a live loop, which needs samples to
/// measure and accumulate rather than a finished container.
///
/// # Errors
///
/// [`VoiceCallError`], including a `Failed` when `samples` is not a whole
/// number of frames for `channels`.
pub async fn prepare_frames(
    config: &Config,
    samples: &[f32],
    source_rate: u32,
    channels: u16,
) -> Result<Vec<f32>, VoiceCallError> {
    let encoded: String = call(
        config,
        "PrepareFrames",
        (encode_samples(samples), source_rate, channels),
    )
    .await?;
    decode_samples(&encoded)
}

/// Root-mean-square energy of each fixed-size frame in a buffer.
///
/// # Errors
///
/// [`VoiceCallError`] when the module is unavailable or `frame_len` is zero.
pub async fn frame_energies(
    config: &Config,
    samples: &[f32],
    frame_len: u32,
) -> Result<Vec<f32>, VoiceCallError> {
    call(
        config,
        "FrameEnergies",
        (encode_samples(samples), frame_len),
    )
    .await
}

/// Frame 16-bit PCM samples as a WAV file, without touching the samples.
///
/// Distinct from [`encode_wav`] because a caller holding `i16` should not have
/// to widen to `f32` and let the module narrow back: that round trip is lossy
/// by one LSB for no reason. This path is exact.
///
/// # Errors
///
/// [`VoiceCallError`] when the module is unavailable or the call fails.
pub async fn encode_wav_pcm16(
    config: &Config,
    samples: &[i16],
    sample_rate: u32,
    channels: u16,
) -> Result<Vec<u8>, VoiceCallError> {
    use base64::Engine as _;
    let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    let wav: String = call(config, "EncodeWavPcm16", (encoded, sample_rate, channels)).await?;
    decode_audio(&wav)
}

/// Load the voice module if it is not already serving.
///
/// Callers do not have to invoke this — every operation above does it — but a
/// caller that wraps its work in a deadline should, *outside* that deadline. A
/// first use may download and verify an artifact, and charging that against a
/// dictation timeout means the first utterance a user ever speaks is the one
/// that fails.
///
/// # Errors
///
/// The same [`VoiceCallError::Unavailable`] the operations return.
pub async fn ensure_ready(config: &Config) -> Result<(), VoiceCallError> {
    ops::ensure_loaded(config, MODULE_ID)
        .await
        .map_err(VoiceCallError::Unavailable)
}

/// Base64 little-endian `f32`, which is how the interface carries samples.
fn encode_samples(samples: &[f32]) -> String {
    use base64::Engine as _;
    let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Decode base64 little-endian `f32` samples the module produced.
fn decode_samples(encoded: &str) -> Result<Vec<f32>, VoiceCallError> {
    let bytes = decode_audio(encoded)?;
    if !bytes.len().is_multiple_of(4) {
        return Err(VoiceCallError::Failed(format!(
            "module returned {} bytes, not a whole number of f32 samples",
            bytes.len()
        )));
    }
    Ok(bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect())
}

/// Decode a base64 audio payload the module produced.
fn decode_audio(encoded: &str) -> Result<Vec<u8>, VoiceCallError> {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| VoiceCallError::Failed(format!("module returned invalid base64: {e}")))
}

/// Ensure the module is serving, then make one call on it.
async fn call<A, R>(config: &Config, method: &str, args: A) -> Result<R, VoiceCallError>
where
    A: serde::Serialize + Send,
    R: serde::de::DeserializeOwned,
{
    ops::ensure_loaded(config, MODULE_ID)
        .await
        .map_err(VoiceCallError::Unavailable)?;
    let record = registry::find(MODULE_ID)
        .ok_or_else(|| VoiceCallError::Unavailable(format!("unknown module '{MODULE_ID}'")))?;
    let runtime = host::runtime()
        .await
        .map_err(|_| VoiceCallError::Unavailable("the module bus is not running".to_string()))?;
    let proxy = runtime
        .proxy(record.bus_name, record.object_path)
        .map_err(|error| VoiceCallError::Failed(error.to_string()))?;

    proxy
        .call(method, args)
        .await
        .map_err(|error| classify(&error))
}

/// Map a wire error onto the two outcomes a caller distinguishes.
fn classify(error: &tinybus::Error) -> VoiceCallError {
    let message = error.to_string();
    match error.wire_name() {
        // Loaded but not answering: refused, faulted, or gone.
        name if name.contains("ModuleUnavailable") => VoiceCallError::Unavailable(message),
        _ => VoiceCallError::Failed(message),
    }
}

#[cfg(test)]
#[path = "voice_tests.rs"]
mod tests;
