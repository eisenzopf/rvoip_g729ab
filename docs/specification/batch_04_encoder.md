> Part of [Specification Plan](README.md)

### Batch 4: Phase 6 (Encoder)

#### SPEC_PHASE_06_encoder.md

**Sources:**

| Source | What it provides |
|--------|-----------------|
| [Implementation Plan Phase 6](../implementation/phase_06_encoder.md) | Function list (~40 functions), all 7 test vectors, TDD workflow |
| `reference/.../g729ab_v14/cod_ld8a.c` | Main encoder loop |
| `reference/.../g729ab_v14/qua_lsp.c`, `lspgetq.c` | LSP quantization |
| `reference/.../g729ab_v14/pitch_a.c` | Open-loop + closed-loop pitch, `Enc_lag3` (pitch lag encoding, line 496) |
| `reference/.../g729ab_v14/acelp_ca.c`, `cor_func.c` | ACELP fixed codebook search |
| `reference/.../g729ab_v14/qua_gain.c`, `gainpred.c` | Gain quantization |
| `reference/.../g729ab_v14/taming.c` | Taming procedure |
| `reference/.../g729ab_v14/pre_proc.c` | Pre-processing HP filter |
| `reference/.../g729ab_v14/p_parity.c` | Parity_Pitch |
| `PRD.md` §3 | Encoder requirements |

**Key decisions to document:**

- EncoderState struct: complete field list with exact initial values, including `L_exc_err = {0x00004000, ...}` (Q14 initialized to 1.0, NOT zero), `old_A[M+1]={4096,0,...,0}` and `old_rc[2]={0,0}` (Levinson-Durbin fallback state; `LPC.C`), `mem_w[M]=0` (weighting filter memory), `mem_zero[M]=0` (zero-input response memory), `pastVad=1`, `ppastVad=1` (VAD history for DTX; `COD_LD8A.C`)
- **`Init_Coder_ld8a()` complete action mapping to `EncoderState::new()`**: The following table maps every initialization action in the C reference code to its Rust equivalent. Sub-functions called by `Init_Coder_ld8a()` are expanded inline since their state collapses into `EncoderState` fields.

| # | C action (`Init_Coder_ld8a` + sub-calls) | C source | Rust `EncoderState::new()` field | Value |
|---|-------------------------------------------|----------|----------------------------------|-------|
| 1 | `new_speech = old_speech + L_TOTAL - L_FRAME` | `cod_ld8a.c:129` | Implicit: `new_speech` offset = `L_TOTAL - L_FRAME = 160` into `old_speech` | (pointer, not stored) |
| 2 | `speech = new_speech - L_NEXT` | `cod_ld8a.c:130` | Implicit: `speech` offset = `L_TOTAL - L_FRAME - L_NEXT` into `old_speech` | (pointer, not stored) |
| 3 | `p_window = old_speech + L_TOTAL - L_WINDOW` | `cod_ld8a.c:131` | Implicit: `p_window` offset = `L_TOTAL - L_WINDOW = 0` into `old_speech` | (pointer, not stored) |
| 4 | `wsp = old_wsp + PIT_MAX` | `cod_ld8a.c:135` | Implicit: `wsp` offset = `PIT_MAX = 143` into `old_wsp` | (pointer, not stored) |
| 5 | `exc = old_exc + PIT_MAX + L_INTERPOL` | `cod_ld8a.c:136` | Implicit: `exc` offset = `PIT_MAX + L_INTERPOL = 154` into `old_exc` | (pointer, not stored) |
| 6 | `Set_zero(old_speech, L_TOTAL)` | `cod_ld8a.c:140` | `old_speech: [0i16; L_TOTAL]` | all zeros |
| 7 | `Set_zero(old_exc, PIT_MAX+L_INTERPOL)` | `cod_ld8a.c:141` | `old_exc: [0i16; L_FRAME + PIT_MAX + L_INTERPOL]` | all zeros |
| 8 | `Set_zero(old_wsp, PIT_MAX)` | `cod_ld8a.c:142` | `old_wsp: [0i16; L_FRAME + PIT_MAX]` | all zeros |
| 9 | `Set_zero(mem_w, M)` | `cod_ld8a.c:143` | `mem_w: [0i16; M]` | all zeros |
| 10 | `Set_zero(mem_w0, M)` | `cod_ld8a.c:144` | `mem_w0: [0i16; M]` | all zeros |
| 11 | `Set_zero(mem_zero, M)` | `cod_ld8a.c:145` | `mem_zero: [0i16; M]` | all zeros |
| 12 | `sharp = SHARPMIN` | `cod_ld8a.c:146` | `sharp: SHARPMIN` | 3277 (Q14, 0.2) |
| 13 | (implicit) `lsp_old` static initializer | `cod_ld8a.c:84-85` | `lsp_old: [30000,26000,21000,15000,8000,0,-8000,-15000,-21000,-26000]` | static init |
| 14 | `Copy(lsp_old, lsp_old_q, M)` | `cod_ld8a.c:150` | `lsp_old_q: lsp_old` (copy of `lsp_old` initial values) | same as row 13 |
| 15 | `Lsp_encw_reset()` → `Copy(freq_prev_reset, freq_prev[i], M)` ×4 | `qua_lsp.c:44-52` | `freq_prev: [[freq_prev_reset; M]; MA_NP]` | `freq_prev_reset` ×4 |
| 16 | `Init_exc_err()` → `L_exc_err[i] = 0x00004000L` ×4 | `taming.c:22-27` | `L_exc_err: [0x4000i32; 4]` | 16384 (Q14, 1.0) ×4 |
| 17 | `pastVad = 1` | `cod_ld8a.c:156` | `pastVad: 1` | 1 |
| 18 | `ppastVad = 1` | `cod_ld8a.c:157` | `ppastVad: 1` | 1 |
| 19 | `seed = INIT_SEED` | `cod_ld8a.c:158` | `seed: 11111` | 11111 |
| 20 | `vad_init()` → `Set_zero(MeanLSF, M)` | `vad.c:47` | `MeanLSF: [0i16; M]` | all zeros |
| 21 | `vad_init()` → `MeanSE = MeanSLE = MeanE = MeanSZC = 0` | `vad.c:50-53` | `MeanSE/SLE/E/SZC: 0` | 0 |
| 22 | `vad_init()` → `count_sil = count_update = count_ext = less_count = 0` | `vad.c:54-57` | `count_sil/update/ext: 0`, `less_count: 0` | 0 |
| 23 | `vad_init()` → `flag = 1` | `vad.c:58` | `flag: 1` | 1 |
| 24 | `vad_init()` → `Min = MAX_16` | `vad.c:59` | `Min: 32767` | MAX_16 |
| 25 | (implicit) `vad_init()` does NOT reset `Min_buffer[16]`, `Prev_Min`, `Next_Min`, `prev_energy`, `v_flag` | `vad.c:28-34` | These persist from C zero-init; set to 0 in `EncoderState::new()` | 0 |
| 26 | `Init_lsfq_noise()` — initializes `noise_fg` | `dec_sid.c:131-146` | Computed at init: `noise_fg[0]=fg[0]`, `noise_fg[1][i][j]=round(0.6*fg[0][i][j]+0.4*fg[1][i][j])` | (derived from `fg` table) |
| 27 | (implicit) `old_A[M+1]` — Levinson-Durbin fallback | `lpc.c` | `old_A: [4096,0,0,0,0,0,0,0,0,0,0]` | A(z)=1 in Q12 |
| 28 | (implicit) `old_rc[2]` — reflection coefficients | `lpc.c` | `old_rc: [0i16; 2]` | all zeros |
| 29 | (implicit) pre-processing filter state | `pre_proc.c` | `y1_hi/y1_lo/y2_hi/y2_lo/x0/x1: 0` | all zeros |
| 30 | (implicit) `mem_syn[M]` — synthesis filter memory | `cod_ld8a.c` | `mem_syn: [0i16; M]` | all zeros |
| 31 | (implicit) `past_qua_en` static initializer | `gainpred.c` | `past_qua_en: [-14336i16; 4]` (`past_qua_en_reset`) | −14336 ×4 |

  **Note:** Rows 1-5 are C pointer arithmetic that becomes compile-time constant offsets in Rust. Rows 25, 27-31 are C static/global variables with implicit zero-initialization that have no explicit `Init_Coder_ld8a()` action — they must still be explicitly set in `EncoderState::new()`. Row 30 (`mem_syn`) is used every subframe by `Syn_filt(Aq, &exc[i_subfr], &synth[i_subfr], L_SUBFR, mem_syn, 1)` at `cod_ld8a.c:525` to update the encoder's local synthesis filter memory. Row 31 (`past_qua_en`) is the gain prediction memory shared with the decoder (see decoder init table row 24); it is initialized from `past_qua_en_reset` in `gainpred.c`. Row 26 (`Init_lsfq_noise`) also initializes the decoder-side `noise_fg` copy; the encoder holds its own copy via `Init_Cod_cng()` → `Init_lsfq_noise()`.
- Pre-processing HP filter: b140=[1899,-3798,1899] Q12, a140=[4096,7807,-3733] Q12. Filter state MUST use DPF split high/low precision (y1_hi, y1_lo, y2_hi, y2_lo)
- Open-loop pitch search (`Pitch_ol_fast`): inner correlation loop uses `j+=2` (decimation by 2 on samples), outer lag loop sections 1-2 test every lag, section 3 tests every other lag (`i+=2`) then refines +/-1
- **Pitch energy overflow handling** (`PITCH_A.C:55-69`): The open-loop pitch search accumulates correlation energy via `L_mac` in a loop. If `DspContext.overflow` is set after the energy accumulation, the preprocessed speech signal `scaled_signal[]` is right-shifted by 3 (`shr(signal[i], 3)` for all samples in the analysis window) and the entire correlation search restarts from the beginning with reduced dynamic range. This is the second of three overflow check sites in the codec (alongside autocorrelation retry in Phase 3 `LPC.C:48-62` and decoder synthesis retry in Phase 5 `DEC_LD8A.C:169-181,331-344`). The overflow flag must be cleared before each energy accumulation loop and checked after it completes. Reference: `PITCH_A.C` lines 55-69
- **Weighted speech tilt constant 22938 (0.7 Q15):** The encoder's weighted speech computation for open-loop pitch search (`COD_LD8A.C:345-347`) builds a tilt-compensated filter `Ap1[i] = Ap[i] - 0.7*Ap[i-1]` using the hardcoded constant 22938 (Q15). This value is numerically identical to `GAMMA1_PST` (post-filter denominator gamma=0.70) but serves a completely different purpose (encoder-side weighted speech vs. decoder-side formant post-filter). Define a separate named constant (e.g., `TILT_WSP = 22938`) — do NOT alias to `GAMMA1_PST`
- Closed-loop pitch (`Pitch_fr3_fast`) + optimal pitch gain computation (`G_pitch`): SF1 skips fractional search entirely when open-loop estimate is in range 85-143; SF2 always uses fractional 1/3-sample search regardless. `G_pitch()` computes `g_p = <xn,y1>/<y1,y1>` bounded to [0, 1.2]
- **SF2 closed-loop pitch search range:** `T0_min = T0_SF1 - 5`, `T0_max = T0_min + 9`, clamped to `[PIT_MIN, PIT_MAX]` (asymmetric: 5 below, 4 above SF1's delay). The differential encoding uses `index = (T0 - T0_min) * 3 + T0_frac - 1` (5 bits). This asymmetric range is a specification constraint, not an implementation choice — it must match the decoder's `Dec_lag3()` range exactly. Reference: `PITCH_A.C:370-400`, `DEC_LAG3.C`
- **Impulse response in-place computation** (`COD_LD8A.C:407-409`): The C code uses `h[0]=4096; Set_zero(&h[1],L_SUBFR-1); Syn_filt(Ap, h, h, L_SUBFR, &h[1], 0);` — the `mem` parameter points INTO the output buffer (`&h[1]`), and `update=0`. This works in C because `Syn_filt` reads `mem[0..M-1]` (= `h[1..M]`, all zero at this point) before overwriting `h[0..L_SUBFR-1]`. In Rust, overlapping mutable references to the same array are not allowed. **Recommended Rust translation:** use a separate zero-initialized `mem` array (`let mem = [0i16; M];`) and pass it to `syn_filt()` with `update=false`, instead of replicating the C pointer-into-buffer trick. The output is identical since the initial `mem` values are all zero in both cases
- Pitch sharpening applied to impulse response h(n) BEFORE fixed codebook search, then also to output code vector
- **`sharp` update during active subframes:** After gain quantization in each active subframe, the encoder updates `sharp = shr(gain_pitch, 1)` if `gain_pitch < SHARPMAX`, else `sharp = SHARPMAX`; floor at `SHARPMIN`. This per-subframe update ensures the next subframe's pitch sharpening uses the freshly quantized adaptive codebook gain. Mirrors the decoder's `sharp` update (Phase 5). Reference: `COD_LD8A.C:498-501`
- ACELP search: depth-first 4-pulse search (320 total candidate evaluations: 2 track assignments × 2 depth-first searches × (16 pre-selection + 64 full 4-pulse evaluation)). **Verified** by tracing `D4i40_17_fast()` in `ACELP_CA.C`: outer loop `for(track=3..4)` = 2 iterations; each iteration contains DFS-3 (Phase A: 2 `i0` × 8 `i1` = 16 criterion comparisons; Phase B: 8 `i2` × 8 `i3` = 64 criterion comparisons = 80 total) + DFS-4 (Phase A: 2 `i0` × 8 `i1` = 16; Phase B: 8 `i2` × 8 `i3` = 64 = 80 total) = 160 per track iteration × 2 = 320 total
- Gain quantization: taming constraint when `test_err()` triggers (`tameflag == 1`): candidates with `g_pitch >= GP0999` (16383 Q14, ~0.999) are skipped AND `best_gain[0]` is clipped to `GPCLIP2` during pre-selection AND `g_p <= GPCLIP` (15564 Q14) is enforced. When `tameflag == 0`: no pitch gain ceiling is applied in the codebook search. Reference: `QUA_GAIN.C:138-140` (GPCLIP2 pre-selection), `QUA_GAIN.C:249-281` (GP0999 + GPCLIP search), `QUA_GAIN.C:282-300` (unconstrained search)
- Encoder->decoder round-trip test: encode SPEECH.IN, decode result, compare against SPEECH.PST
- Memory update per subframe: construct excitation `u(n) = g_p*v(n) + g_c*c(n)`, then run `Syn_filt(Aq, &exc[i_subfr], &synth[i_subfr], L_SUBFR, mem_syn, 1)` to produce local synthesis output and update the encoder's synthesis filter memory `mem_syn[M]` (`cod_ld8a.c:525`; `synth_buf[L_FRAME+M]` is a function-local scratch array, NOT persistent state — only `mem_syn` persists across subframes/frames), update `exc_err` via `update_exc_err(gain_pit, T0)`, update filter memory `mem_w0[j] = xn[j] - g_p*y1[j] - g_c*y2[j]` (COD_LD8A.C:504-525)
- **Taming `tab_zone` index boundary handling** (`taming.c`): Both `test_err()` and `update_exc_err()` compute indices into `tab_zone[153]` that can be negative for short pitch delays. The C code has explicit guards: (a) `test_err()` first access clamps `i = max(0, t1 - (L_SUBFR+L_INTER10))` = `max(0, t1 - 50)` before `tab_zone[i]` (`taming.c:48-51`). (b) `test_err()` second access at `taming.c:54-55`: `i = t1 + (L_INTER10 - 2) = t1 + 8`; this has NO explicit guard, but is safe because `t1 >= PIT_MIN = 20` (minimum pitch delay for fractional case) or `t1 >= PIT_MIN - 1 = 19` (integer case with `T0_frac <= 0`), so `i >= 19 + 8 = 27 >= 0`, and `i <= PIT_MAX + 1 + 8 = 152 < 153 = len(tab_zone)`. The Rust implementation should include a `debug_assert!(i >= 0 && i < 153)` for this invariant. (c) `update_exc_err()` branches to an alternate path when `n = T0 - L_SUBFR < 0` (`taming.c:92-109`): uses `L_exc_err[0]` directly through two chained `Mpy_32_16` iterations without any `tab_zone` access. The Rust implementation must replicate the explicit guards and document the implicit safety invariants — without them, any pitch delay below 40 would cause an out-of-bounds panic
- Buffer management at frame end: shift `old_speech`, `old_wsp`, `old_exc` left by L_FRAME=80 samples
- Conformance gate: all 7 Annex A encoder test vectors (including undocumented TEST) must pass bit-exactly
- Static/helper functions for C-to-Rust mapping: `Dot_Product(x,y,L)` in `PITCH_A.C` (energy normalization for open-loop pitch, maps to inline helper in `pitch/open_loop.rs`); `Corr_xy2(xn,y1,y2,g_coeff,exp_g_coeff)` in `COR_FUNC.C` (computes three normalized cross-correlation coefficients for gain quantization; called at `COD_LD8A.C:481`; maps to helper in `gain/quantize.rs`). Full C signature: `void Corr_xy2(Word16 xn[], Word16 y1[], Word16 y2[], Word16 g_coeff[], Word16 exp_g_coeff[])`. Outputs: `g_coeff[2]`/`exp_g_coeff[2]` = `<y2,y2>` (energy of filtered fixed CB vector), `g_coeff[3]`/`exp_g_coeff[3]` = `-2<xn,y2>` (negative double correlation between target and filtered fixed CB), `g_coeff[4]`/`exp_g_coeff[4]` = `2<y1,y2>` (double correlation between filtered adaptive CB and filtered fixed CB). Each output has its own Q-format exponent; the Rust implementation must preserve all three exponents for correct gain quantization; `Gbk_presel(best_gain,cand1,cand2,gcode0)` in `QUA_GAIN.C` (gain codebook pre-selection, maps to helper in `gain/quantize.rs`); `perc_var()` (from base G.729's `PWF.C`, absent in g729ab_v14): **eliminated** — computes adaptive perceptual weighting gammas in base G.729; Annex A uses fixed gamma=0.75 (`GAMMA1=24576` Q15) instead, so `perc_var()` is never called and `PWF.C` does not exist in the combined AB codebase. No Rust mapping needed
- **`Corr_xy2` complete normalization pipeline** (`cor_func.c:26-83`): (1) Scale `y2[]` from Q12 to Q9: `scaled_y2[i] = shr(y2[i], 3)` for overflow avoidance. (2) `<y2,y2>`: `L_acc = 1; for i in 0..L_SUBFR: L_acc = L_mac(L_acc, scaled_y2[i], scaled_y2[i])` (Q19); `exp = norm_l(L_acc); y2y2 = round(L_shl(L_acc, exp)); exp_y2y2 = exp + 19 - 16`; store `g_coeff[2] = y2y2`, `exp_g_coeff[2] = exp_y2y2`. (3) `-2<xn,y2>`: `L_acc = 1; for i in 0..L_SUBFR: L_acc = L_mac(L_acc, xn[i], scaled_y2[i])` (Q10); `exp = norm_l(L_acc); xny2 = round(L_shl(L_acc, exp)); exp_xny2 = exp + 10 - 16`; store `g_coeff[3] = negate(xny2)`, `exp_g_coeff[3] = exp_xny2 - 1` (the -1 accounts for the factor of 2). (4) `2<y1,y2>`: `L_acc = 1; for i in 0..L_SUBFR: L_acc = L_mac(L_acc, y1[i], scaled_y2[i])` (Q10); `exp = norm_l(L_acc); y1y2 = round(L_shl(L_acc, exp)); exp_y1y2 = exp + 10 - 16`; store `g_coeff[4] = y1y2`, `exp_g_coeff[4] = exp_y1y2 - 1`. Key: each `L_acc` initializes to 1 (not 0) to avoid `norm_l(0)` undefined behavior; the `round()` + `norm_l()` pattern normalizes each correlation to ~Q15 mantissa with a separate exponent. Indices [0] and [1] of `g_coeff`/`exp_g_coeff` are set elsewhere (by `Corr_xy2`'s caller for `<xn,xn>` and `<xn,y1>`). Reference: `COR_FUNC.C:26-83`
- State management accessors: `Lsp_encw_reset()`, `Get_freq_prev()`, `Update_freq_prev()` in `QUA_LSP.C` become methods on `EncoderState` in Rust (no standalone functions)
- **`Init_Pre_Process()` elimination** (`pre_proc.c`): Called from `coder.c:main()` (not from `Init_Coder_ld8a()`). Zeros the pre-processing HP filter state (y1_hi, y1_lo, y2_hi, y2_lo, x0, x1). In Rust, eliminated — initial values in `EncoderState::new()` (see encoder init table row 29)

**PRD errata to document (see Section 11):**

- PRD §3.7: "decimate by factor 3" refers to the algorithm name, not the literal loop step -- actual inner correlation step is `j+=2` (decimation by 2 on samples)
- PRD §3.10.1 (E13): Fractional pitch delay range for SF1 starts at 19+1/3, not 20. The encoding formula uses `T` in [19,85] with fractional offsets: `index = (T-19)*3 + frac - 1`. `PIT_MIN=20` constrains the open-loop search minimum; closed-loop refinement extends below it. Reference: `pitch_a.c:476`, `dec_lag3.c:35-44`

**Module file mapping:** `codec/state/encoder_state.rs`, `preproc.rs`, `lsp_quant/encode.rs` (`Qua_lsp`, `Lsp_qua_cs` — `Lsp_qua_cs` is the main quantization routine called once by `Qua_lsp`; it internally evaluates both MA modes via `Relspwed`'s mode loop; `QUA_LSP.C:55`), `lsp_quant/helpers.rs` (`Lsp_pre_select`, `Lsp_select_1/2`, `Lsp_get_quant` [9-parameter signature: `lspcb1[][M]` Q13, `lspcb2[][M]` Q13, `code0`, `code1`, `code2`, `fg[][M]` Q15, `freq_prev[][M]` Q13, `lspq[]` Q13 (out), `fg_sum[]` Q15 — full C signature in `lspgetq.c:16-26`; SPEC_PHASE_06 must document complete C-to-Rust type mapping for all 9 parameters], `Lsp_get_tdist`, `Lsp_last_select`, `Get_wegt`, `Relspwed` -- `Relspwed` is the core two-mode VQ codebook search loop within `Lsp_qua_cs`: for each MA mode (0 and 1), it extracts the prediction residual (`Lsp_prev_extract`), pre-selects the L1 candidate (`Lsp_pre_select`), selects L2/L3 candidates (`Lsp_select_1`, `Lsp_select_2`), enforces stability (`Lsp_expand_*`), and computes weighted distortion (`Lsp_get_tdist`); after both modes, `Lsp_last_select` picks the best mode), `pitch/open_loop.rs`, `pitch/closed_loop.rs`, `pitch/lag_encode.rs`, `pitch/parity.rs` (`Parity_Pitch`), `fixed_cb/search.rs` (`ACELP_Code_A`, `D4i40_17_fast`, `Cor_h` [private helper — `Cor_h` is `static` in `ACELP_CA.C`, not externally visible]), `fixed_cb/correlation.rs` (`Cor_h_X` — **cross-module dependency**: `Cor_h_X` is called from both `ACELP_CA.C:76` (fixed codebook search) and `PITCH_A.C:308` (closed-loop pitch search). `pitch/closed_loop.rs` depends on `fixed_cb/correlation.rs` for `Cor_h_X`. This placement matches the IMPLEMENTATION_PLAN module layout), `gain/quantize.rs`, `gain/taming.rs`, `codec/encode.rs`, `codec/encode_sub.rs`

**LSP quantization call chain:** The LSP quantization pipeline has a nested call structure that must be documented in SPEC_PHASE_06. The top-level call flow:

1. `Qua_lsp(lsp, lsp_q, ana)` — single entry point called from encoder main loop (`QUA_LSP.C:32`)
2. `Lsp_qua_cs(flsp_in, lspq_out, code)` — called once by `Qua_lsp`; manages quantization state

Inside `Lsp_qua_cs`, the core search is delegated to `Relspwed`:

3. `Relspwed(lsp, wegt, lspq, lspcb1, lspcb2, fg, freq_prev, fg_sum, fg_sum_inv, code_ana)` (`QUA_LSP.C:90-117`):
   - `for(mode=0; mode<MODE; mode++)` — iterates over both MA prediction modes
   - For each mode:
     - `Lsp_prev_extract(lsp, lsp_residual, fg[mode], freq_prev, fg_sum_inv[mode])` — extract prediction residual
     - `Lsp_pre_select(lsp_residual, lspcb1, &cand_cur)` — pre-select L1 candidate from 128-entry codebook
     - `Lsp_select_1(lspcb1, lsp_residual, wegt, lspcb2, &index)` — select best L2 candidate from 32-entry codebook
     - `Lsp_expand_1(lspcb1, GAP1)` — enforce minimum spacing after L2 selection
     - `Lsp_select_2(lspcb1, lsp_residual, wegt, lspcb2, &index)` — select best L3 candidate from 32-entry codebook
     - `Lsp_expand_2(lspcb1, GAP2)` — enforce minimum spacing after L3 selection
     - `Lsp_expand_1_2(lspcb1, GAP1)` — re-enforce L2 spacing after L3 adjustment
     - `Lsp_get_tdist(wegt, lsp_residual, &L_tdist, lspcb1, lspcb2, fg_sum)` — compute weighted total distortion for this mode
   - After both modes: `Lsp_last_select(L_tdist, &mode_index)` — pick the mode with lowest distortion

4. Back in `Lsp_qua_cs`:
   - `Lsp_get_quant(lspcb1, lspcb2, code0, code1, code2, fg, freq_prev, lspq, fg_sum)` — reconstruct final quantized LSP
   - `Lsp_prev_update(lspq, freq_prev)` — update prediction memory for next frame

**Annex B integration point:** `codec/encode.rs` must include a `#[cfg(feature = "annex_b")]` call site for `Update_cng(rh_nbe, exp_R0, Vad)` which runs **unconditionally** on every frame **after the VAD call but before the VAD conditional** (`cod_ld8a.c:254` — between the `vad()` call and the `if (Vad == 0)` check), regardless of VAD decision. Placement here is critical: if placed after the subframe loop, `Update_cng` would never execute on DTX frames (because `Cod_cng` returns early before the subframe loop), breaking autocorrelation accumulation for SID filter averaging. In Phase 6, this is a compile-gated stub; Phase 8 provides the real implementation. This prevents refactoring the encoder main loop when Annex B support is added. See Phase 8 key decisions for the complete DTX filter memory update loop (`wsp`, `mem_w`, `mem_w0` update via residual+filtering on inactive frames when `Vad == 0`)

**`vad_enable` flag propagation:** The C `Coder_ld8a(ana[], frame, vad_enable)` takes a third parameter `vad_enable` that gates all VAD/DTX processing. In the Rust implementation, this maps to `EncoderConfig::annex_b: bool` from the API layer. The encoder main loop (`codec/encode.rs`) must propagate this flag: when `vad_enable == false` (or `annex_b` feature disabled), the VAD call at `COD_LD8A.C:252-258`, the `Update_cng()` call, and the DTX conditional processing path (`COD_LD8A.C:260-302`) are all skipped. SPEC_PHASE_06 must document this gate and the compile-time (`#[cfg(feature = "annex_b")]`) vs runtime (`config.annex_b`) distinction

**TDD requirements:**

- Integration tests in `g729/tests/integration/encoder_conformance.rs`: 7 test functions plus round-trip test
- Bitstream byte-by-byte comparison against reference `.BIT` files
- Tier mapping: Tier 0 (unit tests), Tier 1 (ITU vectors), Tier 2 (C cross-validation via `tests/scripts/cross_validate.py`), Tier 3 (spectral analysis via `tests/scripts/spectral_analysis.py`)

**Mandatory spec deliverables checklist (Batch 4):**

- [ ] `Lsp_get_quant` complete C-to-Rust type mapping for all 9 parameters: `lspcb1[][M]` (Q13), `lspcb2[][M]` (Q13), `code0` (Word16), `code1` (Word16), `code2` (Word16), `fg[][M]` (Q15), `freq_prev[][M]` (Q13), `lspq[]` (Q13, output), `fg_sum[]` (Q15). Must document: C type, Rust type, Q-format, mutability, ownership/borrow semantics. Full C signature at `lspgetq.c:16-26`
