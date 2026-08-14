//! Minimal, dependency-free RIFF/WAVE writer for 16-bit PCM.
//!
//! The dictation WebSocket receives raw PCM16LE frames but every hosted STT
//! endpoint wants a container. Wrapping the buffer in a 44-byte WAV header is
//! the whole job, so this is 40 lines of byte pushing rather than a dependency:
//! `hound` lives behind the `voice` feature gate and `inference::voice` is
//! always compiled, so reaching for it here would drag a gated crate into the
//! ungated half of the tree. (`meet::agent::wav` is the same trick behind the
//! `meet` gate — kept separate because a shared helper would have to live in a
//! third always-on module for no benefit.)

/// Wrap 16-bit little-endian PCM samples in a canonical 44-byte WAV header.
///
/// `sample_rate` is in Hz and `channels` is the interleaved channel count.
/// Returns the complete file bytes, ready to upload.
pub fn pcm16_to_wav(samples: &[i16], sample_rate: u32, channels: u16) -> Vec<u8> {
    let bits_per_sample: u16 = 16;
    let block_align = channels * bits_per_sample / 8;
    let byte_rate = sample_rate * u32::from(block_align);
    let data_len = (samples.len() * 2) as u32;

    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    // RIFF chunk size = 36 + data (everything after this field).
    out.extend_from_slice(&(36u32.saturating_add(data_len)).to_le_bytes());
    out.extend_from_slice(b"WAVE");

    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // PCM fmt chunk size
    out.extend_from_slice(&1u16.to_le_bytes()); // audio format = PCM
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&bits_per_sample.to_le_bytes());

    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_canonical_44_byte_header() {
        let wav = pcm16_to_wav(&[1, -1, 32767], 16_000, 1);
        assert_eq!(wav.len(), 44 + 6, "header + 3 samples × 2 bytes");
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(&wav[36..40], b"data");
        // RIFF size = 36 + data bytes.
        assert_eq!(u32::from_le_bytes(wav[4..8].try_into().unwrap()), 42);
        assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 6);
        // Sample rate and byte rate round-trip for 16 kHz mono.
        assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), 16_000);
        assert_eq!(u32::from_le_bytes(wav[28..32].try_into().unwrap()), 32_000);
        // Samples are little-endian.
        assert_eq!(&wav[44..46], &1i16.to_le_bytes());
    }

    #[test]
    fn empty_input_still_produces_a_valid_header() {
        let wav = pcm16_to_wav(&[], 16_000, 1);
        assert_eq!(wav.len(), 44);
        assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 0);
    }
}
