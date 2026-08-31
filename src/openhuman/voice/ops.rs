//! Voice domain business logic — hosted STT and local piper TTS.
//!
//! Each public function follows the `RpcOutcome<T>` pattern used by other
//! domain modules (billing, health, etc.).

use chrono::Utc;
use log::{debug, warn};
use std::time::Instant;

use crate::openhuman::config::Config;
use crate::openhuman::inference::local as local_ai;
use crate::openhuman::inference::local::model_ids;
use crate::openhuman::inference::local::paths::{resolve_piper_binary, resolve_tts_voice_path};
use crate::rpc::RpcOutcome;

use super::factory::{create_stt_provider, effective_stt_provider};
use super::postprocess;
use super::types::{VoiceSpeechResult, VoiceStatus, VoiceTtsResult};
use crate::openhuman::modules::voice::{is_hallucinated, HallucinationMode};

const LOG_PREFIX: &str = "[voice]";

/// Check availability of the STT engine and the TTS binary/model without
/// executing them.
pub async fn voice_status(config: &Config) -> Result<RpcOutcome<VoiceStatus>, String> {
    debug!("{LOG_PREFIX} checking voice status");

    let piper_bin = resolve_piper_binary();
    let tts_voice = resolve_tts_voice_path(config).ok();

    // STT is hosted now, so "available" means the configured engine actually
    // resolves to a provider: the backend proxy always does, a third-party slug
    // only when its `voice_providers` entry exists. Constructing the provider is
    // the same check the transcribe path performs, and it makes no network call
    // — so this reports the real failure a user would hit, not a guess.
    let stt_engine = effective_stt_provider(config);
    let stt_error = create_stt_provider(&stt_engine, "", config)
        .and_then(|_| {
            let slug = stt_engine
                .split_once(':')
                .map_or(stt_engine.as_str(), |(slug, _)| slug);
            if matches!(slug.trim(), "cloud" | "openhuman" | "backend") {
                return Ok(());
            }
            let key = crate::openhuman::inference::provider::factory::lookup_key_for_slug(
                slug.trim(),
                config,
            )?;
            if key.trim().is_empty() {
                anyhow::bail!("voice provider '{slug}' has no API credential configured")
            }
            Ok(())
        })
        .err()
        .map(|e| e.to_string());
    let stt_available = stt_error.is_none();
    let tts_available = piper_bin.is_some() && tts_voice.is_some();

    debug!(
        "{LOG_PREFIX} stt_available={stt_available} stt_engine={stt_engine} \
         tts_available={tts_available} piper_bin={} tts_voice={} stt_error={:?}",
        safe_basename_path(&piper_bin),
        safe_basename_str(&tts_voice),
        stt_error,
    );

    let tts_provider = if config.local_ai.tts_provider.trim().is_empty() {
        "cloud".to_string()
    } else {
        config.local_ai.tts_provider.clone()
    };

    let status = VoiceStatus {
        stt_available,
        tts_available,
        stt_model_id: model_ids::effective_stt_model_id(config),
        tts_voice_id: model_ids::effective_tts_voice_id(config),
        piper_binary: piper_bin.map(|p| p.display().to_string()),
        tts_voice_path: tts_voice,
        llm_cleanup_enabled: config.local_ai.voice_llm_cleanup_enabled,
        stt_engine,
        stt_error,
        tts_provider,
    };

    Ok(RpcOutcome::single_log(status, "voice status checked"))
}

/// Transcribe audio from a file path through the configured STT engine.
///
/// If `context` is provided, the raw transcription is post-processed through
/// a local LLM to fix grammar and disambiguate words using conversation history.
pub async fn voice_transcribe(
    config: &Config,
    audio_path: &str,
    context: Option<&str>,
    skip_cleanup: bool,
) -> Result<RpcOutcome<VoiceSpeechResult>, String> {
    let started = Instant::now();
    debug!("{LOG_PREFIX} transcribing audio_path={audio_path}");

    let service = local_ai::global(config);
    let transcribe_started = Instant::now();
    // Context is forwarded as a vocabulary-bias hint where the engine has one.
    let output = service
        .transcribe_with_prompt(config, audio_path.trim(), context)
        .await
        .map_err(|e| e.to_string())?;
    let transcribe_elapsed = transcribe_started.elapsed();

    let raw_text = output.text.clone();
    debug!(
        "{LOG_PREFIX} transcription completed, text length={}, stt_elapsed_ms={}",
        raw_text.len(),
        transcribe_elapsed.as_millis()
    );

    let cleanup_started = Instant::now();
    let text = if skip_cleanup {
        raw_text.clone()
    } else {
        postprocess::cleanup_transcription(config, &raw_text, context).await
    };
    let cleanup_elapsed = cleanup_started.elapsed();
    debug!(
        "{LOG_PREFIX} voice_transcribe complete (cleanup_elapsed_ms={}, total_elapsed_ms={})",
        cleanup_elapsed.as_millis(),
        started.elapsed().as_millis()
    );

    Ok(RpcOutcome::single_log(
        VoiceSpeechResult {
            text,
            raw_text,
            model_id: output.model_id,
        },
        "voice transcription completed",
    ))
}

/// Transcribe audio from raw bytes. Writes to a temp file, transcribes, cleans up.
///
/// If `context` is provided, the raw transcription is post-processed through
/// a local LLM.
pub async fn voice_transcribe_bytes(
    config: &Config,
    audio_bytes: &[u8],
    extension: Option<String>,
    context: Option<&str>,
    skip_cleanup: bool,
) -> Result<RpcOutcome<VoiceSpeechResult>, String> {
    let started = Instant::now();
    let ext = normalize_extension(extension)?;
    debug!(
        "{LOG_PREFIX} transcribe_bytes size={} ext={ext}",
        audio_bytes.len()
    );

    let service = local_ai::global(config);

    let voice_dir = std::env::temp_dir().join("openhuman_voice_input");
    tokio::fs::create_dir_all(&voice_dir)
        .await
        .map_err(|e| format!("failed to create voice input directory: {e}"))?;

    let filename = format!(
        "voice-{}-{}.{}",
        Utc::now().timestamp_millis(),
        uuid::Uuid::new_v4(),
        ext
    );
    let file_path = voice_dir.join(filename);
    let write_started = Instant::now();
    tokio::fs::write(&file_path, audio_bytes)
        .await
        .map_err(|e| format!("failed to write audio file: {e}"))?;
    let write_elapsed = write_started.elapsed();

    let transcribe_started = Instant::now();
    // Context is forwarded as a vocabulary-bias hint where the engine has one.
    let output = service
        .transcribe_with_prompt(config, file_path.to_string_lossy().as_ref(), context)
        .await;
    let transcribe_elapsed = transcribe_started.elapsed();
    if let Err(e) = tokio::fs::remove_file(&file_path).await {
        warn!(
            "{LOG_PREFIX} failed to clean up temp audio file {}: {e}",
            file_path.display()
        );
    }

    let output = output.map_err(|e| e.to_string())?;
    let raw_text = output.text.clone();

    debug!(
        "{LOG_PREFIX} transcribe_bytes completed, text length={}, write_elapsed_ms={}, stt_elapsed_ms={}",
        raw_text.len(),
        write_elapsed.as_millis(),
        transcribe_elapsed.as_millis()
    );

    // Filter hallucinated output before spending time on LLM cleanup.
    //
    // Falls OPEN when the module cannot be reached: passing a stock phrase
    // through costs the user one bad transcription they can see and redo,
    // whereas defaulting to "hallucinated" would silently delete real speech.
    // Only one of those is recoverable.
    let hallucinated = match is_hallucinated(config, &raw_text, HallucinationMode::Conversation)
        .await
    {
        Ok(verdict) => verdict,
        Err(error) => {
            warn!("{LOG_PREFIX} hallucination filter unavailable ({error}); passing text through");
            false
        }
    };
    if hallucinated {
        debug!("{LOG_PREFIX} transcribe_bytes: hallucination detected, returning empty result");
        return Ok(RpcOutcome::single_log(
            VoiceSpeechResult {
                text: String::new(),
                raw_text,
                model_id: output.model_id,
            },
            "voice transcription filtered (hallucination)",
        ));
    }

    let cleanup_started = Instant::now();
    let text = if skip_cleanup {
        raw_text.clone()
    } else {
        postprocess::cleanup_transcription(config, &raw_text, context).await
    };
    let cleanup_elapsed = cleanup_started.elapsed();
    debug!(
        "{LOG_PREFIX} transcribe_bytes pipeline complete (cleanup_elapsed_ms={}, total_elapsed_ms={})",
        cleanup_elapsed.as_millis(),
        started.elapsed().as_millis()
    );

    Ok(RpcOutcome::single_log(
        VoiceSpeechResult {
            text,
            raw_text,
            model_id: output.model_id,
        },
        "voice transcription completed",
    ))
}

/// Synthesize speech from text using piper.
pub async fn voice_tts(
    config: &Config,
    text: &str,
    output_path: Option<&str>,
) -> Result<RpcOutcome<VoiceTtsResult>, String> {
    debug!(
        "{LOG_PREFIX} tts text_length={} output_path={:?}",
        text.len(),
        output_path
    );

    let service = local_ai::global(config);
    let output = service
        .tts(config, text.trim(), output_path)
        .await
        .map_err(|e| e.to_string())?;

    debug!("{LOG_PREFIX} tts completed, output={}", output.output_path);

    Ok(RpcOutcome::single_log(
        VoiceTtsResult::from(output),
        "voice tts completed",
    ))
}

/// Normalize an optional audio file extension. Returns a clean lowercase
/// alphanumeric extension string, defaulting to "webm".
pub(crate) fn normalize_extension(ext: Option<String>) -> Result<String, String> {
    let normalized = ext
        .unwrap_or_else(|| "webm".to_string())
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase();

    if normalized.is_empty() {
        return Err("audio extension must not be empty".to_string());
    }
    if !normalized.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(format!(
            "invalid audio extension '{normalized}': must be alphanumeric"
        ));
    }

    Ok(normalized)
}

/// Extract the file name from an `Option<PathBuf>`, returning `"<none>"` if absent.
fn safe_basename_path(p: &Option<std::path::PathBuf>) -> String {
    p.as_ref()
        .and_then(|pb| pb.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("<none>")
        .to_string()
}

/// Extract the file name from an `Option<String>` path, returning `"<none>"` if absent.
fn safe_basename_str(p: &Option<String>) -> String {
    p.as_ref()
        .and_then(|s| std::path::Path::new(s).file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("<none>")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_extension_defaults_to_webm() {
        assert_eq!(normalize_extension(None).unwrap(), "webm");
    }

    #[test]
    fn normalize_extension_strips_dot_and_lowercases() {
        assert_eq!(
            normalize_extension(Some(".WebM".to_string())).unwrap(),
            "webm"
        );
        assert_eq!(normalize_extension(Some("OGG".to_string())).unwrap(), "ogg");
        assert_eq!(
            normalize_extension(Some("  .WAV  ".to_string())).unwrap(),
            "wav"
        );
    }

    #[test]
    fn normalize_extension_accepts_alphanumeric() {
        assert_eq!(normalize_extension(Some("m4a".to_string())).unwrap(), "m4a");
        assert_eq!(normalize_extension(Some("mp3".to_string())).unwrap(), "mp3");
    }

    #[test]
    fn normalize_extension_rejects_empty() {
        assert!(normalize_extension(Some("".to_string())).is_err());
        assert!(normalize_extension(Some("  ".to_string())).is_err());
        assert!(normalize_extension(Some(".".to_string())).is_err());
    }

    #[test]
    fn normalize_extension_rejects_invalid_chars() {
        assert!(normalize_extension(Some("a/b".to_string())).is_err());
        assert!(normalize_extension(Some("web m".to_string())).is_err());
        assert!(normalize_extension(Some("a.b".to_string())).is_err());
    }

    #[tokio::test]
    async fn voice_status_returns_without_error() {
        let config = Config::default();
        let result = voice_status(&config).await;
        assert!(result.is_ok());
        let status = result.unwrap().value;
        assert!(!status.stt_model_id.is_empty());
        assert!(!status.tts_voice_id.is_empty());
    }

    /// RAII guard that restores an env var on drop, even on panic.
    struct EnvGuard {
        key: &'static str,
        prev: Option<std::ffi::OsString>,
    }
    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let prev = std::env::var_os(key);
            std::env::set_var(key, value);
            Self { key, prev }
        }
    }
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => std::env::set_var(self.key, v),
                None => std::env::remove_var(self.key),
            }
        }
    }

    #[tokio::test]
    async fn voice_status_detects_stub_binaries() {
        let tmp = tempfile::tempdir().expect("tempdir");

        let piper_stub = tmp.path().join("piper");
        std::fs::write(&piper_stub, b"#!/bin/sh\n").expect("write stub");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&piper_stub, std::fs::Permissions::from_mode(0o755))
                .expect("chmod");
        }

        let _guard = EnvGuard::set("PIPER_BIN", &piper_stub.display().to_string());

        let mut config = Config::default();
        config.workspace_dir = tmp.path().join("workspace");
        config.config_path = tmp.path().join("config.toml");

        let result = voice_status(&config).await.unwrap();
        assert!(result.value.piper_binary.is_some());
    }

    /// STT is hosted, so status must report it available on a default config
    /// with no local binaries or models anywhere — the state that used to mean
    /// "STT unavailable" back when a whisper.cpp install was required.
    #[tokio::test]
    async fn voice_status_reports_backend_stt_available_without_any_local_install() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut config = Config::default();
        config.workspace_dir = tmp.path().join("workspace");
        config.config_path = tmp.path().join("config.toml");

        let result = voice_status(&config).await.unwrap();
        assert!(
            result.value.stt_available,
            "backend engine is always routable"
        );
        assert_eq!(result.value.stt_engine, "cloud");
        assert!(result.value.stt_error.is_none());
    }

    /// Selecting a third-party engine with no matching `voice_providers` entry
    /// must surface as unavailable-with-a-reason rather than silently falling
    /// back to the backend proxy and billing the wrong account.
    #[tokio::test]
    async fn voice_status_reports_unconfigured_engine_as_unavailable() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut config = Config::default();
        config.workspace_dir = tmp.path().join("workspace");
        config.config_path = tmp.path().join("config.toml");
        config.voice_server.stt_engine = crate::openhuman::config::schema::SttEngine::Elevenlabs;

        let result = voice_status(&config).await.unwrap();
        assert!(!result.value.stt_available);
        assert_eq!(result.value.stt_engine, "elevenlabs");
        let err = result.value.stt_error.expect("reason must be reported");
        assert!(err.contains("elevenlabs"), "reason names the slug: {err}");
    }

    #[tokio::test]
    async fn voice_status_reports_external_engine_without_credentials_as_unavailable() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut config = Config::default();
        config.workspace_dir = tmp.path().join("workspace");
        config.config_path = tmp.path().join("config.toml");
        config.voice_server.stt_engine = crate::openhuman::config::schema::SttEngine::Elevenlabs;
        config.voice_providers.push(
            crate::openhuman::config::schema::voice_providers::VoiceProviderCreds {
                slug: "elevenlabs".into(),
                endpoint: "https://api.elevenlabs.io/v1".into(),
                capability:
                    crate::openhuman::config::schema::voice_providers::VoiceCapability::Both,
                ..Default::default()
            },
        );

        let status = voice_status(&config).await.unwrap().value;
        assert!(!status.stt_available);
        assert!(status
            .stt_error
            .as_deref()
            .is_some_and(|error| error.contains("no API credential")));
    }

    #[test]
    fn safe_basename_helpers_cover_missing_and_present_values() {
        assert_eq!(safe_basename_path(&None), "<none>");
        assert_eq!(safe_basename_str(&None), "<none>");

        let path = Some(std::path::PathBuf::from("/tmp/models/voice.bin"));
        let string = Some("/tmp/models/voice.bin".to_string());
        assert_eq!(safe_basename_path(&path), "voice.bin");
        assert_eq!(safe_basename_str(&string), "voice.bin");
    }

    #[tokio::test]
    /// Transcription no longer depends on the local-AI runtime — STT is a
    /// hosted call, so a workspace with `local_ai.runtime_enabled = false` must
    /// still reach the engine. The only failure left on this path is the audio
    /// file itself, and it must name the file rather than blaming local AI.
    async fn voice_transcribe_errors_on_unreadable_audio_not_on_disabled_local_ai() {
        let mut config = Config::default();
        config.local_ai.runtime_enabled = false;
        let missing = std::env::temp_dir().join("openhuman-no-such-input.wav");
        let _ = std::fs::remove_file(&missing);

        let err = voice_transcribe(&config, &format!(" {} ", missing.display()), None, true)
            .await
            .expect_err("a missing audio file must fail");
        assert!(
            err.contains("failed to read audio file"),
            "error should name the unreadable file, got: {err}"
        );
        assert!(
            !err.contains("local ai is disabled"),
            "hosted STT must not be gated on the local-AI runtime: {err}"
        );
    }

    #[tokio::test]
    /// Same contract for the bytes entry point: with local AI off it still
    /// reaches the hosted engine, so the failure comes from the upload (no
    /// signed-in session in a test workspace), never from a local-AI gate.
    async fn voice_transcribe_bytes_is_not_gated_on_the_local_ai_runtime() {
        let mut config = Config::default();
        config.local_ai.runtime_enabled = false;

        let err = voice_transcribe_bytes(&config, b"abc", Some("wav".to_string()), None, true)
            .await
            .expect_err("no signed-in session means the upload cannot happen");
        assert!(
            !err.contains("local ai is disabled"),
            "hosted STT must not be gated on the local-AI runtime: {err}"
        );
    }

    #[tokio::test]
    async fn voice_tts_errors_when_local_ai_disabled() {
        let mut config = Config::default();
        config.local_ai.runtime_enabled = false;

        let err = voice_tts(&config, "hello world", None)
            .await
            .expect_err("disabled local ai should fail");
        assert!(err.contains("local ai is disabled"));
    }
}
