//! `rvoip_g729ab` is a pure Rust implementation of ITU-T G.729AB intended for
//! standalone use today and later integration into
//! [`rvoip`](https://github.com/eisenzopf/rvoip).
//!
//! The public API is centered around [`G729Encoder`], [`G729Decoder`], and the
//! small set of configuration and frame types re-exported from `api`.
//!
//! # Example
//!
//! ```
//! use rvoip_g729ab::{
//!     FrameType, G729Config, G729Decoder, G729Encoder, FRAME_SAMPLES, SPEECH_FRAME_BYTES,
//! };
//!
//! let cfg = G729Config { annex_b: false };
//! let mut encoder = G729Encoder::new(cfg);
//! let mut decoder = G729Decoder::new(cfg);
//!
//! let pcm_in = [0i16; FRAME_SAMPLES];
//! let mut bitstream = [0u8; SPEECH_FRAME_BYTES];
//! let frame_type = encoder.encode(&pcm_in, &mut bitstream);
//! assert_eq!(frame_type, FrameType::Speech);
//!
//! let mut pcm_out = [0i16; FRAME_SAMPLES];
//! decoder.decode(&bitstream, &mut pcm_out);
//! ```
//!
//! This repo also preserves the PRD, implementation plan, and specification
//! documents that shaped the project. Those documents are historical planning
//! artifacts; the hidden reference corpora and internal validation harnesses
//! they mention are intentionally not bundled in this extracted public repo.
#![cfg_attr(not(feature = "std"), no_std)]

/// Public API layer (`G729Encoder`, `G729Decoder`, configs, and frame types).
pub mod api;
/// Public bitstream utilities.
pub mod bitstream;
/// Public constants used by API consumers.
pub mod constants;
/// Public error type.
pub mod error;

/// Internal codec pipeline modules (kept public-for-testing but hidden from docs).
#[doc(hidden)]
pub mod codec;
/// Internal DSP helpers.
#[doc(hidden)]
pub mod dsp;
/// Internal filter helpers.
#[doc(hidden)]
pub mod filter;
/// Internal fixed codebook helpers.
#[doc(hidden)]
pub mod fixed_cb;
/// Internal gain helpers.
#[doc(hidden)]
pub mod gain;
/// Internal LP analysis helpers.
#[doc(hidden)]
pub mod lp;
/// Internal LSP quantization helpers.
#[doc(hidden)]
pub mod lsp_quant;
/// Internal pitch helpers.
#[doc(hidden)]
pub mod pitch;
/// Internal post-filter helpers.
#[doc(hidden)]
pub mod postfilter;
/// Internal post-processing helpers.
#[doc(hidden)]
pub mod postproc;
/// Internal pre-processing helpers.
#[doc(hidden)]
pub mod preproc;
/// Internal codec tables.
#[doc(hidden)]
pub mod tables;

/// Internal Annex B helpers.
#[cfg(feature = "annex_b")]
#[doc(hidden)]
pub mod annex_b;

/// Public encoder/decoder runtime configuration types.
pub use api::{DecoderConfig, EncoderConfig, FrameType, G729Config, G729Decoder, G729Encoder};
/// Public re-export.
pub use error::CodecError;
/// Backward-compatible alias.
pub type G729Error = CodecError;

/// Number of PCM samples per 10 ms frame.
pub const FRAME_SAMPLES: usize = 80;
/// Packed speech frame size in bytes.
pub const SPEECH_FRAME_BYTES: usize = 10;
/// Packed SID frame size in bytes.
pub const SID_FRAME_BYTES: usize = 2;

#[cfg(test)]
mod tests {
    use core::mem::size_of;

    use crate::codec::state::{DecoderState, EncoderState};
    use crate::{G729Decoder, G729Encoder};

    #[test]
    fn send_bounds_compile_for_public_types() {
        fn assert_send<T: Send>() {}
        assert_send::<G729Encoder>();
        assert_send::<G729Decoder>();
        assert_send::<EncoderState>();
        assert_send::<DecoderState>();
    }

    #[test]
    fn size_assertions_encoder_decoder_state() {
        assert!(size_of::<EncoderState>() < 8 * 1024);
        assert!(size_of::<DecoderState>() < 4 * 1024);
        assert!(size_of::<EncoderState>() + size_of::<DecoderState>() < 64 * 1024);
    }
}
