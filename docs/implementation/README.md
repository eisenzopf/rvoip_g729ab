# G.729AB Codec Implementation Plan (Pure Rust) — Final

> Historical note: the detailed phase docs below were written against the
> original `g729/` sub-crate workspace. In this extracted public repo the crate
> now lives at repository root as `rvoip_g729ab`.

> This document has been split from the original monolithic `IMPLEMENTATION_PLAN_final.md`. Phase details are in individual files linked below. Appendices (dependency graph, conformance, test infrastructure, CI, risks) are in [appendices.md](appendices.md).

## Phase Index

| Phase | Name | File |
|-------|------|------|
| 1 | DSP Math Kernel | [phase_01_dsp_math.md](phase_01_dsp_math.md) |
| 2 | Codec Constants and Tables | [phase_02_tables.md](phase_02_tables.md) |
| 3 | Common DSP Functions | [phase_03_common_dsp.md](phase_03_common_dsp.md) |
| 4 | Bitstream Pack/Unpack | [phase_04_bitstream.md](phase_04_bitstream.md) |
| 5 | Decoder (Core G.729A) | [phase_05_decoder.md](phase_05_decoder.md) |
| 6 | Encoder (Core G.729A) | [phase_06_encoder.md](phase_06_encoder.md) |
| 7 | Annex B — VAD | [phase_07_vad.md](phase_07_vad.md) |
| 8 | Annex B — DTX | [phase_08_dtx.md](phase_08_dtx.md) |
| 9 | Annex B — CNG | [phase_09_cng.md](phase_09_cng.md) |
| 10 | Public API, CLI, Hardening | [phase_10_api_cli.md](phase_10_api_cli.md) |

See also: [Appendices](appendices.md) (dependency graph, conformance checkpoints, test infrastructure, CI gates, risks, estimated totals)

## Context

This plan describes the implementation of an ITU-T G.729AB speech codec in pure Rust. G.729AB combines Annex A (reduced-complexity CS-ACELP at 8 kbit/s) with Annex B (VAD/DTX/CNG silence compression). The codec is intended for production SIP/VoIP platforms handling hundreds to thousands of concurrent connections. All patents expired in 2017, making the codec royalty-free.

The implementation must be:
- **Bit-exact** with ITU-T reference code (Annex A v1.1, Annex B v1.5)
- **`#![no_std]` compatible** — no heap allocation during encode/decode
- **< 64 KB memory** per encoder+decoder instance (~3.5 KB actual)
- **< 2 ms encode / < 1 ms decode** latency per 10 ms frame
- **Thread-safe** — `Send` but not `Sync` (one instance per call leg)
- **Rust stable toolchain only** — no nightly features (PRD §11.6)

Reference materials are at `/Users/jonathan/Developer/g729_reference/`:
- `PRD.md` — Full product requirements document
- `g729_math.md` — DSP math library specification
- `reference/itu_reference_code/g729ab_v14/` — ITU reference C (52 files)
- `reference/itu_reference_code/G729_Release3/` — ITU Release 3 with test vectors
- `reference/bcg729/` — Clean C99 open-source implementation (~8K lines)
- `reference/specs/` — ITU-T specification PDFs

---

## Crate Organization

Single crate with an optional binary target. The DSP math, tables, and codec logic are tightly coupled — splitting into a workspace would add cross-crate inlining friction for no modularity benefit. All hot-path DSP functions require `#[inline(always)]` and share types extensively; a multi-crate workspace would force either LTO or accept missed inlining across crate boundaries.

> **Note:** PRD §14 suggests a flat ~20-file layout. This plan deliberately uses a hierarchical ~99-file structure (~84 algorithm/data files + ~15 `mod.rs` re-export and infrastructure files) to enforce the 200 LOC file-length policy and improve navigability. The rationale is above; the single-crate constraint is preserved.

```toml
[package]
name = "g729"
version = "0.1.0"
edition = "2024"

[features]
default = ["annex_b"]
std = []                # std::error::Error impls, file I/O, CLI
annex_b = []            # VAD/DTX/CNG (Annex B support)
itu_serial = ["std"]    # ITU-T serial file format for test vectors

[[bin]]
name = "g729-cli"
path = "src/bin/cli.rs"
required-features = ["std"]
```

> **Edition note:** `edition = "2024"` requires Rust 1.85+ (stable since February 2025). CI should pin a minimum Rust version (MSRV) to ensure reproducible builds per PRD §11.6 (stable toolchain only).

---

## Architecture Strategy

### Layered Model

The crate uses six logical layers, all within a single crate:

1. **`dsp/` (math layer):** Fixed-point DSP kernel and overflow signaling
2. **`tables/` (tables layer):** Immutable ROM/codebook data shared by all channels
3. **`lp/`, `lsp_quant/`, `pitch/`, `fixed_cb/`, `gain/`, `filter/` (kernel layer):** Codec algorithm blocks
4. **`codec/` (pipeline layer):** Frame encoder/decoder orchestration and state machines
5. **`bitstream/` (io layer):** Bitstream pack/unpack (ITU serial + packed octet mode)
6. **`api/` (api layer):** Developer-facing safe types and ergonomic methods

### Concurrency and Scalability Model

1. One `G729Encoder` and/or `G729Decoder` instance per RTP stream
2. All mutable state is instance-local; shared data is read-only `const` arrays
3. Zero locks on hot path (frame encode/decode)
4. No allocations after `new()`
5. `Send` contexts; no `Sync` guarantee needed for mutable codec state
6. Overflow flag per-instance on `DspContext`, eliminating the global mutable `Overflow` of the C reference

### Memory and CPU Strategy

1. Static fixed-size arrays for all frame/state buffers
2. No `Vec` in core processing path
3. Reuse scratch buffers inside state structs
4. Keep frequently used constants in `const` arrays to improve cache locality
5. Use `#[inline(always)]` for verified hot primitives (math ops, tiny kernels)
6. Add optional SIMD feature after bit-exact baseline is locked

---

## Module Structure

```
src/
  lib.rs                          -- #![no_std], feature gates, public re-exports
  error.rs                        -- CodecError enum (~40 lines)

  api/
    mod.rs                        -- Public API re-exports
    encoder.rs                    -- G729Encoder struct + encode()
    decoder.rs                    -- G729Decoder struct + decode()
    config.rs                     -- EncoderConfig, DecoderConfig
    frame.rs                      -- FrameType enum, EncodeResult

  dsp/
    mod.rs                        -- DSP module root
    types.rs                      -- Word16, Word32, DspContext, Overflow
    arith.rs                      -- add, sub, mult, negate, abs_s (16-bit ops)
    arith32.rs                    -- L_add, L_sub, L_mult, L_mac, L_msu (32-bit ops)
    shift.rs                      -- shl, shr, L_shl, L_shr, norm_s, norm_l
    div.rs                        -- div_s, inv_sqrt, Log2, Pow2
    random.rs                     -- Random(): LCG pseudo-random generator
    oper32.rs                     -- L_Extract, L_Comp, Mpy_32, Mpy_32_16, Div_32

  tables/
    mod.rs                        -- Table module root
    window.rs                     -- Hamming window (240), lag window (M+2=12, DPF hi/lo)
    lsp.rs                        -- LSP codebooks (128x10 + 32x5 + 32x5), MA predictors
    gain.rs                       -- Gain codebooks (8x2 + 16x2), MA coefficients
    pitch.rs                      -- Sinc interpolation filter (31 coeff, FIR_SIZE_SYN)
    misc.rs                       -- Grid points (51), taming zones (153), cosine tables
    bitstream.rs                  -- Bit allocation table, frame size constants
    postfilter.rs                 -- Post-filter gamma tables
    vad.rs                        -- VAD threshold tables [cfg(annex_b)]
    sid.rs                        -- SID codebooks, noise_fg tables [cfg(annex_b)]

  lp/
    mod.rs                        -- LP analysis module root
    window.rs                     -- Apply asymmetric analysis window
    autocorr.rs                   -- Autocorrelation with overflow retry
    levinson.rs                   -- Levinson-Durbin recursion (DPF precision)
    az_lsp.rs                     -- LP to LSP (Chebyshev root finding)
    lsp_az.rs                     -- LSP to LP (polynomial reconstruction)
    lsf.rs                        -- Lsp_lsf2, Lsf_lsp2 (Q13 LSF<->LSP), Lsp_lsf (Q15 LSP->LSF)
    weight.rs                     -- Weight_Az bandwidth expansion
    interp.rs                     -- LSP interpolation

  lsp_quant/
    mod.rs                        -- LSP quantization module root
    encode.rs                     -- Qua_lsp: two-stage VQ with MA prediction
    decode.rs                     -- D_lsp / Lsp_iqua_cs: LSP decoding + stability
    helpers.rs                    -- Lsp_get_quant, Lsp_get_tdist, Lsp_pre_select
    prev.rs                       -- Lsp_prev_extract, Lsp_prev_compose, Lsp_prev_update
    stability.rs                  -- Lsp_stability, Lsp_expand_1, Lsp_expand_2, Lsp_expand_1_2

  pitch/
    mod.rs                        -- Pitch module root
    open_loop.rs                  -- Pitch_ol_fast: decimated open-loop search
    closed_loop.rs                -- Pitch_fr3_fast: closed-loop fractional search, G_pitch()
    lag_encode.rs                 -- Enc_lag3: pitch lag encoding
    lag_decode.rs                 -- Dec_lag3: pitch lag decoding
    parity.rs                     -- Parity_Pitch / Check_Parity_Pitch
    pred_lt3.rs                   -- Pred_lt_3: adaptive codebook vector

  fixed_cb/
    mod.rs                        -- Fixed codebook module root
    search.rs                     -- ACELP_Code_A + D4i40_17_fast + Cor_h (private): depth-first search
    correlation.rs                -- Cor_h_X: backward-filtered target correlation (shared with pitch/closed_loop.rs)
    decode.rs                     -- Decod_ACELP: position/sign decoding
    build_code.rs                 -- Build excitation vector from pulse positions

  gain/
    mod.rs                        -- Gain module root
    quantize.rs                   -- Qua_gain: joint pitch+code gain VQ
    decode.rs                     -- Dec_gain: gain decoding with MA prediction
    predict.rs                    -- Gain_predict, Gain_update
    taming.rs                     -- Init_exc_err, update_exc_err, test_err

  bitstream/
    mod.rs                        -- Bitstream module root
    pack.rs                       -- prm2bits_ld8k: parameters -> 80-bit frame
    unpack.rs                     -- bits2prm_ld8k: 80-bit frame -> parameters
    itu_serial.rs                 -- ITU serial format [cfg(itu_serial)]

  filter/
    mod.rs                        -- Filter module root
    syn.rs                        -- Syn_filt: LP synthesis (with overflow handling)
    resid.rs                      -- Residu: LP residual computation
    convolve.rs                   -- Convolve: impulse response convolution
    preemph.rs                    -- Pre-emphasis / de-emphasis

  preproc.rs                      -- Pre_Process: encoder HP filter (140 Hz)
  postproc.rs                     -- Post_Process: decoder HP filter + upscaling

  postfilter/
    mod.rs                        -- Post-filter module root
    pitch_pf.rs                   -- Pitch post-filter (integer delays, Annex A)
    formant.rs                    -- Formant post-filter A(z/gamma_n)/A(z/gamma_d)
    agc.rs                        -- Adaptive gain control
    pipeline.rs                   -- Orchestrate: pitch PF -> formant -> tilt -> AGC

  codec/
    mod.rs                        -- Codec engine module root
    encode.rs                     -- Per-frame encoder pipeline
    encode_sub.rs                 -- Per-subframe encoder logic
    decode.rs                     -- Per-frame decoder pipeline
    decode_sub.rs                 -- Per-subframe decoder logic
    erasure.rs                    -- Frame erasure concealment
    state/
      mod.rs                      -- State module root
      encoder_state.rs            -- EncoderState struct (~2.4 KB)
      decoder_state.rs            -- DecoderState struct (~1.1 KB)

  annex_b/                        -- #[cfg(feature = "annex_b")]
    mod.rs                        -- Annex B module root
    vad/
      mod.rs                      -- VAD module root
      detect.rs                   -- vad(): main decision function
      features.rs                 -- Energy, ZCR, spectral distance extraction
      decision.rs                 -- MakeDec(): 14 linear discriminants
      state.rs                    -- VadState struct
    dtx/
      mod.rs                      -- DTX module root
      encode.rs                   -- Cod_cng, Update_cng
      stationarity.rs             -- Itakura distance, filter averaging
      state.rs                    -- DtxState struct
    cng/
      mod.rs                      -- CNG module root
      decode.rs                   -- Dec_cng: comfort noise synthesis
      sid.rs                      -- SID frame encode/decode, lsfq_noise
      excitation.rs               -- Calc_exc_rand: random excitation
      state.rs                    -- CngDecState struct (encoder-side CNG state lives on DtxState and EncoderState)

  bin/
    cli.rs                        -- CLI: g729-cli encode/decode/test-vectors
```

**~99 files total (~84 algorithm/data files + ~15 `mod.rs` re-export and infrastructure files), averaging ~120 lines each. The per-phase estimated totals table below sums to ~84 files because `mod.rs` files and shared infrastructure (`lib.rs`, `error.rs`) are not counted in individual phases. Target: no file exceeds 200 lines.**

---

## File Length Policy

1. Keep algorithm blocks split by responsibility (e.g., `lp/autocorr.rs`, `lp/levinson.rs`)
2. Allow exceptions for generated/constant table files and test vector fixtures
3. Enforce with CI script:
   - Fail if non-generated `.rs` file > 220 LOC
   - Warn for 180–220 LOC
4. Generated table files and `const` data arrays are exempt

---

## Key Design Decisions

### DSP Types and Overflow Flag

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Word16(pub i16);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Word32(pub i32);

pub struct DspContext {
    pub overflow: bool,
}
```

- **No operator overloading** — explicit `add(a, b)`, `L_mac(acc, x, y)` calls signal DSP intent
- **Overflow flag per-instance** — lives on `DspContext`, passed as `&mut` to operations that can saturate
- **Three call sites read the overflow flag** (PRD §3.2.2, §3.7, §5.8.1):
  1. Autocorrelation retry (LPC.C:48-62) — right-shifts windowed signal until no overflow
  2. Pitch energy scaling (PITCH_A.C:55-69) — triggers signal rescaling by >>3
  3. Decoder synthesis (DEC_LD8A.C:169-181,331-344) — triggers excitation >>2 and re-synthesis

### State Structs

All C global/static variables become fields on `EncoderState` or `DecoderState`. Memory budget:

| Component | Bytes |
|-----------|-------|
| Encoder (core + buffers + filter memories) | ~1,600 |
| Encoder (LSP/gain prediction, taming) | ~300 |
| Encoder (VAD + DTX) | ~460 |
| **Encoder total** | **~2,360** |
| Decoder (core + excitation + synthesis) | ~560 |
| Decoder (post-filter + post-process) | ~460 |
| Decoder (CNG + erasure state) | ~120 |
| **Decoder total** | **~1,140** |
| **Combined per call leg** | **~3,500** |

For 1,000 concurrent full-duplex channels: ~3.5 MB of instance state + ~8 KB shared ROM tables. Well under the PRD's 64 KB per-instance limit.

### Public API

```rust
pub struct G729Encoder { state: EncoderState }
pub struct G729Decoder { state: DecoderState }

impl G729Encoder {
    pub fn new(config: EncoderConfig) -> Self;
    pub fn encode(&mut self, pcm: &[i16; 80], output: &mut [u8; 10]) -> FrameType;
    pub fn encode_frame(&mut self, pcm: &[i16]) -> Result<[u8; FRAME_BYTES], CodecError>;
    pub fn reset(&mut self);
}

impl G729Decoder {
    pub fn new(config: DecoderConfig) -> Self;
    pub fn decode(&mut self, bitstream: &[u8], output: &mut [i16; 80]);
    pub fn decode_with_type(&mut self, bitstream: &[u8], frame_type: FrameType, output: &mut [i16; 80]);
    pub fn decode_frame(&mut self, data: &[u8]) -> Result<[i16; FRAME_SAMPLES], CodecError>;
    pub fn decode_erasure(&mut self, output: &mut [i16; 80]);
    pub fn reset(&mut self);
}

pub struct EncoderConfig {
    pub annex_b: bool,      // Enable VAD/DTX/CNG (default: true)
}

pub struct DecoderConfig {
    pub annex_b: bool,      // Enable CNG (default: true)
    pub post_filter: bool,  // Enable adaptive post-filter (default: true)
}

pub enum FrameType { Speech, Sid, NoData }
```

The buffer-based methods (`encode`, `decode`) are the zero-allocation hot path for production use. The `Result`-returning methods (`encode_frame`, `decode_frame`) accept slices and perform validation, returning errors for invalid input lengths. `decode()` infers frame type from bitstream length (10→Speech, 2→SID, 0→NoData) and accepts BFI via the transport layer or a separate method. `decode_with_type()` accepts an explicit `FrameType` parameter for transport layers that provide frame type metadata alongside the data (e.g., the ITU reference code's `read_frame()` which produces both bitstream data and a frame type field); it takes precedence over length-based inference when the two disagree. Frame erasure is handled via `decode_erasure()`.

---

## Reuse vs Build Decision

### Reuse (external crates)

| Crate | Purpose | Justification |
|-------|---------|---------------|
| `criterion` | Benchmarks | Standard Rust benchmark framework |
| `proptest` | Property-based tests | Saturation commutativity, range preservation |
| `arbitrary` + `cargo-fuzz` | Robustness fuzzing | Decoder/encoder panic-free guarantees |

### Build In-House

| Component | Reason |
|-----------|--------|
| All codec algorithm code | Core deliverable |
| Fixed-point DSP kernel (`dsp/`) | Must match ITU behavior exactly |
| Bitstream pack/unpack | Simple, domain-specific |
| Table loading | One-time transcription, no codegen framework needed |

### Explicit Non-Reuse

| Item | Reason |
|------|--------|
| `libm` | Not needed — codec is 100% fixed-point, no floating-point math |
| `thiserror` | Overkill for a 4-variant error enum; manual `Display` + `Error` impl suffices |
| `bcg729` source | GPL licensing mismatch; reference-only |
| `xtask` for table codegen | Over-engineering for a one-time transcription task |
| RTP/SDP helpers | PRD §1.4 explicitly excludes RTP from core |

---
