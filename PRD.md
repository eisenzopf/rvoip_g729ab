# G.729 Codec Implementation - Product Requirements Document

> Preserved planning artifact from the original development workspace. Any
> references to private `reference/` material or workspace-specific paths are
> historical provenance, not a promise that those assets ship in this extracted
> public repo.

## Local Reference Materials

All reference materials are consolidated in `g729_reference/reference/`:

**Specification PDFs** (`reference/specs/`):
- `T-REC-G.729-201206-I.epub` - Full ITU-T G.729 Recommendation (2012 edition, 9.1 MB)
- `T-REC-G.Imp729-201710-I.pdf` - Implementers' Guide for G.729 (2017)
- `g729.pdf` - Main G.729 CS-ACELP specification (CMU mirror)
- `g729a.pdf` - Annex A reduced-complexity specification (UCLA mirror)
- `g729anxa.pdf` - Original ITU Annex A document
- `g729anxb.pdf` - Original ITU Annex B document
- `G729E.pdf` - Annex E specification
- `g729-2012-full.pdf` - 2012 consolidated revision summary
- `g729ab-implementation-AN2261.pdf` - NXP Annex B implementation guide (AN2261)

**ITU-T Reference C Source Code** (`reference/itu_reference_code/`):
- `G729_Release3/` - Official 2012 Release 3 (all annexes A-I with test vectors)
  - Contains subdirectories for each annex, each with `c_code/` and `test_vectors/`
- `g729ab_v14/` - Standalone Annex B v14 AB-combined code
- `g729b_v14/` - Standalone Annex B v14 B-only code
- `g729_annex_b_test_vectors/` - Standalone Annex B v14 test vectors

**bcg729 Open-Source Implementation** (`reference/bcg729/`):
- Clean C99 implementation of G.729A/B (~8K lines)
- Well-structured with descriptive file names
- Source: https://github.com/BelledonneCommunications/bcg729

**Community Implementations** (`reference/community/`):
- `doubango-g729/` - DoubangoTelecom fork with autotools build system and ARM64 fixes

**Key Reference Files for Implementation:**
- Constants/prototypes: `reference/itu_reference_code/G729_Release3/g729AnnexA/c_code/LD8A.H`
- All codebook tables: `reference/itu_reference_code/G729_Release3/g729AnnexA/c_code/TAB_LD8A.C`
- Basic operators: `reference/itu_reference_code/G729_Release3/g729AnnexA/c_code/BASIC_OP.C`
- Annex B VAD/DTX/CNG: `reference/itu_reference_code/G729_Release3/g729AnnexB/c_codeBA/vad.c`, `dtx.c`, `dec_sid.c`

---

# PRD: ITU-T G.729AB (CS-ACELP) Encoder/Decoder in Rust

## 1. Overview

### 1.1 Purpose

This document specifies the complete requirements for implementing the ITU-T G.729AB speech codec in the Rust programming language. G.729AB refers to the combination of G.729 Annex A (reduced-complexity CS-ACELP) and Annex B (VAD/DTX/CNG silence compression). This is a **direct G.729AB implementation** — the Annex A algorithms are the primary and only encode/decode path; base G.729 (full-complexity) is not implemented. The implementation will provide a production-quality encoder and decoder suitable for integration into a SIP (Session Initiation Protocol) VoIP platform.

### 1.2 Codec Identity

| Property | Value |
|----------|-------|
| Standard | ITU-T Recommendation G.729 (06/2012) |
| Algorithm | CS-ACELP (Conjugate-Structure Algebraic-Code-Excited Linear Prediction) |
| Bit rate | 8 kbit/s |
| Sampling rate | 8000 Hz |
| Sample format | 16-bit linear PCM (signed) |
| Frame duration | 10 ms |
| Frame size | 80 samples |
| Subframes per frame | 2 (each 40 samples, 5 ms) |
| Bits per frame | 80 (10 octets) |
| Algorithmic delay | 15 ms (10 ms frame + 5 ms look-ahead) |
| LP filter order | 10 |
| Compression ratio | 16:1 vs G.711 (64 kbit/s) |
| MOS (clean speech) | ~3.7 (Annex A) |
| RTP payload type | 18 (static) |
| MIME type | audio/G729 |
| Patent status | Expired worldwide (royalty-free since 2017) |

### 1.3 Scope

This is a **G.729AB-only** implementation. The encoder and decoder use the Annex A (reduced-complexity) algorithms exclusively. There is no base G.729 (full-complexity) code path.

The implementation SHALL cover:

1. **G.729 Annex A encoder/decoder** - Reduced-complexity 8 kbit/s CS-ACELP (the sole encode/decode path)
2. **G.729 Annex B** - VAD/DTX/CNG silence compression (integrated with Annex A)
3. **Frame erasure concealment** - Packet loss handling (mandatory for SIP)
4. **Conformance testing** - Bit-exact verification against ITU-T test vectors

The bitstream format is identical to base G.729, so output is fully interoperable with any compliant G.729 or G.729A decoder.

### 1.4 Non-Goals

- **No base G.729 (full-complexity)** - The full-complexity encoder (nested-loop codebook search, adaptive weighting filter, fractional pitch post-filter) is not implemented. Annex A is the only code path.
- **No Annex D/E/H** - Multi-rate extensions (6.4 kbit/s, 11.8 kbit/s) are out of scope.
- **No G.729.1 / Annex J** - Wideband extensions are out of scope.
- **No proprietary enhancements** - Non-ITU variations or vendor-specific extensions are not included.
- **No RTP in core** - Real-time packetization or RTP payload formatting is not part of the core library (can be added as a separate layer).

---

## 2. Architecture

### 2.1 High-Level Block Diagram

```
ENCODER:
  PCM Input (8kHz, 16-bit)
    -> Pre-processing (HP filter + scaling)
    -> LP Analysis (autocorrelation, Levinson-Durbin)
    -> LP-to-LSP Conversion (Chebyshev polynomials)
    -> LSP Quantization (two-stage VQ with MA prediction)
    -> LP Interpolation (per subframe)
    -> Perceptual Weighting Filter
    -> Open-Loop Pitch Analysis
    -> [Per Subframe]:
        -> Target Signal Computation
        -> Adaptive Codebook Search (closed-loop pitch)
        -> Fixed Codebook Search (algebraic ISPP)
        -> Gain Quantization (joint VQ)
        -> Memory Update
    -> Bitstream Packing
  -> 80-bit Frame Output

DECODER:
  80-bit Frame Input
    -> Bitstream Unpacking
    -> LSP Decoding + Interpolation
    -> LSP-to-LP Conversion
    -> [Per Subframe]:
        -> Adaptive Codebook Reconstruction
        -> Fixed Codebook Reconstruction
        -> Gain Decoding
        -> Excitation Construction
        -> LP Synthesis Filtering
    -> Post-Processing:
        -> Long-term (pitch) post-filter
        -> Short-term (formant) post-filter
        -> Tilt compensation
        -> Adaptive gain control
        -> High-pass filter + upscaling
  -> PCM Output (8kHz, 16-bit)
```

### 2.2 State Management

The encoder and decoder each maintain persistent state across frames:

**Encoder state:**
- Pre-processing filter memory (2 input, 2 output samples; DPF split y1_hi/y1_lo/y2_hi/y2_lo)
- LP analysis window buffer (240 samples)
- Previous unquantized LP coefficients (10 values)
- Previous quantized LSP coefficients (10 values)
- LSP MA prediction memory (4 frames x 10 values)
- Levinson-Durbin fallback state: old_A (M+1 values), old_rc (2 values)
- Perceptual weighting filter memory: mem_w (M values), mem_w0 (M values), mem_zero (M values)
- Weighted speech buffer (for open-loop pitch)
- Excitation buffer (past excitation, length >= PIT_MAX + L_INTERPOL)
- Synthesis filter memory (10 values)
- Previous adaptive codebook gain (for gain prediction)
- Gain MA prediction memory (4 values)
- Previous pitch delay (frame-local: SF1's T0 serves as SF2's search center within the same frame; NOT persistent cross-frame state — see Errata E28)
- Taming state (accumulated energy)
- Target signal computation memory: mem_w0 (M values; updated per subframe from error signal e(n) = x(n) - gp*y1(n) - gc*y2(n))
- VAD history: pastVad, ppastVad (for DTX state machine, Annex B)

**Decoder state:**
- Previous quantized LSP coefficients (10 values)
- LSP MA prediction memory (4 frames x 10 values)
- Previous MA predictor mode: prev_ma (for frame erasure LSP recovery)
- Excitation buffer (past excitation, length >= PIT_MAX + L_INTERPOL)
- Synthesis filter memory (10 values)
- Previous adaptive codebook gain
- Gain MA prediction memory (4 values)
- Post-filter state: res2_buf (PIT_MAX+L_SUBFR), scal_res2_buf (PIT_MAX+L_SUBFR), mem_syn_pst (M values), past_gain AGC (Q12)
- High-pass filter memory (2 input, 2 output samples; DPF split y1_hi/y1_lo/y2_hi/y2_lo)
- Previous pitch delay
- Frame erasure history (previous BFI, voicing classification)

#### 2.2.1 Initialization Values

All state variables MUST be initialized to these exact values for bit-exactness:

| Variable | Initial Value | Format | Scope |
|---|---|---|---|
| lsp_old | {30000, 26000, 21000, 15000, 8000, 0, -8000, -15000, -21000, -26000} | Q15 | Encoder + Decoder |
| lsp_old_q | {30000, 26000, 21000, 15000, 8000, 0, -8000, -15000, -21000, -26000} | Q15 | Encoder |
| sharp | 3277 (SHARPMIN = 0.2) | Q14 | Encoder + Decoder |
| old_T0 | 60 | integer | Decoder |
| gain_pitch | 0 | Q14 | Decoder |
| gain_code | 0 | Q1 | Decoder |
| past_qua_en | {-14336, -14336, -14336, -14336} | Q10 | Encoder + Decoder |
| freq_prev[0..3] | each row = {2339, 4679, 7018, 9358, 11698, 14037, 16377, 18717, 21056, 23396} | Q13 | Encoder + Decoder |
| random seed (decoder FEC) | 21845 | Q0 | Decoder |
| past_ftyp | 1 (speech) | integer | Decoder |
| bad_lsf | 0 | flag | Decoder |
| seed (CNG decoder) | 11111 (INIT_SEED) | Q0 | Decoder |
| seed (CNG encoder) | 11111 (INIT_SEED) | Q0 | Encoder (CNG excitation; `Init_Coder_ld8a()`) |
| old_A | {4096, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0} | Q12 | Encoder (Levinson fallback) |
| old_rc | {0, 0} | Q15 | Encoder (Levinson fallback) |
| prev_ma | 0 | integer | Decoder (previous MA predictor mode for LSP) |
| prev_lsp | {2339, 4679, 7018, 9358, 11698, 14037, 16377, 18717, 21056, 23396} (= freq_prev_reset) | Q13 | Decoder (frame erasure LSP recovery; distinct from lsp_old) |
| pastVad | 1 | flag | Encoder (previous VAD decision) |
| ppastVad | 1 | flag | Encoder (two-frames-ago VAD decision) |
| past_gain (AGC) | 4096 (= 1.0) | Q12 | Decoder (post-filter AGC smoothed gain) |
| sid_gain | tab_Sidgain[0] (= 2) | Q0 | Decoder (CNG gain) |
| L_exc_err[4] | {0x00004000, 0x00004000, 0x00004000, 0x00004000} (= 1.0 each) | Q14 (Word32) | Encoder (taming state; `Init_exc_err()` in TAMING.C) |
| sid_sav | 0 | Word16 | Decoder (SID energy fallback for CNG when first SID after speech is erased) |
| sh_sid_sav | 1 | Word16 | Decoder (SID energy shift fallback; paired with sid_sav) |
| lspSid | {31441, 27566, 21458, 13612, 4663, -4663, -13612, -21458, -27566, -31441} | Q15 | Decoder (CNG LSP vector; `Init_Dec_cng()` in DEC_SID.C — distinct from speech `lsp_old`) |
| cur_gain (CNG decoder) | 0 | Q0 | Decoder (CNG smoothed gain; C static zero-init in DEC_SID.C, `Init_Dec_cng()`) |
| frame (encoder counter) | 0 | integer | Encoder (frame counter for VAD initialization period; maintained by calling program in C, by `EncoderState` in Rust) |
| mem_pre | 0 | Word16 | Decoder (post-filter tilt compensation preemphasis memory; function-local static in POSTFILT.C, NOT reset by Init_Post_Filter()) |
| mem_w | {0, ..., 0} (M values) | - | Encoder (weighting filter memory) |
| mem_w0 | {0, ..., 0} (M values) | - | Encoder (target signal computation filter memory; updated per subframe via error signal e(n) = x(n) - gp*y1(n) - gc*y2(n)) |
| mem_zero | {0, ..., 0} (M values) | - | Encoder (filtered adaptive codebook vector synthesis memory) |
| All filter memories | 0 | - | All |
| Excitation buffers | 0 | - | All |

---

## 3. Encoder Specification

### 3.1 Pre-Processing

**Requirement:** Apply a 2nd-order IIR high-pass filter with 140 Hz cutoff combined with division by 2.

**Transfer function:**
```
H(z) = (0.46363718 - 0.92724706*z^-1 + 0.46363718*z^-2)
       / (1 - 1.9059465*z^-1 + 0.9114024*z^-2)
```

The division by 2 is incorporated into the numerator coefficients (the values shown are already halved from the original Butterworth prototype).

**Implementation:**
- Direct Form II implementation
- Filter state: 2 previous input samples, 2 previous output samples
- Fixed-point: Use split high/low precision for output history (y1_hi, y1_lo, y2_hi, y2_lo)
- Process sample-by-sample across the 80-sample frame

### 3.2 LP Analysis

**Requirement:** Extract 10th-order LP filter coefficients once per frame using autocorrelation method.

#### 3.2.1 Asymmetric Analysis Window

Window length: 240 samples (30 ms), composed of:

- Part 1 (samples 0-199): Half Hamming window
  ```
  w(n) = 0.54 - 0.46 * cos(2*pi*n / 399),  n = 0,...,199
  ```
- Part 2 (samples 200-239): Quarter cosine taper
  ```
  w(n) = cos(2*pi*(n-200) / 159),  n = 200,...,239
  ```

The window is centered such that it covers 120 past samples + 80 current samples + 40 look-ahead samples.

#### 3.2.2 Autocorrelation

Compute 11 autocorrelation coefficients (lags 0 through 10) for LP analysis (order M=10):
```
r(k) = SUM(n=k to 239) s_w(n) * s_w(n-k),  k = 0,1,...,10
```

> **Annex B note:** When Annex B (VAD/DTX/CNG) is active, `Autocorr()` is called with order NP=12 instead of M=10, producing 13 lags (0-12). The first 11 lags are used for LP analysis (Levinson-Durbin), and all 13 are passed to the VAD energy feature computation (see §8.1.1). The `Autocorr` function signature must accept a variable order parameter. Reference: `COD_LD8A.C:231,242`.

Floor: if r(0) < 1.0, set r(0) = 1.0.

**Overflow-Retry Loop (critical for bit-exactness):**

The autocorrelation computation uses a dynamic scaling loop to handle high-energy frames. The windowed signal `s_w(n)` is computed with a scale factor that increases if 32-bit accumulators overflow:

```
scale = 0
do {
    Overflow = 0                              // Clear global overflow flag
    s_w(n) = signal(n) × window(n) >> (15 + scale)   // Apply window with current scale
    compute r(0) = SUM s_w(n)²                // This may set Overflow = 1
    if (Overflow != 0):
        scale += 1                            // Increase right-shift
} while (Overflow != 0)
// Now compute r(1)..r(10) with the final scaled s_w(n)
```

This loop (from `LPC.C:48-62`) retries the windowed signal computation with increasing right-shifts until the energy fits in 32-bit accumulators. Without this overflow detection, the encoder will produce incorrect LP coefficients on high-energy input frames. The `Overflow` flag is the global flag from the basic operations library (see Section 10.5).

#### 3.2.3 Lag Windowing (Bandwidth Expansion)

Apply 60 Hz bandwidth expansion:
```
r'(k) = r(k) * exp(-0.5 * (2*pi*60*k/8000)^2),  k = 1,...,10
```

The lag window values SHALL be precomputed and stored as a constant table.

#### 3.2.4 White Noise Correction

```
r'(0) = r(0) * 1.0001
```

This adds a -40 dB noise floor to ensure positive-definiteness.

#### 3.2.5 Levinson-Durbin Algorithm

Solve for LP coefficients a(1)...a(10) using iterative Levinson-Durbin recursion:

```
Initialize: E(0) = r'(0)

For i = 1 to 10:
    k_i = -[r'(i) + SUM(j=1 to i-1) a_j^(i-1) * r'(i-j)] / E(i-1)
    a_j^(i) = a_j^(i-1) + k_i * a_{i-j}^(i-1),  j = 1,...,i-1
    a_i^(i) = k_i
    E(i) = (1 - k_i^2) * E(i-1)
```

Stability check: if |k_i| >= 1.0, stop recursion and use coefficients from previous frame.

### 3.3 LP to LSP Conversion

**Requirement:** Convert 10 LP coefficients to 10 Line Spectral Pair frequencies in the cosine domain.

**Algorithm:**

1. Form the sum and difference polynomials:
   ```
   f1(i) = a(i) + a(11-i),  i = 1,...,5
   f2(i) = a(i) - a(11-i),  i = 1,...,5
   ```
   (with appropriate normalization for the symmetric/antisymmetric decomposition)

2. Evaluate f1 and f2 on a grid of 50 points (GRID_POINTS=50, plus endpoints) uniformly spaced in the cosine domain [-1, 1] using Chebyshev polynomial evaluation.

3. Detect sign changes between adjacent grid points to locate root intervals.

4. Refine root locations using 2 iterations of bisection within each interval (Annex A reduces from the base G.729's 4 iterations; see spec A.3.2.3 and reference code `lpc.c` Az_lsp: `for (i = 0; i < 2; i++)`).

5. The 10 roots, alternating between f1 and f2, are the LSP frequencies (in cosine domain).

**Fallback:** If fewer than 10 roots are found, use the LSP values from the previous frame.

**Dead code note:** The reference code `LPCFUNC.C` contains a function `Lsf_lsp()` (Q15-domain LSF-to-LSP conversion) alongside the active `Lsp_lsf()`, `Lsp_lsf2()`, and `Lsf_lsp2()`. `Lsf_lsp()` has no callers in the Annex A/B code path and is dead code — only `Lsf_lsp2()` (Q13-domain variant) is used. The Rust implementation omits `Lsf_lsp()` entirely.

### 3.4 LSP Quantization (18 bits)

**Requirement:** Quantize 10 LSP coefficients using switched MA prediction and two-stage vector quantization.

#### 3.4.1 Structure

| Component | Bits | Codebook Size | Dimension |
|-----------|------|---------------|-----------|
| L0: MA predictor switch | 1 | 2 modes | -- |
| L1: First-stage VQ | 7 | 128 entries | 10-D |
| L2: Second-stage split VQ (lower) | 5 | 32 entries | 5-D (LSPs 1-5) |
| L3: Second-stage split VQ (upper) | 5 | 32 entries | 5-D (LSPs 6-10) |
| **Total** | **18** | | |

#### 3.4.2 MA Prediction

Two sets of 4th-order MA prediction coefficients (selected by L0):
```
lsp_predicted(n) = SUM(i=1 to 4) ma_coeff[mode][i] * residual(n-i)
```

The residual between the current LSP vector and the predicted vector is what gets quantized.

#### 3.4.3 Codebook Search

1. For each of the 2 MA modes:
   - Compute prediction residual
   - Search the 128-entry first-stage codebook (minimize MSE)
   - Compute second-stage residual
   - Search two 32-entry second-stage codebooks (split VQ: lower 5 + upper 5 dimensions)
   - Compute total distortion
2. Select the mode (L0) and indices (L1, L2, L3) yielding minimum total distortion.

#### 3.4.4 LSP Stability

After quantization, enforce ordering with minimum gaps:
- Pass 1: minimum spacing = 0.0012 (in normalized frequency)
- Pass 2: minimum spacing = 0.0006

Boundary constraints: LSP(1) >= 0.005, LSP(10) <= pi - 0.005 (in radians, mapped appropriately).

### 3.5 LSP Interpolation

For each subframe, interpolate between previous and current frame LSPs:

- **Subframe 1:** `lsp_sf1 = 0.5 * lsp_prev + 0.5 * lsp_curr`
- **Subframe 2:** `lsp_sf2 = lsp_curr` (direct, no interpolation)

In Annex A, only **quantized** LSPs are interpolated (via `Int_qlpc`), since the weighting filter uses quantized LP parameters (spec A.3.2.5). The base G.729 `Int_lpc` (unquantized LSP interpolation) does not exist in the Annex A reference code.

Convert interpolated LSPs back to LP coefficients via Chebyshev polynomial reconstruction:
1. Separate 10 LSPs into odd (f1: LSP 1,3,5,7,9) and even (f2: LSP 2,4,6,8,10) groups
2. Build 5th-order polynomials for each group using `Cheb_poly_eval()`
3. Recover LP coefficients: `a[i] = 0.5×(f1[i] + f2[i])`, `a[10-i] = 0.5×(f1[i] - f2[i])`

### 3.6 Perceptual Weighting Filter

**Transfer function (Annex A):**

Unlike base G.729's `W(z) = A(z/γ1) / A(z/γ2)` with signal-adaptive gammas, Annex A uses **quantized** LP coefficients with a fixed gamma:
```
W(z) = Â(z) / Â(z/γ)    with γ = 0.75
```
(Spec A.3.3: "the perceptual weighting filter is based on the quantized LP filter coefficients")

This gives the combined weighted synthesis filter:
```
H(z) = W(z) / Â(z) = 1 / Â(z/γ) = 1 / Â(z/0.75)
```
(Spec A.3.3: "simplifies the combination of synthesis and weighting filters to W(z)/Â(z) = 1/Â(z/γ)")

Note that W(z) does NOT cancel to unity — the numerator is Â(z), not Â(z/γ). The bandwidth-expanded LP coefficients `Ap[i] = â[i] × 0.75^i` are computed via `Weight_Az()` and used for:
- Computing the impulse response h(n) = 1/Â(z/0.75) via `Syn_filt(Ap, ...)`
- Computing the target signal by filtering LP residual through 1/Â(z/0.75)
- Filtering speech through `Â(z)/[Â(z/0.75)·(1 − 0.7z⁻¹)]` to produce weighted speech for open-loop pitch search

The simplification vs. base G.729 is that there is **one fixed gamma** instead of signal-adaptive gammas, and quantized rather than unquantized coefficients are used — but bandwidth expansion by 0.75 is still applied.

> **Note:** Base G.729 uses adaptive gamma values (0.94/0.60 for flat, 0.98/0.40-0.70 for tilted spectrum) based on the first reflection coefficient. This is not implemented.

### 3.7 Open-Loop Pitch Analysis

**Requirement:** Estimate open-loop pitch delay T_op once per frame from weighted speech.

1. Compute weighted speech by filtering through W(z).
2. **Overflow handling:** Before computing correlations, check for overflow in the weighted speech energy computation. If the global `Overflow` flag is set (see Section 10.5), rescale the signal by right-shifting by 3:
   ```
   Overflow = 0
   compute energy of scaled_signal
   if (Overflow != 0):
       for i = 0 to length-1:
           scaled_signal[i] = signal[i] >> 3
   ```
   This prevents correlation computation from producing incorrect results on high-energy frames (from `PITCH_A.C:55-69`).
3. **Compute correlations using only even samples** (j+=2 inner loop step in all three delay ranges). The Annex A spec (A.3.4) states: "only the even samples are used." Note: the algorithm name `Pitch_ol_fast` refers to the Annex A reduced-complexity approach vs base G.729's `Pitch_ol`; the literal sample step is 2, not 3. Reference: `pitch_a.c` lines 112, 142, 172 all use `j+=2`.
4. Search three ranges for maximum normalized correlation on the decimated signal:
   - Range 1: delays 20-39
   - Range 2: delays 40-79
   - Range 3: delays 80-143 — **searches every 2nd sample** in the initial pass, then refines ±1 around the best (Annex A optimization)

   > **Warning (SE6):** The Annex A specification §A.3.4 PDF contains an OCR artifact where this third range reads "80,...,43" instead of "80,...,143" — the leading "1" was dropped during PDF rendering. The reference C code (`PITCH_A.C`) confirms the correct upper bound is 143 (= PIT_MAX). See SPECIFICATION_PLAN §13 SE6.

5. Select the best candidate from each range.
6. Choose the overall best with preference for submultiples (to avoid pitch doubling): the Annex A algorithm (`Pitch_ol_fast`) **boosts** the normalized correlation of lower-delay candidates when they are near sub-harmonics of higher-delay candidates (`pitch_a.c:216-247`), then selects the candidate with the highest (possibly boosted) correlation via direct comparison without a threshold (`pitch_a.c:249-256`). This differs from base G.729's `Pitch_ol` which uses an explicit 0.85× threshold (`THRESHP`).

### 3.8 Target Signal Computation

Compute the target signal x(n) for the adaptive codebook search:

```
x(n) = s_w(n) - s_w0(n)
```

where s_w(n) is the weighted speech and s_w0(n) is the zero-input response of the weighted synthesis filter H(z) = W(z)/A_hat(z).

Equivalently, filter the LP residual through the combination filter with proper state initialization.

### 3.9 Impulse Response Computation

Compute the impulse response h(n) of the weighted synthesis filter for n = 0,...,39:

In Annex A, `W(z)/Â(z) = 1/Â(z/γ)` with γ=0.75 (see Section 3.6), so:
```
H(z) = 1 / Â(z/0.75)
```

The impulse response is computed by filtering a unit sample through `1/Â(z/0.75)`, i.e., `Syn_filt(Ap, ...)` where `Ap` = `Weight_Az(Â, 0.75)`. Truncated to 40 samples (subframe length). Used by both adaptive and fixed codebook searches.

### 3.10 Adaptive Codebook Search

**Requirement:** Find the optimal fractional pitch delay using correlation maximization (Annex A uses correlation-only search, not the weighted MSE minimization of base G.729).

#### 3.10.1 First Subframe (P1 = 8 bits)

- Search range: T_op-3 to T_op+3 (integer), then refine with 1/3-sample resolution
- Fractional pitch delays 19+1/3 to 84+2/3 use 1/3-sample resolution (PIT_MIN=20 constrains the open-loop search; closed-loop fractional refinement extends below it via encoding formula `index = (T-19)*3 + frac - 1` with T in [19,85])
- Integer-only delays 85 to 143 — **the closed-loop fractional pitch search is skipped entirely** when the open-loop estimate falls in range 85-143 (subframe 1 only; subframe 2 always uses fractional search regardless)
- Interpolation filter: Hamming-windowed sinc, length 31 (3 phases × 10 + 1 = `FIR_SIZE_SYN`)

**Encoding (8 bits):**
- Fractional range: `index = 3*(T_int - 19) + T_frac - 1` (where T_frac in {0,1,2})
- Integer range (85-143): `index = T_int - 85 + 197`

#### 3.10.2 Second Subframe (P2 = 5 bits)

- Search range: T1-5+1/3 to T1+4+2/3 (1/3-sample resolution, 30 possibilities)
- Differential encoding relative to first subframe delay

#### 3.10.3 Parity Bit (P0 = 1 bit)

Parity computed on the 6 MSBs of P1 for error detection.

#### 3.10.4 Gain Computation

Optimal pitch gain:
```
g_p = (x^T * y) / (y^T * y)
```
Bounded: 0 <= g_p <= 1.2

where y(n) = h(n) * v(n) (convolution of impulse response with adaptive codebook vector).

**LP residual copy for short delays:** For pitch delays T0 < L_SUBFR (40), the future excitation samples u(n) for n=0,...,39 are not yet known during the search stage. The LP residual is copied into the excitation buffer before the adaptive codebook search to provide these samples (Annex A spec A.3.7). This is handled implicitly in `Pred_lt_3()` which reads from `exc[]` where the LP residual (computed via `Residu()`) already occupies the needed buffer positions.

#### 3.10.5 Target Update

Remove adaptive codebook contribution from target:
```
x2(n) = x(n) - g_p * y(n)
```

### 3.11 Fixed (Algebraic) Codebook Search

**Requirement:** Find the optimal 4-pulse algebraic codebook vector.

#### 3.11.1 Codebook Structure (ISPP)

4 non-zero pulses of amplitude +1 or -1 placed on interleaved tracks:

| Track | Pulse | Possible Positions | Position Bits |
|-------|-------|--------------------|---------------|
| 0 | i0 | 0, 5, 10, 15, 20, 25, 30, 35 | 3 |
| 1 | i1 | 1, 6, 11, 16, 21, 26, 31, 36 | 3 |
| 2 | i2 | 2, 7, 12, 17, 22, 27, 32, 37 | 3 |
| 3 | i3 | 3, 8, 13, 18, 23, 28, 33, 38, 4, 9, 14, 19, 24, 29, 34, 39 | 4 |

Track 3 has 16 possible positions organized as two sub-tracks of 8 positions each: sub-track A starts at position 3 (positions 3, 8, 13, 18, 23, 28, 33, 38 -- step 5) and sub-track B starts at position 4 (positions 4, 9, 14, 19, 24, 29, 34, 39 -- step 5). The MSB of the 4-bit position code selects the sub-track (0 = sub-track A, 1 = sub-track B), and the remaining 3 bits encode the position within that sub-track. This dual sub-track structure is what gives Track 3 a 4-bit position code vs 3 bits for the other tracks.

Encoding per subframe: 13 bits (positions) + 4 bits (signs) = 17 bits.

#### 3.11.2 Search Algorithm (Annex A Depth-First Tree Search)

1. Compute backward-filtered target (correlation vector):
   ```
   d(n) = SUM(i=n to 39) x2(i) * h(i-n)
   ```

2. Precompute correlation matrix Phi(i,j) from impulse response h(n).

3. For each track, preselect candidate positions based on |d(n)|.

4. Use **depth-first tree search**, testing 320 candidate evaluations (160 per track assignment × 2 track assignments; each assignment performs 2 searches of 16 pre-selection + 64 full 4-pulse evaluations). For each candidate combination, maximize:
   ```
   criterion = (d^T * c)^2 / (c^T * Phi * c)
   ```

> **Note:** Base G.729 uses a nested-loop exhaustive search (~100,000 candidates). This is not implemented.

> **Reference implementation note:** bcg729 uses an alternate nested-loop approach testing ~576 candidate evaluations (vs 320 total evaluations — 160 per track assignment — in the ITU reference's depth-first tree search). This produces different encoder output for many input frames. The ITU reference code's depth-first tree search (spec A.3.8.1) is the authoritative target for bit-exact conformance. bcg729's decoder is unaffected since codebook reconstruction is identical regardless of the search method used at encoding time.

### 3.11.3 Pitch Sharpening

> **Scope note:** Although numbered as a subsection of the fixed codebook search, pitch sharpening is a shared operation used by both the encoder (applied to the impulse response h(n) before ACELP search — `ACELP_CA.C:65-68`, and to the code vector after search — `ACELP_CA.C:89-91`) and the decoder (applied to the decoded fixed codebook vector — §5.5.1). The algorithm is identical in both cases.

Pitch sharpening is applied to the impulse response before fixed codebook search (encoder) and to the fixed codebook vector after decoding (both encoder and decoder). This sharpens the spectral shape of the fixed codebook contribution.

**Algorithm:**
```
sharp_q15 = sharp << 1                    // Q14 → Q15
if T0 < L_SUBFR (40):
  for i = T0 to L_SUBFR-1:
    code[i] += code[i - T0] × sharp_q15   // Q13 multiply
```

**After gain quantization/decoding, update sharp for the next subframe:**
```
sharp = clamp(gain_pitch, SHARPMIN, SHARPMAX)   // [3277, 13017] Q14 = [0.2, 0.8]
```

The encoder applies identical sharpening to the impulse response h(n) before the fixed codebook search, so the search accounts for the sharpening effect.

### 3.12 Gain Quantization (7 bits per subframe)

**Requirement:** Jointly vector-quantize adaptive codebook gain g_p and fixed codebook gain g_c.

#### 3.12.1 MA Prediction of Fixed Codebook Gain

4th-order MA predictor in log-energy domain:
```
E_pred = SUM(i=1 to 4) b(i) * R(n-i)
```

MA coefficients: b = [0.68, 0.58, 0.34, 0.19]

The mean energy of the fixed codebook vector is computed and added to the predicted energy to form the predicted gain.

#### 3.12.2 Two-Stage Conjugate Structure

| Stage | Symbol | Bits | Entries |
|-------|--------|------|---------|
| Stage 1 | GA | 3 | 8 |
| Stage 2 | GB | 4 | 16 |
| **Total** | | **7** | |

Quantized gains:
```
g_p_q = GA_gp + GB_gp
g_c_q = (GA_gc + GB_gc) * g_c_predicted
```

Pre-selection: Test NCAN1=4 best stage-1 candidates, then NCAN2=8 best stage-2 candidates.

#### 3.12.3 Taming

The taming procedure prevents adaptive codebook divergence using zone-based error tracking:

**4 pitch zones:**

| Zone | Pitch Range |
|---|---|
| 0 | 20-39 |
| 1 | 40-79 |
| 2 | 80-119 |
| 3 | 120-143 |

`L_exc_err[4]` tracks per-zone accumulated excitation error energy. `Test_err()` returns a taming flag if the maximum zone error exceeds `L_THRESH_ERR` (~2^30). When taming is active:
- Constrain g_p ≤ GPCLIP (15564 Q14 ≈ 0.95) to prevent adaptive codebook divergence
- This prevents runaway feedback in the synthesis filter

**`tab_zone` index boundary handling (critical for Rust):** Both `test_err()` and `update_exc_err()` compute indices into `tab_zone[153]` (size `PIT_MAX+L_INTERPOL-1`) that can go negative for short pitch delays. The C reference code handles both cases with explicit guards:
- `test_err()` (`taming.c:48-51`): computes `i = t1 - (L_SUBFR + L_INTER10)`. When `t1 < 50`, `i` is negative; the code clamps: `if(i < 0) { i = 0; }`, mapping to zone 0.
- `update_exc_err()` (`taming.c:90-109`): computes `n = T0 - L_SUBFR`. When `T0 < 40`, `n` is negative; the code branches to a different path that propagates zone 0's error through two iterations of `Mpy_32_16(L_exc_err[0], gain_pit)` without accessing `tab_zone` at all.

The Rust implementation must replicate these guards. Without them, `tab_zone[n]` would panic on bounds checks for any pitch delay below 40 (which includes valid delays down to `PIT_MIN=20`).

### 3.13 Memory Update

After each subframe:

1. Construct excitation: `u(n) = g_p_q * v(n) + g_c_q * c(n)`
2. Update excitation buffer (shift and append new excitation)
3. Filter excitation through 1/A_hat(z) to update synthesis filter memory
4. Compute error between original speech and locally decoded speech
5. Update target signal memory (mem_w0) — the residual after removing both codebook contributions:
   ```
   mem_w0[j] = xn[L_SUBFR-M+j] - gp×y1[L_SUBFR-M+j] - gc×y2[L_SUBFR-M+j]
   ```

---

## 4. Bitstream Format

### 4.1 Parameter Encoding (80 bits per frame)

| # | Parameter | Symbol | Bits | Subframe |
|---|-----------|--------|------|----------|
| 1 | MA predictor switch | L0 | 1 | Frame |
| 2 | First-stage LSP VQ | L1 | 7 | Frame |
| 3 | Second-stage LSP VQ (lower) | L2 | 5 | Frame |
| 4 | Second-stage LSP VQ (upper) | L3 | 5 | Frame |
| 5 | Pitch delay (1st subframe) | P1 | 8 | SF1 |
| 6 | Parity bit on P1 | P0 | 1 | SF1 |
| 7 | Fixed codebook index (1st) | C1 | 13 | SF1 |
| 8 | Fixed codebook sign (1st) | S1 | 4 | SF1 |
| 9 | Gain codebook stage 1 (1st) | GA1 | 3 | SF1 |
| 10 | Gain codebook stage 2 (1st) | GB1 | 4 | SF1 |
| 11 | Pitch delay (2nd subframe) | P2 | 5 | SF2 |
| 12 | Fixed codebook index (2nd) | C2 | 13 | SF2 |
| 13 | Fixed codebook sign (2nd) | S2 | 4 | SF2 |
| 14 | Gain codebook stage 1 (2nd) | GA2 | 3 | SF2 |
| 15 | Gain codebook stage 2 (2nd) | GB2 | 4 | SF2 |
| | **Total** | | **80** | |

The 15 individual parameters are grouped into 11 encoding units for bitstream packing:

```
prm[0]  = L0 + L1 (1+7 = 8 bits)
prm[1]  = L2 + L3 (5+5 = 10 bits)
prm[2]  = P1 (8 bits)
prm[3]  = P0 (1 bit)
prm[4]  = C1 (13 bits)
prm[5]  = S1 (4 bits)
prm[6]  = GA1 + GB1 (3+4 = 7 bits)
prm[7]  = P2 (5 bits)
prm[8]  = C2 (13 bits)
prm[9]  = S2 (4 bits)
prm[10] = GA2 + GB2 (3+4 = 7 bits)
```

`bitsno[11] = {8, 10, 8, 1, 13, 4, 7, 5, 13, 4, 7}` — total = 80 bits

### 4.2 Bit Allocation Summary

| Category | Bits/Frame | % |
|----------|-----------|---|
| LSP parameters | 18 | 22.5% |
| Pitch delays + parity | 14 | 17.5% |
| Fixed codebook | 34 | 42.5% |
| Gains | 14 | 17.5% |
| **Total** | **80** | **100%** |

### 4.3 Packed Frame Format

The 80 bits SHALL be packed into 10 octets in network byte order (big-endian, MSB first) for RTP transport. The bit ordering follows the ITU-T specification Table 8/G.729.

### 4.4 Annex B SID Frame Format

SID frames contain quantized background noise LSF and energy parameters. The frame type is distinguished by size:
- 10 octets = speech frame
- 2 octets = SID frame
- 0 octets = no transmission (silence)

**OCTET_TX_MODE (default for RTP/SIP and test vector conformance):**

The reference code's `octet.h` defines `OCTET_TX_MODE` **by default**, which changes SID frame size from 15 bits to 16 bits (adding 1 padding zero bit). The Annex B test vectors use OCTET_TX_MODE. If the implementation uses 15-bit SID, it will fail test vector comparison.

**RATE constants:**

| Constant | Value | Description |
|---|---|---|
| RATE_8000 | 80 | Speech frame (80 bits) |
| RATE_SID | 15 | SID frame without octet alignment |
| RATE_SID_OCTET | 16 | SID frame with octet alignment (default) |
| RATE_0 | 0 | No transmission |

**SID parameter encoding (`bitsno2[4] = {1, 5, 4, 5}` = 15 bits):**

| # | Parameter | Bits | Description |
|---|---|---|---|
| 1 | L0 | 1 | Noise MA predictor mode select |
| 2 | L1 | 5 | First-stage noise LSF codebook (32 entries) |
| 3 | L2 | 4 | Second-stage noise LSF codebook (16 entries per half) |
| 4 | Gain | 5 | SID energy quantization (32 levels) |
| | **Total** | **15** | **+ 1 padding bit in OCTET_TX_MODE = 16 bits = 2 octets** |

**Noise LSP quantization (distinct from speech):**
- Uses separate `noise_fg` MA predictor tables derived from speech tables (see Section 8.4)
- L1: 32-entry subset via `PtrTab_1[32]` indirection table
- L2: 16-entry subset via `PtrTab_2[2][16]` indirection table (mode-dependent)
- Stability enforcement: minimum spacing = 10 (Q13) — this applies specifically to the SID noise LSP quantizer (`sid_lsfq_decode`), which uses a different stability enforcement than speech LSPs (which use GAP1=10, GAP2=5, GAP3=321)

**SID gain quantization:**
- 32-level non-uniform table (`tab_Sidgain[32]`):
  ```
  {2, 5, 8, 13, 20, 32, 50, 64, 80, 101, 127, 160, 201, 253, 318, 401,
   505, 635, 800, 1007, 1268, 1596, 2010, 2530, 3185, 4009, 5048, 6355,
   8000, 10071, 12679, 15962}
  ```
- Range: approximately -12 dB to +66 dB

### 4.5 ITU Serial Bitstream Format

The ITU reference code uses a "serial" format (not packed bytes) for test vectors:

**Structure per frame:**
1. SYNC_WORD: `0x6B21` (16-bit, always present)
2. SIZE_WORD: `80` (speech), `15` or `16` (SID), or `0` (no-tx) — 16-bit
3. Bit data: SIZE_WORD × 16-bit words, each either `BIT_0 (0x007F)` or `BIT_1 (0x0081)`

**Frame type detection (decoder):**
- SIZE_WORD = 80 → speech frame (ftyp=1), BFI from bit-level inspection
- SIZE_WORD = 15/16 → SID frame (ftyp=2)
- SIZE_WORD = 0 → no transmission (ftyp=0), BFI = check SYNC_WORD

**BFI Detection in ITU Serial Format (critical for test vector conformance):**

The `read_frame()` function in `bits.c` detects BFI by inspecting individual bit values in the serial stream:

```
parm[0] = 0;                                    // assume good frame
if (serial[1] != 0) {                           // SIZE_WORD non-zero
    for (i = 0; i < serial[1]; i++)
        if (serial[i+2] == 0) parm[0] = 1;      // any zero-value bit → frame erased
}
else if (serial[0] != SYNC_WORD) parm[0] = 1;   // untransmitted: check sync
```

**Key subtlety:** A bit value of `0x0000` (not `BIT_0 = 0x007F`) indicates an erased bit. BFI is triggered when ANY bit in the frame has the value 0x0000 — each zero-valued word (as opposed to BIT_0 or BIT_1) signals bit erasure.

For production RTP use, BFI comes from the transport layer (packet loss detection) rather than bit-level inspection.

The implementation must support both BFI detection methods:
1. **ITU serial format** — bit-level zero detection for test vector processing
2. **External BFI** — from RTP packet loss for production SIP integration

---

## 5. Decoder Specification

### 5.1 Parameter Decoding

Extract all parameters from the 80-bit frame:

1. Decode LSP indices (L0, L1, L2, L3) -> quantized LSP vector
2. Decode pitch delays (P1, P2) -> fractional pitch delays T1, T2
3. Verify parity bit (P0) on P1
4. Decode fixed codebook (C1, S1, C2, S2) -> codebook vectors
5. Decode gains (GA1, GB1, GA2, GB2) -> quantized gains

#### 5.1.1 Annex B Augmented Parameter Vector

When Annex B (DTX/CNG) is active, the decoder uses an augmented parameter vector populated by `read_frame()`:

```
parm[0] = BFI        // 0 = good frame, 1 = erased frame
parm[1] = frame_type // 0 = untransmitted, 1 = speech, 2 = SID
parm[2..12] = speech parameters (if ftyp == 1)
parm[2..5]  = SID parameters (if ftyp == 2)
```

Frame type is determined from SIZE_WORD in the ITU serial format:
- SIZE_WORD = 80 → ftyp = 1 (speech)
- SIZE_WORD = 15 or 16 → ftyp = 2 (SID)
- SIZE_WORD = 0 → ftyp = 0 (untransmitted)

The `bad_lsf` flag (see Section 6.1) is OR'd with BFI when decoding LSPs:
```
D_lsp(parm, lsp_new, bfi | bad_lsf)
```
This allows channel protection to signal corrupted LSP indices independently of full frame erasure.

### 5.2 LSP Decoding

1. Use L0 to select MA predictor mode.
2. Use L1 to index the 128-entry first-stage codebook.
3. Use L2, L3 to index the two 32-entry second-stage codebooks.
4. Add the codebook contributions to the MA-predicted LSP vector.
5. Enforce stability (ordering + minimum spacing).
6. Interpolate: SF1 = average of previous and current; SF2 = current.
7. Convert interpolated LSPs to LP coefficients.

### 5.3 Pitch Delay Decoding

**First subframe (P1, 8 bits):**
- If index < 197: `T0 = (index + 2) / 3 + 19` (integer division), `T0_frac = index - T0*3 + 58` (yields T0_frac in {-1, 0, 1}). Reference: `DEC_LAG3.C:37-44`
- If index >= 197: `T0 = index - 112`, `T0_frac = 0` (integer-only delays 85-143). Reference: `DEC_LAG3.C:48-49`

**Parity check:** Compute parity on P1's 6 MSBs; if mismatch with P0, flag potential error.

**Second subframe (P2, 5 bits):**
- Compute search range: `T0_min = T0_SF1 - 5`, `T0_max = T0_min + 9`, clamped to `[PIT_MIN, PIT_MAX]`
- Decode: `i = (index + 2) / 3 - 1`, `T0 = i + T0_min`, `T0_frac = index - 2 - i*3` (yields T0_frac in {-1, 0, 1}). Reference: `DEC_LAG3.C:71-80`

### 5.4 Adaptive Codebook Reconstruction

Construct the adaptive codebook vector v(n) by extracting past excitation at the decoded pitch delay, using sinc interpolation for fractional delays.

**Interpolation filter:** Hamming-windowed sinc with 1/3-sample resolution (UP_SAMP=3). Three phase-shifted filter coefficient sets stored for fractional offsets 0, 1/3, 2/3.

### 5.5 Fixed Codebook Reconstruction

Decode C and S parameters to reconstruct the 4-pulse algebraic codebook vector:
- Extract 4 pulse positions from the 13-bit index (3+3+3+4 bits per track)
- Extract 4 signs from the 4-bit sign word
- Construct 40-sample vector with pulses at decoded positions with decoded signs

#### 5.5.1 Pitch Sharpening (Decoder)

After reconstructing the fixed codebook vector, apply pitch sharpening:

```
sharp_q15 = sharp << 1                    // Q14 → Q15
if T0 < L_SUBFR (40):
  for i = T0 to L_SUBFR-1:
    code[i] += code[i - T0] × sharp_q15   // Q13 multiply
```

After gain decoding, update sharp for the next subframe:
```
sharp = clamp(gain_pitch, SHARPMIN, SHARPMAX)   // [3277, 13017] Q14 = [0.2, 0.8]
```

### 5.6 Gain Decoding

1. Use GA and GB indices to look up gain codebook entries.
2. Compute predicted fixed codebook gain using MA predictor.
3. Apply:
   ```
   g_p = GA_gp + GB_gp
   g_c = (GA_gc + GB_gc) * g_c_predicted
   ```
4. Update gain prediction memory with quantized residual.

### 5.7 Excitation Construction

```
u(n) = g_p * v(n) + g_c * c(n),   n = 0,...,39
```

### 5.8 LP Synthesis Filter

```
s_hat(n) = u(n) + SUM(k=1 to 10) a_hat(k) * s_hat(n-k)
```

Maintain 10-sample filter memory across subframes.

#### 5.8.1 Synthesis Overflow Handling

The decoder must detect and handle arithmetic overflow during LP synthesis filtering:

```
Overflow = 0
Syn_filt(Az, exc, synth, L_SUBFR, mem_syn, update=0)    // trial synthesis
if Overflow:
  // Scale down ENTIRE excitation buffer by 4 (right shift by 2)
  for i = 0 to PIT_MAX + L_INTERPOL + L_FRAME - 1:
    old_exc[i] >>= 2
  // Redo synthesis with memory update
  Syn_filt(Az, exc, synth, L_SUBFR, mem_syn, update=1)
else:
  // Copy last M samples to filter memory
  mem_syn = synth[L_SUBFR-M : L_SUBFR-1]
```

The `Overflow` flag is set by any saturating operation within `Syn_filt()`. This prevents synthesis filter instability from producing clipped output.

> **Reference implementation note:** bcg729 handles synthesis overflow via per-sample saturation (clamping each output to MAXINT16) instead of the retry-with-scaling approach above. This produces different decoder output for frames that trigger synthesis overflow. The ITU `OVERFLOW.BIT` test vector specifically exercises the retry-with-scaling path. The ITU reference code's approach (detect Overflow flag, scale excitation >>2, re-synthesize) is required for bit-exact conformance.

### 5.9 Post-Processing

The post-processing chain consists of the following stages. The **execution order** in the reference code (`postfilt.c:133-192`) differs from the subsection numbering below — subsections are grouped conceptually, but the implementation must follow this per-subframe pipeline order:

| Step | Operation | Subsection | Reference |
|------|-----------|------------|-----------|
| 1 | Compute residual through `A(z/γ_n)` (γ_n=0.55, GAMMA2_PST) | §5.9.1 steps 1-2 | `postfilt.c:133-140` |
| 2 | Scale residual >>2 for overflow headroom | §5.9.1 step 2 | `postfilt.c:141-143` |
| 3 | Pitch post-filter on residual (correlation search, filtering) | §5.9.1 steps 3-5 | `postfilt.c:148-160` |
| 4 | Tilt compensation `H_t(z) = 1 + μ·k'_1·z⁻¹` on residual-domain signal | §5.9.3 | `postfilt.c:172-180` |
| 5 | Formant synthesis through `1/A(z/γ_d)` (γ_d=0.70, GAMMA1_PST) | §5.9.2 | `postfilt.c:182-184` |
| 6 | Adaptive Gain Control (AGC) | §5.9.4 | `postfilt.c:186-188` |

> **Critical:** Tilt compensation (step 4) is applied BEFORE formant synthesis (step 5). Annex A spec A.4.2.3 confirms: "The compensation filtering H_t(z) is performed before synthesis through 1/Â(z/γ_d)." Implementing these in the wrong order will produce incorrect output and fail conformance.

#### 5.9.1 Long-Term (Pitch) Post-Filter

The post-filter operates on a **spectrally shaped residual**, not the synthesized speech directly and not the plain LP residual. The full per-subframe pipeline is:

1. Compute residual through `A(z/gamma_n)` (the formant post-filter numerator, GAMMA2_PST=0.55): `Weight_Az(Az, GAMMA2_PST, M, Ap3); Residu(Ap3, synth, res2, L_SUBFR)`. This produces a spectrally shaped signal, NOT a plain LP residual. Using plain `A(z)` or `A(z/gamma_d)` would give a different signal. Reference: `postfilt.c:133-138`. (**Corrected:** previously said `A(z/gamma_d)` with value 0.70 — this was the wrong gamma subscript; see errata E14 in SPECIFICATION_PLAN.md.)
2. Scale residual down by factor of 4 for overflow avoidance: `scal_res2[j] = shr(res2[j], 2)`. The pitch post-filter correlation search operates on `scal_res2[]`; the output uses the unscaled `res2[]`. Reference: `postfilt.c:140-141`.
3. Search for integer pitch delay T in the **scaled** residual signal, ±3 around the decoded delay, clamped to PIT_MAX only (no explicit PIT_MIN check; lower bound is implicitly safe from minimum decoded pitch range). **Integer delays only** per Annex A. Reference: `postfilt.c:124-129`.
4. Compute gain on residual correlation: jointly normalize `cor_max`, `ener`, `ener0` via `norm_l`, then test the 3 dB threshold: `L_mult(cmax, cmax) < L_shr(L_mult(en, en0), 1)`. If below threshold, bypass pitch PF (copy `res2` to `res2_pst` unchanged — also done when `Vad==0` for CNG frames).
5. Derive pitch post-filter gains `g0` and `gain` from the correlation (`postfilt.c:306-324`):
   - If `cmax > en` (pitch gain > 1): `g0 = INV_GAMMAP` (21845, Q15), `gain = GAMMAP_2` (10923, Q15)
   - Otherwise: `cmax_q14 = shr(mult(cmax, GAMMAP), 1)`, `en_q14 = shr(en, 1)`, then `gain = div_s(cmax_q14, cmax_q14 + en_q14)` (Q15), `g0 = 32767 - gain` (Q15). If `cmax_q14 + en_q14 <= 0`, fallback: `g0 = 32767`, `gain = 0`
   - Apply: `signal_pst[i] = g0 * signal[i] + gain * signal[i-T]` (Q15 multiplies via `mult`)
   - This implements `H_p(z) = g_l * (1 + b * z^-T) / (1 + b)` where `g_l = g0/(1+gain)` maps to the direct-form coefficients above using `GAMMAP=16384` (0.5 Q15), `INV_GAMMAP=21845` (1/(1+GAMMAP) Q15), `GAMMAP_2=10923` (GAMMAP/(1+GAMMAP) Q15)

#### 5.9.2 Short-Term (Formant) Post-Filter

The synthesis step re-filters the pitch-post-filtered residual through `A(z/gamma_d)`, NOT through plain `A(z)`:

```
Syn_filt(A(z/gamma_d), res2_pst, output, L_SUBFR, mem_syn_pst)
```

| Parameter | Value |
|-----------|-------|
| gamma_n (numerator of H_f, used for residual via Residu) | 0.55 (GAMMA2_PST) |
| gamma_d (denominator of H_f, used for synthesis via Syn_filt) | 0.70 (GAMMA1_PST) |

The combined formant post-filter transfer function is `H_f(z) = A(z/gamma_d) / A(z/gamma_n)`. Reference: `postfilt.c:184`.

**Note (Annex A):** The gain normalization factor `g_f` from base G.729 is **eliminated** in Annex A (spec A.4.2). The formant post-filter applies spectral weighting only, with no explicit gain normalization. Reference: `postfilt.c` (no `gf` variable exists in the ITU reference code).

#### 5.9.3 Tilt Compensation

```
H_t(z) = 1 + gamma_t * k'_1 * z^-1    (Annex A eq A.15)
```

where `k'_1 = -r_h(1)/r_h(0)` is the first reflection coefficient of the **combined post-filter impulse response** `h_f` (the impulse response of `A(z/gamma_d)/A(z/gamma_n)`), NOT the LP analysis reflection coefficient. The code computes `h_f[0..21]` by applying `Syn_filt(A(z/gamma_d))` to `Residu(A(z/gamma_n))` of a unit impulse. Reference: `postfilt.c:156-160`.
- gamma_t = 0.8 when k'_1 < 0 (equivalently, r_h(1) > 0); gamma_t = 0.0 otherwise (bypass)
- The reference code (`postfilt.c:162-180`) computes `g = MU * r_h(1)/r_h(0)` directly (avoiding the k'_1 sign flip) and applies `preemphasis(signal, g, L)` which implements `signal[i] = signal[i] - g * signal[i-1]`, yielding `H_t = 1 - g*z^-1 = 1 + gamma_t * k'_1 * z^-1`

#### 5.9.4 Adaptive Gain Control (AGC)

Scale the post-filtered signal to match the energy of the pre-post-filter signal:
```
G(n) = alpha * G(n-1) + (1-alpha) * sqrt(E_in(n) / E_out(n))
output(n) = G(n) * filtered(n)
```

The smoothing factor alpha controls adaptation speed.

**Zero-energy edge case:** When the post-filtered signal energy `E_out` is zero (sum of squared samples = 0), the gain scaling factor G is set to zero and the previous adaptive gain state is reset to 0. The post-filtered signal passes through unchanged. This prevents division by zero and is present in both the ITU reference code (`postfilt.c` `agc()`) and bcg729 but is not explicitly documented in the specification.

#### 5.9.5 VAD-Dependent Post-Filter Behavior (Annex B)

When Annex B is active, the `Vad` flag is passed to `Post_Filter()`:
```
Post_Filter(synth, Az_dec, T2, Vad)
```

During comfort noise generation (`Vad = 0`), the long-term (pitch) post-filter is **bypassed entirely** — `res2` is copied directly to `res2_pst` without any correlation search or pitch filtering (`postfilt.c:148-151`). The formant post-filter, tilt compensation, and AGC stages still apply. The `old_T0` value is preserved (not overwritten) during CNG so that when speech resumes and the pitch post-filter reactivates, it has a valid pitch delay to search around.

#### 5.9.6 High-Pass Filter

2nd-order IIR high-pass filter at ~100 Hz applied to the output signal.

#### 5.9.7 Upscaling

Multiply by 2 to reverse the encoder's divide-by-2 preprocessing. This is combined with the high-pass filter coefficients.

---

## 6. Frame Erasure Concealment

### 6.1 Detection

Frame erasure is signaled by:
- External Bad Frame Indicator (BFI) from the transport layer (RTP/SRTP)
- Parity check failure on P1 (pitch delay of first subframe)
- `bad_lsf` flag — an independent error indicator that signals corrupted LSP indices without full frame erasure. It is OR'd with BFI specifically for LSP decoding: `D_lsp(parm, lsp_new, bfi | bad_lsf)`. For bit-exact conformance with test vectors, `bad_lsf` must be initialized to 0 (see Section 2.2.1). In production SIP deployments, it can be driven by channel protection schemes.

### 6.2 Concealment Strategy

When a frame is erased:

**LP coefficients:** Use previous quantized LSP values. `D_lsp()` receives the erasure flag and repeats the last good LSP vector, then converts to LP coefficients normally.

**Pitch delay during erasure:**

The pitch delay increment happens **per subframe**, not per frame. For each subframe during erasure: use `T0 = old_T0`, set `T0_frac = 0`, then increment `old_T0 = add(old_T0, 1)` capped at PIT_MAX (143). Both subframes execute this identical logic, so across one erased frame `old_T0` increments by **+2** (one per subframe). Reference: `dec_ld8a.c` lines 243-249 (SF1) and 259-265 (SF2) — both paths have `T0 = old_T0; T0_frac = 0; old_T0 = add(old_T0, 1);`.

**Fixed codebook during erasure:**
```
codebook_index = Random() & 0x1FFF   // 13 random bits
sign_word = Random() & 0x000F        // 4 random bits
```
Then decode normally via `Decod_ACELP()` using these random indices.

**Gains:** Different attenuation factors for pitch and code gains:
```
g_p(erased) = 0.9 * g_p(previous)    // ×29491 Q15; g_p is Q14, result Q14
if g_p > 0.9: g_p = 0.9              // cap at 29491 Q14 (= 0.9 × 16384)
g_c(erased) = 0.98 * g_c(previous)   // ×32111 Q15; g_c is Q1, result Q1
```
**Critical:** `past_qua_en[]` (gain prediction memory) **IS updated** during erasure via `Gain_update_erasure()` (`gainpred.c:137-158`, called from `dec_gain.c:67`). This function computes the average of all 4 `past_qua_en` entries, subtracts 4.0 dB (4096 in Q10), clamps to a minimum of -14.0 dB (-14336 Q10), shifts the history array, and inserts the decayed average into `past_qua_en[0]`. The attenuated concealment gains (0.9×g_p, 0.98×g_c) themselves are NOT fed back into the MA predictor — but `past_qua_en` IS modified with a decaying average. This gradual decay toward -14 dB is what enables stable recovery: when good frames resume, the MA predictor operates from a decayed energy floor rather than the pre-erasure state.

**Parity check interaction:**
- `bad_pitch = bfi OR parity_error`
- Parity error triggers pitch concealment for **subframe 1 only**
- LSP and gain decoding use BFI only (not parity error)
- Second subframe pitch uses BFI only

**Excitation generation:**
- **Voiced (periodic):** Repeat the pitch-period waveform from past excitation
- **Unvoiced (non-periodic):** Generate pseudo-random excitation using LCG:
  ```
  seed = 31821 * seed + 13849
  ```

### 6.3 Recovery

On receiving a good frame after erasures:
- Resume normal decoding immediately — the reference decoder (`dec_ld8a.c`) has **no explicit gain limiting or smoothing** on the first good frame
- `D_lsp()` decodes LSPs normally, `Dec_gain()` calls `Gain_predict()` then `Gain_update()` with the actual quantized gain
- Recovery stability is emergent: `Gain_update_erasure()` during erasure decays `past_qua_en` toward a -14 dB floor, so when normal decoding resumes the MA predictor operates from this decayed floor rather than tracking artificial concealment gains
- Update all state variables normally

### 6.4 Voicing Classification

The Annex A decoder has **no explicit voicing classification**. The Annex A specification (A.4.4) states: "Same as 4.4/G.729 with the difference that no voicing detection is used. The excitation is always the addition of both adaptive and fixed codebook contributions." The voiced/unvoiced distinction during frame erasure is emergent from the previous pitch gain magnitude — high previous `gain_pitch` produces pitch-coherent concealment, low gain produces noise-like output. There is no explicit "> 3 dB prediction gain" check in the Annex A reference code.

> **Note:** Base G.729 (not implemented here) classifies frames as periodic if long-term prediction gain > 3 dB. This classification does not exist in the Annex A code path.

### 6.5 Frame Erasure Interaction with DTX

When BFI is signaled during DTX operation:
- Check `past_ftyp` (previous frame type):
  - If past_ftyp = 1 (speech): treat as voice frame erasure (Section 6.2)
  - If past_ftyp = 0 or 2 (SID/DTX): continue CNG with previous SID parameters
- On SID frame erasure: use previously decoded SID gain and LSP, continue comfort noise generation
- `past_ftyp` is updated at the end of each successfully decoded frame

**BFI handling with Annex B frame type routing:**

When a frame is erased, the decoder routes based on `past_ftyp` and updates the frame type field:

```
bfi = *parm++;                  // read BFI from parm[0], advance pointer
ftyp = *parm;                   // read frame type from parm[1]
if (bfi == 1) {
    if (past_ftyp == 1) ftyp = 1;   // previous was speech → speech erasure
    else ftyp = 0;                    // previous was SID/DTX → DTX erasure
    *parm = ftyp;                     // write to parm[1] (V1.3 maintenance update)
}
```

No forced parity error is needed. The pitch concealment path for subframe 1 is triggered naturally by the decoder's existing logic: `bad_pitch = add(bfi, parity_result)`, which is non-zero whenever `bfi == 1`, regardless of the parity check result. This causes subframe 1 to use the previous pitch delay (`old_T0`) instead of decoding a corrupted value. Reference: `dec_ld8a.c` lines 147-155 (frame type routing) and 234-235 (bad_pitch computation).

**SID energy recovery from last speech frame (`sid_sav` / `sh_sid_sav`):**

When the first SID frame after speech is erased, the decoder needs to estimate SID gain but has no decoded SID parameters. The reference code saves energy information from the last speech frame:

```
// In dec_sid.c — when first noise frame after speech is erased
if (past_ftyp == 1) {   // transition from speech
    Qua_Sidgain(&sid_sav, &sh_sid_sav, 0, &temp, &ind);
    sid_gain = tab_Sidgain[(int)ind];
}
```

The `sid_sav` and `sh_sid_sav` values are saved during speech decoding and provide a fallback estimate for CNG gain when the first SID frame is lost.

### 6.6 Homing Frame Detection

> **Note:** Homing frame detection is **not implemented** in the Annex A reference code, the Annex B reference code, or bcg729. No homing frame test vectors exist in the Release 3 test suite. The feature is defined in the ITU-T Implementers' Guide (G.Imp729) for testing purposes.

**Concept (from Implementers' Guide):**
- **Encoder Homing Frame (EHF):** 80 samples of all zeros as input
- When encoder receives EHF AND codec is in initial state → produces Decoder Homing Frame (DHF) bitstream
- After producing DHF, encoder resets all state to initial values
- **Decoder Homing Frame (DHF):** The specific bitstream pattern produced by encoding EHF from initial state
- When decoder receives DHF → resets all state to initial values

**Implementation approach:** Generate the DHF pattern by encoding 80 zeros from initial state, then use that pattern for detection. Verify by round-tripping: `encode(zeros) → detect(DHF) → reset → encode(zeros)` should produce identical output.

---

## 7. Annex A Algorithm Summary

This implementation uses the Annex A (reduced-complexity) algorithms exclusively. The following table summarizes where Annex A differs from the base G.729 specification described in the reference documents. The encoder and decoder sections above (Sections 3 and 5) already describe the Annex A algorithms directly.

### 7.1 Annex A vs Base G.729 (Reference Only)

| Component | Base G.729 (not implemented) | G.729A (this implementation) |
|-----------|-----------|--------|
| Open-loop pitch | Full-rate correlation | Decimated search (even samples, j+=2) |
| Adaptive codebook | Weighted MSE search | Correlation-only |
| Fixed codebook | Nested loop (~100K candidates) | Depth-first tree (320 candidate evaluations) |
| Weighting filter | Adaptive gamma (signal-dependent) | Fixed gamma = 0.75 |
| Pitch post-filter | Fractional interpolation | Integer delays only |
| Complexity | ~20 MIPS | ~10-12 MIPS |

### 7.2 Interoperability

- G.729A output is **bitstream-compatible** with any compliant G.729 decoder, and vice versa
- The 80-bit frame format is identical
- Quality is marginally lower under stress conditions (MOS ~3.7 vs ~3.9 for base G.729)

### 7.3 Version Targeting

This implementation targets the following specific versions of the ITU-T reference code:

| Component | Version | Date | Key Changes from Earlier Versions |
|---|---|---|---|
| Annex A reference code | v1.1 | September 1996 | Changed frame erasure detection from v1.0 |
| Annex B reference code | v1.5 | October 2006 | v1.3 introduced `parm[1] = ftyp` modification for DTX |

Notable version-specific behaviors:
- **v1.1 (Annex A):** Frame erasure BFI detection method changed from v1.0 — the implementation must use the v1.1 bit-level zero detection (see Section 4.5)
- **v1.3+ (Annex B):** The `*parm = ftyp` assignment during BFI handling (see Section 6.5) was introduced in v1.3 for correct DTX interaction
- **v1.5 (Annex B):** Latest maintenance version with all accumulated fixes

The ITU-T test vectors correspond to these specific versions. Using algorithms from earlier versions will produce different output.

---

## 8. Annex B - VAD/DTX/CNG

### 8.1 Voice Activity Detection (VAD)

#### 8.1.1 Feature Extraction (per frame)

4 features computed from the preprocessed speech signal:

1. **Full-band energy (Ef, Q11):**
   - `Ef = Log2(r[0]) × 9864` (where r[0] is autocorrelation at lag 0 from NP=12 order analysis)
   - Uses 13 autocorrelation coefficients (lags 0-12, NP=12 for VAD; distinct from the speech LP analysis which uses M=10, lags 0-10)
   - Offset: subtract 4875 (Q11)

2. **Low-band energy (El, Q11):**
   - Compute weighted autocorrelation: `r_l[k] = r[k] × lbf_corr[k]` for k=0..NP
   - `lbf_corr[13] = {7869, 7011, 4838, 2299, 321, -660, -782, -484, -164, 3, 39, 21, 4}` — these are the autocorrelation coefficients of the low-pass FIR filter impulse response `h` described in Annex B spec B.3.1.3 (cutoff ≈ 1 kHz), used to weight the full-band autocorrelation into a low-band estimate
   - `El = Log2(r_l[0]) × 9864`, offset by -4875

3. **Spectral distortion (SD, Q15):**
   - `SD = Σ(lsf[i] - MeanLSF[i])²` for i=0..9

4. **Zero-crossing rate (ZC, Q15):**
   - Count sign changes in preprocessed signal, samples ZC_START=120 to ZC_END=200
   - Each crossing adds 410 (Q15)

#### 8.1.2 Decision Algorithm

**Input features (differential, computed against running means):**
- `dSE = MeanSE - Ef` (full-band energy difference, Q11)
- `dSLE = MeanSLE - El` (low-band energy difference, Q11)
- `dSZC = MeanSZC - ZC` (zero-crossing rate difference, Q15)
- `SD` (spectral distortion, Q15)

**MakeDec() — 14 linear discriminant conditions:**

| # | Variables | Computation | Decision |
|---|---|---|---|
| 1 | SD, dSZC | `L_mult(dSZC,-14680) + L_mac(8192,-28521)` → `L_shr(acc0,8)` → `L_add(acc0, L_deposit_h(SD))` | > 0 → VOICE |
| 2 | SD, dSZC | `L_mult(dSZC,19065) + L_mac(8192,-19446)` → `L_shr(acc0,7)` → `L_add(acc0, L_deposit_h(SD))` | > 0 → VOICE |
| 3 | dSE, dSZC | `L_mult(dSZC,20480) + L_mac(8192,16384)` → `L_shr(acc0,2)` → `L_add(acc0, L_deposit_h(dSE))` | < 0 → VOICE |
| 4 | dSE, dSZC | `L_mult(dSZC,-16384) + L_mac(8192,19660)` → `L_shr(acc0,2)` → `L_add(acc0, L_deposit_h(dSE))` | < 0 → VOICE |
| 5 | dSE | `L_mult(dSE,32767) + L_mac(1024,30802)` | < 0 → VOICE |
| 6 | dSE, SD | `L_mult(SD,-28160) + L_mac(64,19988) + L_mac(dSE,512)` | < 0 → VOICE |
| 7 | SD | `L_mult(SD,32767) + L_mac(32,-30199)` | > 0 → VOICE |
| 8 | dSE, dSZC | `L_mult(dSZC,-20480) + L_mac(8192,22938)` → `L_shr(acc0,2)` → `L_add(acc0, L_deposit_h(dSE))` | < 0 → VOICE |
| 9 | dSE, dSZC | `L_mult(dSZC,23831) + L_mac(4096,31576)` → `L_shr(acc0,2)` → `L_add(acc0, L_deposit_h(dSE))` | < 0 → VOICE |
| 10 | dSE | `L_mult(dSE,32767) + L_mac(2048,17367)` | < 0 → VOICE |
| 11 | dSLE, SD | `L_mult(SD,-22400) + L_mac(32,25395) + L_mac(dSLE,256)` | < 0 → VOICE |
| 12 | dSLE, dSE | `L_mult(dSE,-30427) + L_mac(256,-29959)` → `L_add(acc0, L_deposit_h(dSLE))` | > 0 → VOICE |
| 13 | dSLE, dSE | `L_mult(dSE,-23406) + L_mac(512,28087)` → `L_add(acc0, L_deposit_h(dSLE))` | < 0 → VOICE |
| 14 | dSLE, dSE | `L_mult(dSE,24576) + L_mac(1024,29491) + L_mac(dSLE,16384)` | < 0 → VOICE |

If **any** condition is satisfied → return VOICE. Otherwise → return NOISE.

> **ITU spec discrepancy (SE3):** Conditions 8-10 use `dSE` (full-band energy) above, matching the reference code (`vad.c:419-430`). The Annex B specification Table B.1 labels these conditions as involving `delta_El` (low-band energy), and the code comments at `vad.c:415` also say "dSLE". The **executable code** uses `dSE` and is authoritative for bit-exact conformance. See SPECIFICATION_PLAN §13 SE3 for full details.

#### 8.1.3 Initialization and Smoothing

**Initialization (frames 0-32, i.e., 33 frames total where `sub(frm_count, INIT_FRAME) <= 0` with INIT_FRAME=32):**
- Hard threshold: if `Ef < 3072` → NOISE, else → VOICE
- Accumulate statistics only on VOICE frames (exponential averaging, weight 1024)
- Track `less_count` (frames below threshold)
- At frame 32 (the 33rd and final initialization frame): adjust means using `factor_fx[less_count]` and `shift_fx[less_count]` tables; clear initialization flag
- Set: `MeanSE = MeanE - 2048`, `MeanSLE = MeanE - 2458`

**Smoothing pipeline (4 stages, post-MakeDec):**
1. **Inertia:** If NOISE but count_inert < 6 → force VOICE; reset on VOICE
2. **Energy hangover:** If NOISE, prev=VOICE, `Ef-prev_Ef > 2dB` (614 Q11), `Ef > 3072` → force VOICE
3. **Extension:** If NOISE, prev 2 frames VOICE, `|ΔEf| ≤ 614` → force VOICE up to count_ext=4 frames
4. **Forced noise:** If `(Ef - 614 < MeanSE)` AND `(frm_count > 128)` AND `(!v_flag)` AND `(rc < 19661)` → force NOISE. Reference: `vad.c:270-272`. Note: this override uses a stricter reflection coefficient threshold (`rc < 19661 ≈ 0.6 Q15`) than the background noise update condition (`rc < 24576 ≈ 0.75 Q15`)

#### 8.1.4 Background Noise Update

Update conditions: `(Ef - 614 < MeanSE) AND (rc < 24576) AND (SD < 83)`

Running averages update ONLY during confirmed non-speech segments, with rate-adaptive coefficients:

| Frame Count | COEF | C_COEF | COEFZC | C_COEFZC | COEFSD | C_COEFSD |
|---|---|---|---|---|---|---|
| < 20 | 24576 | 8192 | 26214 | 6554 | 19661 | 13017 |
| 20-29 | 31130 | 1638 | 30147 | 2621 | 21299 | 11469 |
| 30-39 | 31785 | 983 | 30802 | 1966 | 22938 | 9830 |
| 40-49 | 32440 | 328 | 31457 | 1311 | 24576 | 8192 |
| 50-59 | 32604 | 164 | 32440 | 328 | 24576 | 8192 |
| 60+ | 32604 | 164 | 32702 | 66 | 24576 | 8192 |

All values Q15. Update: `Mean = (COEF × Mean + C_COEF × current) >> 15`

**Minimum energy tracking:**
- Maintain 16-entry min buffer (8-frame windows)
- After frame 128: sliding window minimum with dual-boundary approach
- If MeanEf drifts below Emin-10dB, reset update count

#### 8.1.5 Encoder Conditional Processing During VAD=NOISE

When VAD classifies a frame as noise, the encoder does NOT skip all processing:
- Autocorrelation is still computed and saved for DTX filter averaging (Section 8.2)
- LSPs are NOT quantized using the speech quantizer
- SID-specific LSF quantization uses reduced codebooks (`PtrTab_1[32]`, `PtrTab_2[2][16]`)
- The DTX state machine (Section 8.2) decides between SID, no-transmission, and speech output
- The adaptive/fixed codebook search and gain quantization are skipped entirely
- **Filter memory update loop (critical for speech resumption):** Even though the subframe coding loop is skipped, the encoder must still update `wsp[]`, `mem_w`, and `mem_w0` for each subframe to keep filter memories current. The per-subframe pipeline (`COD_LD8A.C:270-291`) is: (a) compute LP residual via `Residu(Aq, speech, xn)`, (b) build tilt-compensated filter `Ap[i] = Ap_t[i] - 0.7*Ap_t[i-1]` where `Ap_t = Weight_Az(Aq, GAMMA1)`, (c) synthesize `Syn_filt(Ap, xn, wsp)` with `mem_w` state update, (d) compute `mem_w0` update via `xn[i] = residu[i] - exc[i]` then `Syn_filt(Ap_t, xn, xn)` with `mem_w0` state update. Without this loop, filter memories become stale and speech quality degrades when active speech resumes. Additionally, `sharp` is reset to `SHARPMIN` and speech/wsp/exc buffers are shifted

### 8.2 Discontinuous Transmission (DTX)

**Frame type signaling (encoder output):**
- `ana[0] = 0`: No transmission (0 octets)
- `ana[0] = 1`: Active speech (80 bits / 10 octets)
- `ana[0] = 2`: SID update (15 bits / 2 octets)

**State transitions:**
1. **VOICE→first NOISE:** Immediately emit SID frame (ana[0]=2)
2. **Subsequent NOISE frames:** Increment count_fr0
   - If count_fr0 < FR_SID_MIN (3): no transmission (ana[0]=0)
   - Else: check stationarity via Itakura distance (thresholds: FRAC_THRESH1=4855, FRAC_THRESH2=3161) and energy change (>2 dB)
   - If non-stationary OR energy changed: emit SID (ana[0]=2), reset count_fr0
   - Else: no transmission (ana[0]=0)
3. **NOISE→VOICE:** Resume normal encoding (ana[0]=1)

**Filter averaging for SID:**
- Maintain NB_SUMACF=3 past autocorrelation sets (each covering NB_CURACF=2 frames = 6-frame history)
- Compare current 2-frame averaged filter with past 6-frame average
- If stationary: use averaged filter for SID frame
- Else: use current filter

**Autocorrelation accumulation (`Update_cng`):** The `Update_cng(rh_nbe, exp_R0, Vad)` function runs **unconditionally** after each frame's VAD decision (both VOICE and NOISE frames), accumulating autocorrelation data for SID filter averaging. It is called after the `vad()` call but before the VAD conditional processing (`cod_ld8a.c:254`). This ensures that autocorrelation history is continuously maintained regardless of the VAD decision, enabling accurate SID filter estimation when the encoder transitions to silence.

### 8.3 Comfort Noise Generation (CNG)

**Per subframe (decoder-side `Calc_exc_rand()`):**

1. Generate random pitch delay T in [40, 103] with fractional part {0, 1, 2}
2. Generate adaptive codebook vector via `Pred_lt_3(exc, T, frac, L_SUBFR)`
3. Generate random adaptive gain Gp in [0, 0.5) (Q14): `Gp = Random(&seed) & 0x1FFF` (comment in `calcexc.c:105`: "< 0.5 Q14"); convert to Q15 for subsequent use: `Gp2 = shl(Gp, 1)`
4. Generate Gaussian excitation (Central Limit Theorem approximation via `Gauss()` in `calcexc.c`):
   - `Gauss(seed)`: sum 12 `Random(&seed)` calls, right-shift result by 7 → approximate Gaussian sample
   - **Normalize using `FRAC1 = 19043` fixed-point pipeline** (`calcexc.c:125-141`): compute `fact = mult_r(cur_gain, FRAC1)` where `FRAC1 = 19043` encodes `(sqrt(L_SUBFR) * alpha / 2 - 1) * 32768` with `alpha=0.5`. Compute Gaussian energy `L_acc = sum(excg[i]^2)`, normalize via `Sqrt()` which returns `sqrt(Num/2)` (not `sqrt(Num)`), then scale each `excg[i]` by `fact / Sqrt(L_acc)`. The factor-of-2 in `Sqrt()` is compensated by the `FRAC1` constant. An implementer using a standard square root would produce incorrect CNG excitation energy
5. Generate 4 random ACELP pulses at random positions on 5-sample tracks
6. **Stage 1 — ACELP-like excitation with quadratic gain solve:**
   - Compose preliminary excitation: `ex1[i] = Gp2*adaptive[i] + Gaussian[i]` (`calcexc.c:150-153`)
   - Rescale `ex1` to `excs` (right-shift by `sh = max(0, 3-norm_s(max))` for overflow avoidance)
   - Compute interaction term `b` = signed sum of `excs[]` at the 4 pulse positions
   - Solve quadratic for fixed codebook gain Gf: `4×Gf² + 2b×Gf + c = 0`, where `c = Gp²×Ea² - K0×cur_gain²`, `K0 = 24576` (= 1 - α² with α=0.5, in Q15)
   - Select the root with the **lowest absolute value** for Gf (Annex B spec B.4.4)
   - **Discriminant-negative fallback** (`calcexc.c:208-232`): when `b² - 4c < 0` (discriminant negative), the adaptive excitation contribution is abandoned: `cur_exc[i]` is overwritten with the Gaussian excitation `excg[i]`, `Gp` is set to 0, the interaction term `b` is recomputed from `excg[]` at the pulse positions, and a second quadratic is solved using `K0 * cur_gain²` as the target energy (equation: `delta = K0*k + b²`, always non-negative). This fallback occurs when the adaptive+Gaussian energy exceeds the target energy budget
   - Cap fixed codebook gain: `if |Gf| > G_MAX (=5000), Gf = ±G_MAX` (bilateral clamp applied inside `Calc_exc_rand`, `calcexc.c:240-245`)
7. **Stage 2 — Final excitation composition:** Add the signed ACELP pulses at gain Gf to `cur_exc[]`: `cur_exc[pos[i]] += sign[i] * Gf` (`calcexc.c:248-256`). The final excitation per subframe is `cur_exc[i]` which already contains the Gp-weighted adaptive + Gaussian mixture from Stage 1, plus the ACELP pulse contribution from this step. This implements the two-stage mixture of Annex B spec B.4.4 (equations B.20-B.26): the `alpha`/`beta` energy partition is embedded in the K0 constant and the Gaussian normalization via FRAC1

**CNG gain smoothing:**
- On first NOISE frame after VOICE: `cur_gain = sid_gain` (step)
- Subsequent frames: `cur_gain = mult_r(A_GAIN0, cur_gain) + mult_r(A_GAIN1, sid_gain)` (Q15)
  - `A_GAIN0 = 28672` (0.875), `A_GAIN1 = 4096` (0.125)
  - Both multiplications use `mult_r` (rounding multiply, `dec_sid.c:109-110`), not `mult`. Using `mult` instead of `mult_r` produces off-by-one errors in the gain value, causing bit-exact failures

**CNG random seed:** INIT_SEED = 11111 (separate from decoder frame erasure seed = 21845).

**Seed reset on active speech frames (decoder):** The decoder resets the CNG seed to `INIT_SEED` on **every active speech frame** (ftyp == 1), not just on voice→noise transitions:
```
// dec_ld8a.c:197 (inside the ftyp==1 branch)
seed = INIT_SEED;
```

**Seed reset on active speech frames (encoder):** The encoder also resets the CNG seed to `INIT_SEED` on **every active speech frame**, mirroring the decoder behavior:
```
// cod_ld8a.c:312 (inside the active-speech branch, after *ana++ = 1)
seed = INIT_SEED;
```
Both resets ensure deterministic CNG generation after each speech segment, regardless of how many speech frames occurred. Without the encoder-side reset, the encoder CNG excitation after speech would start from whatever seed value was left over from the previous DTX period, producing different output that fails Annex B encoder conformance tests (tstseq1-4).

**Sharp reset during CNG:** When processing non-active frames (ftyp != 1), the `sharp` variable is reset to `SHARPMIN` (3277, Q14 = 0.2):
```
// dec_ld8a.c:191
sharp = SHARPMIN;
```
This prevents stale pitch sharpening values from the last speech frame from affecting CNG excitation generation.

### 8.4 Annex B Decoder Integration Details

This section collects implementation details for integrating Annex B (DTX/CNG) with the Annex A decoder that are critical for bit-exactness.

#### 8.4.1 Noise LSF MA Predictor Table Derivation (`noise_fg`)

The SID frame LSP decoder uses separate `noise_fg` MA predictor tables, derived from the speech MA predictor tables `fg` during initialization (`Init_lsfq_noise()` in `dec_sid.c`):

```
noise_fg[0][i][j] = fg[0][i][j]                                        // Mode 0: direct copy
noise_fg[1][i][j] = (19660 × fg[0][i][j] + 13107 × fg[1][i][j]) >> 15 // Mode 1: weighted blend
```

The Q15 weights are:
- 19660 = 0.6 in Q15
- 13107 = 0.4 in Q15

This weighted blend of modes 0 and 1 of the speech predictor forms mode 1 of the noise predictor. The exact fixed-point computation must be used for SID frame LSP decoding to be bit-exact.

#### 8.4.2 Decoder Frame Processing Flow with DTX

The decoder's main loop checks `ftyp` (from `parm[1]`) to route processing:

```
if (ftyp == 1) {          // Active speech frame
    seed = INIT_SEED;      // Reset CNG seed
    // Normal speech decoding (Sections 5.2-5.8)
}
else {                     // SID or untransmitted frame
    sharp = SHARPMIN;      // Reset pitch sharpening
    // CNG processing (Section 8.3)
}
```

The `past_ftyp` variable tracks the previous frame's type for transition detection and is initialized to 1 (speech) at decoder startup (see Section 2.2.1). This means if the very first frame received is a SID or untransmitted frame, it will be treated as a transition from speech→noise, triggering `cur_gain = sid_gain` (step change, not smoothed).

---

## 9. Codebook Tables (ROM Data)

The implementation SHALL include the following constant tables, derived from the ITU-T reference code:

### 9.1 LP Analysis Tables
- Asymmetric analysis window coefficients (240 values)
- Lag window coefficients (12 values, M+2 entries). All 12 entries are actively used: `Lag_window(m, ...)` accesses `lag_h[i-1]` for `i=1..m`. When Annex B is active and `m=NP=12`, all entries `lag_h[0..11]` are used for lag coefficients 1-12. See SPECIFICATION_PLAN.md erratum E6

### 9.2 LSP Tables
- Chebyshev polynomial grid points (GRID_POINTS = 50)
- LSP VQ codebook L1: 128 x 10 values
- LSP VQ codebook L2: 32 x 5 values
- LSP VQ codebook L3: 32 x 5 values
- MA prediction coefficients: 2 modes x 4 orders x 10 dimensions
- LSP mean values: 10 values

### 9.3 Pitch Tables
- Sinc interpolation filter coefficients (3 phases x filter_length)
- Open-loop pitch search interpolation filter
- `tab_zone[PIT_MAX+L_INTERPOL-1]` (153 values): Taming zone lookup table mapping pitch delay to zone index 0-3 (used by `taming.c` `update_exc_err()` and `test_err()`; see §3.12.3)

### 9.4 Gain Tables
- Gain codebook stage 1 (GA): 8 x 2 values (g_p, g_c correction)
- Gain codebook stage 2 (GB): 16 x 2 values
- Gain MA prediction coefficients: [0.68, 0.58, 0.34, 0.19] → Q13: {5571, 4751, 2785, 1556}
- Mean energy constant: `INV_COEF = -17103` (Q19)
- Initial `past_qua_en[4]` = {-14336, -14336, -14336, -14336} (Q10)

### 9.5 Post-Filter Tables
- Formant post-filter gamma_n powers (10 values, Q15)
- Formant post-filter gamma_d powers (10 values, Q15)
- AGC coefficient

### 9.6 Annex B Tables
- VAD decision boundaries
- SID codebook parameters

---

## 10. Fixed-Point Arithmetic

### 10.1 Number Formats

| Format | Description | Range |
|--------|-------------|-------|
| Q15 | 1 sign + 15 fractional bits | [-1.0, +0.999969] |
| Q14 | 1 sign + 1 integer + 14 fractional | [-2.0, +1.999939] |
| Q13 | Used for LSF values | |
| Q12 | 1 sign + 3 integer + 12 fractional | |
| Q0 | Integer | [-32768, +32767] |
| Q31 | 32-bit: 1 sign + 31 fractional | |

### 10.2 Basic Operations

The implementation SHALL provide the following saturating arithmetic operations:

| Operation | Description |
|-----------|-------------|
| `add(a, b)` | 16-bit addition with saturation |
| `sub(a, b)` | 16-bit subtraction with saturation |
| `mult(a, b)` | Q15 fractional multiply -> Q15 |
| `mult_r(a, b)` | Q15 fractional multiply with rounding |
| `L_mult(a, b)` | 16x16 -> 32-bit (Q15*Q15 -> Q31) |
| `L_mac(acc, a, b)` | 32-bit MAC: acc + a*b |
| `L_msu(acc, a, b)` | 32-bit MSU: acc - a*b |
| `L_add(a, b)` | 32-bit addition with saturation |
| `L_sub(a, b)` | 32-bit subtraction with saturation |
| `shr(a, n)` | 16-bit arithmetic right shift |
| `shl(a, n)` | 16-bit left shift with saturation |
| `L_shr(a, n)` | 32-bit arithmetic right shift |
| `L_shl(a, n)` | 32-bit left shift with saturation |
| `norm_s(a)` | Find normalization shift for 16-bit |
| `norm_l(a)` | Find normalization shift for 32-bit |
| `round(L)` | Round 32-bit to 16-bit (add 0x8000, take upper 16) |
| `extract_h(L)` | Extract upper 16 bits of 32-bit |
| `extract_l(L)` | Extract lower 16 bits of 32-bit |
| `L_deposit_h(a)` | Deposit 16-bit into upper 16 of 32-bit (`a << 16`) |
| `L_deposit_l(a)` | Deposit 16-bit into lower 16 of 32-bit (sign-extend to 32-bit) |
| `negate(a)` | 16-bit negation with saturation |
| `abs_s(a)` | 16-bit absolute value |
| `shr_r(a, n)` | 16-bit arithmetic right shift with rounding |
| `L_shr_r(a, n)` | 32-bit arithmetic right shift with rounding |
| `L_negate(a)` | 32-bit negation with saturation |
| `L_abs(a)` | 32-bit absolute value with saturation |
| `L_sat(a)` | 32-bit saturation (clamp to [MIN_32, MAX_32]; no callers in g729ab_v14 reference code — included for completeness) |
| `saturate(a)` | 32-bit to 16-bit saturation clamp (internal helper called by `add`, `sub`, `mult`, etc.) |

### 10.3 Saturation Bounds

- 16-bit: [-32768, +32767]
- 32-bit: [-2147483648, +2147483647]
- All arithmetic operations MUST saturate (no wraparound)

### 10.4 Implementation Strategy

The Rust implementation SHALL:
1. Implement all basic operators as inline functions with `#[inline(always)]`
2. Use Rust's `i16` and `i32` types for Word16 and Word32
3. Implement saturation explicitly using `.saturating_add()`, `.saturating_sub()`, etc. where appropriate
4. Fixed-point only. No floating-point path — bit-exactness with the ITU reference C code is the sole arithmetic target.
5. Use `#[cfg(test)]` to verify fixed-point operations against known test values

### 10.5 Global Overflow Flag

The ITU-T fixed-point arithmetic model includes a global `Overflow` flag that is critical for bit-exact behavior. The reference code declares this in `BASIC_OP.C:32`:

```c
Flag Overflow = 0;   // Global variable
```

**Mechanism:**
- Operations that can overflow (`shl()`, `L_mac()`, `L_shl()`, and others) **set** `Overflow = 1` when saturation occurs
- Calling code **reads** `Overflow` to detect if saturation happened, then **clears** it to 0 before the next critical section
- This is NOT the same as saturation clamping (which always happens) — it's an out-of-band signal that saturation occurred

**Where the Overflow flag is checked (bit-exact behavior depends on it):**

| Location | Section | Behavior |
|---|---|---|
| Autocorrelation (LPC.C:48-62) | 3.2.2 | Overflow-retry loop: right-shifts windowed signal and retries until no overflow |
| Pitch analysis (PITCH_A.C:55-69) | 3.7 | Triggers signal rescaling by >>3 before correlation |
| Decoder synthesis (DEC_LD8A.C:169-181,331-344) | 5.8.1 | Triggers excitation >>2 and re-synthesis |

**Rust implementation:** In the reference C code, `Overflow` is a global variable. For thread safety in Rust, implement it as a per-instance field on the encoder/decoder state structs, or as a parameter passed through the call chain. It must be accessible to all basic operations that can saturate and readable by the calling code. A `Cell<bool>` or simple `bool` field is sufficient.

---

## 11. SIP Platform Integration Requirements

### 11.1 RTP Packetization

| Parameter | Value |
|-----------|-------|
| RTP payload type | 18 (static assignment per RFC 3551) |
| Clock rate | 8000 Hz |
| Default packetization | 20 ms (2 frames, 20 octets) |
| Supported packetization | 10-240 ms (1-24 frames) |
| Annex B SID frame | 2 octets |
| Silence (no transmission) | 0 octets |

### 11.2 SDP Negotiation

```
m=audio 49170 RTP/AVP 18
a=rtpmap:18 G729/8000
a=fmtp:18 annexb=yes
```

### 11.3 API Requirements

The codec SHALL expose both buffer-based and `Result`-returning interfaces:

```rust
// Encoder
pub struct G729Encoder { /* ... */ }

impl G729Encoder {
    pub fn new(config: EncoderConfig) -> Self;
    pub fn encode(&mut self, pcm: &[i16; 80], output: &mut [u8; 10]) -> FrameType;
    pub fn encode_frame(&mut self, pcm: &[i16]) -> Result<[u8; FRAME_BYTES], Error>;
    pub fn reset(&mut self);
}

// Decoder
pub struct G729Decoder { /* ... */ }

impl G729Decoder {
    pub fn new(config: DecoderConfig) -> Self;
    pub fn decode(&mut self, bitstream: &[u8], output: &mut [i16; 80]);
    pub fn decode_with_type(&mut self, bitstream: &[u8], frame_type: FrameType, output: &mut [i16; 80]);
    pub fn decode_frame(&mut self, data: &[u8]) -> Result<[i16; FRAME_SAMPLES], Error>;
    pub fn decode_erasure(&mut self, output: &mut [i16; 80]);
    pub fn reset(&mut self);
}

// Configuration
// Note: Annex A is always enabled — this is a G.729AB-only implementation.
pub struct EncoderConfig {
    pub annex_b: bool,      // Enable VAD/DTX/CNG (default: true)
}

pub struct DecoderConfig {
    pub annex_b: bool,      // Enable CNG (default: true)
    pub post_filter: bool,  // Enable adaptive post-filter
}

pub enum FrameType {
    Speech,     // 10 octets
    Sid,        // 2 octets (Annex B)
    NoData,     // 0 octets (Annex B silence)
}
```

**Frame type inference design decision:** The primary `decode()` method infers frame type from bitstream length: 10 bytes = Speech, 2 bytes = SID, 0 bytes = NoData. This covers standard RTP payloads (RFC 3551). The alternative `decode_with_type()` accepts an explicit `FrameType` parameter for transport layers that provide frame type metadata alongside the data (e.g., the ITU reference code's `read_frame()` which produces both bitstream data and a frame type field). Both methods are equivalent when the inferred and explicit types agree; `decode_with_type()` takes precedence when they differ, enabling scenarios like forcing a frame erasure on a corrupted speech frame.

### 11.4 CLI Tool

For validation and batch processing:

```
g729-enc <in.pcm> <out.g729>
g729-dec <in.g729> <out.pcm>
```

### 11.5 Performance Requirements

- Encode latency: < 2 ms per frame on modern x86_64
- Decode latency: < 1 ms per frame on modern x86_64
- Memory: < 64 KB per encoder/decoder instance
- Thread safety: Each encoder/decoder instance is `Send` but not necessarily `Sync`
- No heap allocation during encode/decode (after initialization)

### 11.6 Non-Functional Requirements

- **Safety:** Minimal use of `unsafe` blocks. Any `unsafe` usage must be documented and justified (e.g., for SIMD optimizations).
- **Portability:** `#![no_std]` compatible for embedded and bare-metal targets. No OS-specific dependencies for the core library. Rust stable toolchain only (no nightly features).
- **Linkage:** No external C libraries required. The implementation must be fully self-contained in pure Rust.

### 11.7 Packet Loss Handling

- Support BFI (Bad Frame Indicator) input to decoder
- Implement full frame erasure concealment per Section 6
- Support configurable maximum consecutive erasures before muting

---

## 12. Key Constants

| Constant | Symbol | Value |
|----------|--------|-------|
| Sampling rate | fs | 8000 Hz |
| Frame size | L_FRAME | 80 samples |
| Subframe size | L_SUBFR | 40 samples |
| LP order | M | 10 |
| Analysis window | L_WINDOW | 240 samples |
| Look-ahead | L_NEXT | 40 samples |
| Min pitch delay | PIT_MIN | 20 samples |
| Max pitch delay | PIT_MAX | 143 samples |
| Upsampling factor | UP_SAMP | 3 |
| Interpolation length | L_INTERPOL | 11 |
| LSP grid points | GRID_POINTS | 50 (51 with endpoints) |
| MA prediction order | MA_NP | 4 |
| MA modes | MODE | 2 |
| 1st stage LSP codebook | NC0 | 128 entries (7 bits) |
| 2nd stage LSP codebook | NC1 | 32 entries (5 bits) |
| Gain codebook stage 1 | NCODE1 | 8 entries (3 bits) |
| Gain codebook stage 2 | NCODE2 | 16 entries (4 bits) |
| Track step | STEP | 5 |
| Positions per track (0-2) | NB_POS | 8 |
| Positions track 3 | NB_POS+8 | 16 |
| Bandwidth expansion | BW_EXPAND | 60 Hz |
| White noise correction | WNC | 1.0001 |
| LSP min spacing (pass 1) | | 0.0012 |
| LSP min spacing (pass 2) | | 0.0006 |
| Pitch gain upper bound | | 1.2 |
| Gain MA coefficients | | [0.68, 0.58, 0.34, 0.19] |
| Post-filter inv gamma | INV_GAMMAP | 21845 (1/(1+GAMMAP) Q15) |
| Post-filter gamma ratio | GAMMAP_2 | 10923 (GAMMAP/(1+GAMMAP) Q15) |
| Post-filter impulse resp len | L_H | 22 |
| SID decoder initial LSP | lspSid_init | {31441, 27566, 21458, 13612, 4663, -4663, -13612, -21458, -27566, -31441} (Q15) |

> **Note:** The SID decoder uses a **different** initial LSP vector than the speech decoder ({30000, 26000, ...}).

### 12.1 Annex B Constants (VAD / DTX / CNG)

| Constant | Symbol | Value | Q-format | Source |
|----------|--------|-------|----------|--------|
| CNG Gaussian normalization | FRAC1 | 19043 | Q15 | `calcexc.c` |
| CNG/VAD initial seed | INIT_SEED | 11111 | integer | `cod_ld8a.c:158` |
| VAD initialization period | INIT_FRAME | 32 | integer | `vad.c` |
| Spectral distortion threshold | K0 | 24576 | Q15 | `dtx.h` |
| Encoder CNG flag | FLAG_COD | 1 | integer | `calcexc.c` |
| Decoder CNG flag | FLAG_DEC | 0 | integer | `calcexc.c` |
| CNG gain upper bound | G_MAX | 5000 | Q0 | `calcexc.c` |
| CNG gain smoothing (current) | A_GAIN0 | 28672 | Q15 (~0.875) | `dtx.h` |
| CNG gain smoothing (previous) | A_GAIN1 | 4096 | Q15 (= 32768 − A_GAIN0) | `dtx.h` |
| Taming zone table size | tab_zone size | 153 (= PIT_MAX + L_INTERPOL − 1) | — | `taming.c` |

### 12.2 Post-Filter Pitch Enhancement Constants

| Constant | Symbol | Value | Q-format | Source |
|----------|--------|-------|----------|--------|
| Pitch post-filter gamma | GAMMAP | 16384 | Q15 (0.5) | `postfilt.c` |
| Inverse pitch PF factor | INV_GAMMAP | 21845 | Q15 (1/(1+GAMMAP)) | `postfilt.c` |
| Pitch PF composite factor | GAMMAP_2 | 10923 | Q15 (GAMMAP/(1+GAMMAP)) | `postfilt.c` |

### 12.3 VAD Thresholds (from ITU-T G.729 Annex B, Table B.1)

The VAD uses threshold constants for spectral distortion, full-band energy, low-band energy, and zero-crossing rate. These are defined in `vad.c` and correspond to Table B.1 of the Annex B specification. The exact values are embedded in the `MakeDec()` function's 14-condition decision tree (see `vad.c:380-440`).

---

## 13. Conformance and Testing

### 13.1 Bit-Exactness

The implementation MUST produce bit-exact output matching the ITU-T reference C code for all official test vectors. This is the primary compliance criterion.

### 13.2 Test Vector Categories

All test vectors target the Annex A and Annex B algorithms:

1. **Annex A encoder test vectors:** PCM input -> expected bitstream output (from `G729_Release3/g729AnnexA/test_vectors/`)
2. **Annex A decoder test vectors:** Bitstream input -> expected PCM output
3. **Frame erasure test vectors:** Bitstream + BFI patterns -> expected output
4. **Homing frame tests:** Verify state reset on homing sequence
5. **Annex B test vectors:** VAD/DTX/CNG verification (from `G729_Release3/g729AnnexB/test_vectors/` and `g729_annex_b_test_vectors/`)

### 13.3 ITU-T Test Vector Source

Test vectors are distributed with the official G.729 Software Package Release 3 (2012), available in `reference/itu_reference_code/G729_Release3/`. Each annex subdirectory contains its own `test_vectors/` directory with the complete validation suite.

### 13.4 Test Vector Inventory

**Annex A test vectors** (from `G729_Release3/g729AnnexA/test_vectors/`):

| Test | Purpose | Encoder Input | Bitstream | Decoder Output |
|------|---------|--------------|-----------|----------------|
| SPEECH | General coverage | .in | .bit | .pst |
| PITCH | Pitch search | .in | .bit | .pst |
| LSP | LSP quantization | .in | .bit | .pst |
| FIXED | Fixed codebook | .in | .bit | .pst |
| TAME | Taming procedure | .in | .bit | .pst |
| ALGTHM | Conditional branches | .in | .bit | .pst |
| ERASURE | Frame erasure (decoder-only) | — | .bit | .pst |
| OVERFLOW | Synthesis overflow (decoder-only) | — | .bit | .pst |
| PARITY | Parity check (decoder-only) | — | .bit | .pst |
| TEST | Additional general coverage (unlisted in READMETV.txt) | .in | .bit | .pst |

**Annex B test vectors** (from `G729_Release3/g729AnnexB/test_vectors/` and `g729_annex_b_test_vectors/`):

| Test | Purpose | Input | Output |
|------|---------|-------|--------|
| tstseq1.bin → tstseq1a.bit | Encoder test 1 | PCM | Bitstream |
| tstseq2.bin → tstseq2a.bit | Encoder test 2 | PCM | Bitstream |
| tstseq3.bin → tstseq3a.bit | Encoder test 3 | PCM | Bitstream |
| tstseq4.bin → tstseq4a.bit | Encoder test 4 | PCM | Bitstream |
| tstseq1a.bit → tstseq1a.out | Decoder test 1 | Bitstream | PCM |
| tstseq2a.bit → tstseq2a.out | Decoder test 2 | Bitstream | PCM |
| tstseq3a.bit → tstseq3a.out | Decoder test 3 | Bitstream | PCM |
| tstseq4a.bit → tstseq4a.out | Decoder test 4 | Bitstream | PCM |
| tstseq5.bit → tstseq5a.out | Decoder-only test 5 | Bitstream | PCM |
| tstseq6.bit → tstseq6a.out | Decoder-only test 6 | Bitstream | PCM |

**File formats:**
- PCM files (`.in`, `.bin`): 16-bit signed, little-endian (Intel byte order), 8 kHz mono
- Bitstream files (`.bit`): ITU serial format (SYNC_WORD + SIZE_WORD + N × 16-bit words per frame)
- Decoder output files (`.pst`, `.out`): Same format as PCM input (16-bit signed LE)

### 13.5 Test Vector Limitations

From the ITU-T READMETV.TXT:

> "NOTE that these vectors are not part of a validation procedure. It is very difficult to design an exhaustive set of test vectors. Hence passing these vectors should be viewed as a minimum requirement, and is not a guarantee that the implementation is correct for every possible input signal."

The implementation should supplement ITU test vectors with additional testing:
- Real-world speech recordings (various speakers, languages)
- Silence→speech and speech→silence transitions
- Tandem encoding (encode→decode→encode→decode)
- Interoperability testing with other G.729 implementations (e.g., bcg729). Note: bcg729 encoder output will differ from ITU reference encoder output due to a different fixed codebook search algorithm (~576 nested-loop candidate evaluations vs 320 depth-first tree candidate evaluations). Interoperability testing should focus on: (a) decoding bcg729-encoded bitstreams with our decoder, (b) decoding our encoder output with bcg729's decoder. Cross-encoder bitstream comparison is not expected to match. **Decoder output comparison for identical bitstream input will differ for frames that trigger synthesis overflow**, because bcg729 uses saturation-only for synthesis overflow (per-sample clamping to MAXINT16) instead of the ITU reference's retry-with-scaling approach (detect Overflow flag, scale excitation >>2, re-synthesize). Verified constants that match between bcg729 and ITU: post-filter constants (GAMMA2_PST=18022/0.55, GAMMA1_PST=22938/0.70, MU=26214/0.8), VAD initialization (INIT_FRAME=32), CNG gain smoothing (A_GAIN0=28672/0.875, A_GAIN1=4096/0.125), and DTX autocorrelation history (2 current + 6 past frames). bcg729 cross-validation is useful for structural verification and debugging; decoder output should match for all frames that do not trigger synthesis overflow. **Additional bcg729 feature:** bcg729 implements RFC 3389 comfort noise payload interworking (`bcg729GetRFC3389Payload()`, decoder RFC 3389 flag, and `cng.c` lines 247-313 for RFC 3389 CN payload decoding) beyond the ITU specification's native Annex B SID frames. This does not affect bit-exact conformance against ITU reference test vectors, but interop testing should be aware that bcg729 can operate in RFC 3389 CN mode as an alternative to native SID frames. **Additional bcg729 divergence:** bcg729 omits VAD smoothing stage 4 (forced-NOISE override; bcg729 `vad.c:340` explicitly notes this). bcg729 VAD decisions will differ from the ITU reference for frames where stage 4 would trigger; if bcg729 is used as a secondary VAD reference during debugging, this omission could cause false-alarm discrepancies.
- Sustained high-energy input (to exercise overflow handling)
- Long-duration sessions (to catch state accumulation bugs)

### 13.6 Testing Strategy

1. Implement basic operator tests (verify saturation, rounding)
2. Unit test each processing block against intermediate reference values
3. Full encoder/decoder bit-exact tests against ITU-T test vectors
4. Round-trip tests (encode -> decode) with quality measurement
5. Stress tests: silence, tones, noise, music, transitions
6. Packet loss simulation with various erasure patterns
7. Performance benchmarks

---

## 14. Rust Crate Structure

```
g729/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API
│   ├── encoder.rs          # Encoder top-level
│   ├── decoder.rs          # Decoder top-level
│   ├── basic_ops.rs        # Fixed-point arithmetic primitives
│   ├── pre_process.rs      # High-pass filter + scaling
│   ├── lpc.rs              # LP analysis (window, autocorrelation, Levinson-Durbin)
│   ├── lsp.rs              # LP-to-LSP, LSP-to-LP (Chebyshev)
│   ├── lsp_quantize.rs     # LSP VQ (encoder)
│   ├── lsp_decode.rs       # LSP decoding (decoder)
│   ├── pitch.rs            # Open-loop + closed-loop pitch analysis
│   ├── acelp.rs            # Algebraic codebook (search + decode)
│   ├── gains.rs            # Gain quantization + decoding + prediction
│   ├── post_filter.rs      # Adaptive post-filter (long-term, short-term, tilt, AGC)
│   ├── post_process.rs     # High-pass filter + upscaling
│   ├── bitstream.rs        # Bit packing/unpacking
│   ├── vad.rs              # Voice Activity Detection (Annex B)
│   ├── dtx.rs              # Discontinuous Transmission (Annex B)
│   ├── cng.rs              # Comfort Noise Generation (Annex B)
│   ├── erasure.rs          # Frame erasure concealment
│   ├── taming.rs           # Taming procedure
│   ├── filter.rs           # Common filter operations
│   ├── tables.rs           # All codebook tables and constants
│   └── tests/
│       ├── basic_ops_tests.rs
│       ├── encoder_tests.rs
│       ├── decoder_tests.rs
│       ├── conformance_tests.rs
│       └── test_vectors/    # ITU-T test vector files
├── benches/
│   └── codec_bench.rs      # Performance benchmarks
└── examples/
    ├── encode_file.rs       # CLI encoder
    └── decode_file.rs       # CLI decoder
```

---

## 15. Implementation Phases

All phases implement the Annex A algorithms directly. There is no separate "add Annex A" phase.

### Phase 1: Foundation
- Fixed-point arithmetic library (basic_ops.rs)
- Common filter operations
- Codebook tables and constants (from Annex A reference code: `TAB_LD8A.C`)

### Phase 2: Decoder (simpler, testable first)
- Bitstream unpacking
- LSP decoding + interpolation + LSP-to-LP conversion
- Adaptive codebook reconstruction
- Fixed codebook reconstruction
- Gain decoding
- LP synthesis filter
- Basic output (without post-filter)

### Phase 3: Decoder Post-Processing
- Long-term post-filter (integer pitch delays per Annex A)
- Short-term post-filter
- Tilt compensation
- AGC
- High-pass filter + upscaling
- Frame erasure concealment

### Phase 4: Encoder
- Pre-processing
- LP analysis (window, autocorrelation, Levinson-Durbin)
- LP-to-LSP conversion
- LSP quantization
- Perceptual weighting (fixed gamma = 0.75 per Annex A)
- Open-loop pitch analysis (decimated even-sample search per Annex A `Pitch_ol_fast`)
- Target signal computation
- Adaptive codebook search (correlation-only per Annex A)
- Fixed codebook search (depth-first tree search per Annex A)
- Gain quantization
- Memory update
- Bitstream packing

### Phase 5: Annex B (VAD/DTX/CNG)
- Voice Activity Detection
- SID frame encoding/decoding
- Comfort Noise Generation
- DTX state machine

### Phase 6: Conformance and Optimization
- Bit-exact verification against ITU-T Annex A and Annex B test vectors
- Performance optimization (SIMD where beneficial)
- API finalization

---

## 16. Open Questions

1. **V1 target scope:** Confirm v1 target is G.729A + Annex B only (no base G.729). *Resolution: Confirmed. This is a G.729AB-only implementation. Base G.729 is explicitly a non-goal (Section 1.4).*
2. **CLI in v1:** Is a CLI required in v1, or library-only? *Resolution: CLI is optional but preferred for validation (see Section 11.4).*
3. **Fixed-point vs floating-point:** Should we target fixed-point bit exactness or allow floating-point implementations? *Resolution: Fixed-point only. Bit-exactness with ITU-T reference C code is the sole arithmetic target. No floating-point path.*
4. **Reference materials:** Can you provide the G.729 (2012) Recommendation PDF and the reference C code/test vectors? *Resolution: All materials obtained and available in `reference/` (see Local Reference Materials above).*

---

## Appendix: Verified Constants from Reference Code (LD8A.H / TAB_LD8A.C)

These values are confirmed from the actual ITU-T reference source code:

```
// Core frame parameters
L_TOTAL      = 240     // Total speech buffer size
L_WINDOW     = 240     // LP analysis window length
L_NEXT       = 40      // Look-ahead samples
L_FRAME      = 80      // Frame size (samples)
L_SUBFR      = 40      // Subframe size (samples)
M            = 10      // LP filter order
MP1          = 11      // LP order + 1
PIT_MIN      = 20      // Minimum pitch lag
PIT_MAX      = 143     // Maximum pitch lag
L_INTERPOL   = 11      // Interpolation filter length (10+1)
GAMMA1       = 24576   // 0.75 Q15 (Annex A fixed weighting)

// Bitstream
PRM_SIZE     = 11      // Parameters per frame
SERIAL_SIZE  = 82      // 80 bits + 2 (bfi + frame size)
BIT_0        = 0x007F  // Zero-bit encoding
BIT_1        = 0x0081  // One-bit encoding
SYNC_WORD    = 0x6B21  // Frame sync word
SIZE_WORD    = 80      // Speech bits per frame

// Pitch sharpening bounds
SHARPMAX     = 13017   // 0.8 Q14
SHARPMIN     = 3277    // 0.2 Q14

// Pitch interpolation
UP_SAMP      = 3       // Upsampling factor (1/3 resolution)
L_INTER10    = 10      // Interpolation filter half-length
FIR_SIZE_SYN = 31      // UP_SAMP * L_INTER10 + 1

// LSP quantization
NC           = 5       // M/2
MA_NP        = 4       // MA prediction order
MODE         = 2       // Number of MA modes
NC0_B        = 7       // First stage bits (128 entries)
NC1_B        = 5       // Second stage bits (32 entries)
GRID_POINTS  = 50      // LSP root search grid
L_LIMIT      = 40      // LSP lower limit (Q13: 0.005)
M_LIMIT      = 25681   // LSP upper limit (Q13: 3.135)
GAP1         = 10      // Q13 stability gap
GAP2         = 5       // Q13 stability gap
GAP3         = 321     // Q13 stability gap

// Fixed codebook
DIM_RR       = 616     // Correlation matrix size
NB_POS       = 8       // Positions per pulse (tracks 0-2)
STEP         = 5       // Interleave step
MSIZE        = 64      // Cross-correlation vector size

// Gain quantization
NCODE1_B     = 3       // Stage 1 bits (8 entries)
NCODE2_B     = 4       // Stage 2 bits (16 entries)
NCAN1        = 4       // Pre-selection order, stage 1
NCAN2        = 8       // Pre-selection order, stage 2
INV_COEF     = -17103  // Q19

// Post-filter
L_H          = 22      // Truncated impulse response length
GAMMA2_PST   = 18022   // 0.55 Q15 (numerator)
GAMMA1_PST   = 22938   // 0.70 Q15 (denominator)
MU           = 26214   // 0.8 Q15 (tilt compensation)
AGC_FAC      = 29491   // 0.9 Q15 (AGC factor)
AGC_FAC1     = 3276    // 1-AGC_FAC Q15
GAMMAP       = 16384   // 0.5 Q15
INV_GAMMAP   = 21845   // 1/(1+GAMMAP) Q15
GAMMAP_2     = 10923   // GAMMAP/(1+GAMMAP) Q15

// Taming
GPCLIP       = 15564   // Max pitch gain when taming (Q14)
GP0999       = 16383   // ~0.999 pitch gain limit
L_THRESH_ERR = 983040000  // Taming energy threshold

// Gain MA prediction coefficients (Q13): {0.68, 0.58, 0.34, 0.19}
pred[4]      = {5571, 4751, 2785, 1556}

// Pre-processing filter (140 Hz, coefficients /2)
b140[3]      = {1899, -3798, 1899}    // Q12
a140[3]      = {4096, 7807, -3733}    // Q12
// NOTE: Q12 because division by 2 is folded into the numerator coefficients
// (PRD §3.1: "The division by 2 is incorporated into the numerator coefficients").
// The original Butterworth prototype would be Q13; halving drops one bit of
// fractional precision, yielding Q12.

// Post-processing filter (100 Hz)
b100[3]      = {7699, -15398, 7699}   // Q13
a100[3]      = {8192, 15836, -7667}   // Q13
// NOTE: Q13 because multiplication by 2 (upscaling) is folded into the
// numerator coefficients (PRD §5.9.6: output upscaled by 2). The original
// Butterworth prototype would be Q12; doubling gains one bit of fractional
// precision, yielding Q13.

// Bit allocation per parameter
bitsno[11]   = {8, 10, 8, 1, 13, 4, 7, 5, 13, 4, 7}
// (L0+L1=8, L2+L3=10, P1=8, P0=1, C1=13, S1=4, GA1+GB1=7, P2=5, C2=13, S2=4, GA2+GB2=7)
```

## Appendix: Annex B Constants from Reference Code (VAD.H / DTX.H)

These values are confirmed from the actual ITU-T reference source code (Annex B modules):

```
// VAD constants (vad.h)
NP           = 12       // Increased LPC order for VAD autocorrelation
NOISE        = 0        // VAD decision: noise
VOICE        = 1        // VAD decision: voice
INIT_FRAME   = 32       // Last frame of initialization period (33 frames total: 0-32)
INIT_COUNT   = 20       // Initial update count threshold
ZC_START     = 120      // Zero-crossing window start sample
ZC_END       = 200      // Zero-crossing window end sample

// DTX constants (dtx.h)
FLAG_COD     = 1        // Calc_exc_rand flag: encoder (updates taming)
FLAG_DEC     = 0        // Calc_exc_rand flag: decoder (no taming update)
INIT_SEED    = 11111    // CNG random seed initial value
FR_SID_MIN   = 3        // Minimum frames between SID emissions
NB_SUMACF    = 3        // Number of past autocorrelation sets
NB_CURACF    = 2        // Number of current-frame autocorrelation sets
NB_GAIN      = 2        // Number of gain history entries
FRAC_THRESH1 = 4855     // DTX stationarity threshold 1 (Q15)
FRAC_THRESH2 = 3161     // DTX stationarity threshold 2 (Q15)
A_GAIN0      = 28672    // CNG gain smoothing coefficient (0.875 Q15)
A_GAIN1      = 4096     // CNG gain smoothing complement (0.125 Q15)
FRAC1        = 19043    // (sqrt(40)*alpha/2 - 1) * 32768; used in Calc_exc_rand (calcexc.c:127-128) to normalize Gaussian excitation energy: fact = cur_gain * (1 + FRAC1/32768) ≈ alpha * cur_gain * sqrt(L_SUBFR) / 2
K0           = 24576    // (1 - alpha^2) in Q15, alpha=0.5
G_MAX        = 5000     // Maximum CNG fixed codebook gain

// VAD reflection coefficient thresholds (vad.c)
RC_NOISE_UPD = 24576    // rc threshold for background noise update (0.75 Q15); vad.c:274
RC_FORCED    = 19661    // rc threshold for forced-NOISE override (0.6 Q15); vad.c:271

// DTX array sizing (dtx.h, derived)
SIZ_ACF      = 22       // NB_CURACF * (M+1) = 2 * 11; total autocorrelation buffer
SIZ_SUMACF   = 33       // NB_SUMACF * (M+1) = 3 * 11; total accumulated autocorrelation
MP1          = 11       // M + 1; pastCoeff and RCoeff array dimension

// DTX frame rates (dtx.h)
RATE_8000    = 80       // Full rate (8000 bit/s): 80 bits/frame
RATE_SID     = 15       // SID rate: 15 bits/frame
RATE_0       = 0        // No transmission: 0 bits/frame
```

## Appendix A: References

1. ITU-T Recommendation G.729 (06/2012) - "Coding of speech at 8 kbit/s using conjugate-structure algebraic-code-excited linear prediction (CS-ACELP)"
2. ITU-T Recommendation G.729 Annex A (11/1996) - "Reduced complexity 8 kbit/s CS-ACELP speech codec"
3. ITU-T Recommendation G.729 Annex B (10/1996) - "A silence compression scheme for G.729 optimized for terminals conforming to ITU-T Recommendation V.70"
4. ITU-T Implementers' Guide for G.729 (10/2017) - "Guide for the implementation of ITU-T Recommendations G.729, G.729 Annex A, G.729 Annex B and G.729 Annex C/C+"
5. RFC 3551 - "RTP Profile for Audio and Video Conferences with Minimal Control"
6. RFC 3555 - "MIME Type Registration of RTP Payload Formats"
7. ITU-T G.729 Reference C Code, Software Package Release 3 (2012)
8. bcg729 open-source implementation: https://github.com/BelledonneCommunications/bcg729

## Appendix B: Patent Status

All core G.729, G.729A, and G.729B patents expired by January 1, 2017. The codec is fully royalty-free for all implementations worldwide.
