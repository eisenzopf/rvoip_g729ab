> Part of [G.729AB Implementation Plan](README.md)

### Phase 6: Encoder (Core G.729A)

**Goal**: Complete encoder. Most complex phase — the ACELP search alone is ~300 lines.

**Files to create**:
- `src/codec/state/encoder_state.rs` — EncoderState struct
- `src/preproc.rs` — Pre_Process: 2nd-order HP filter at 140 Hz with /2
- `src/lsp_quant/encode.rs` — `Qua_lsp()`: two-stage VQ with MA prediction
- `src/lsp_quant/helpers.rs` — `Lsp_pre_select`, `Lsp_select_1/2`, `Lsp_get_quant`, `Lsp_get_tdist`, `Lsp_last_select`, `Get_wegt`, `Relspwed`
- `src/pitch/open_loop.rs` — `Pitch_ol_fast()`: decimated multi-range search (even samples, j+=2)
- `src/pitch/closed_loop.rs` — `Pitch_fr3_fast()`: fractional 1/3-sample search, `G_pitch()`: optimal pitch gain computation
- `src/pitch/lag_encode.rs` — `Enc_lag3()`: encode pitch lag to P1/P2
- `src/fixed_cb/search.rs` — `ACELP_Code_A()` + `D4i40_17_fast()`: depth-first 4-pulse search; `Cor_h()` [private helper — `static` in `ACELP_CA.C`, computes impulse response autocorrelation matrix]
- `src/fixed_cb/correlation.rs` — `Cor_h_X()`: backward-filtered target correlation (externally visible in `COR_FUNC.C`; called from both `ACELP_CA.C:76` and `PITCH_A.C:308`)
- `src/gain/quantize.rs` — `Qua_gain()`: joint pitch+code gain VQ
- `src/gain/taming.rs` — `Init_exc_err()`, `update_exc_err()`, `test_err()`
- `src/codec/encode.rs` — `Coder_ld8a()`: main per-frame encoder
- `src/codec/encode_sub.rs` — Per-subframe encoder logic

**Reference**: `cod_ld8a.c`, `qua_lsp.c`, `lspgetq.c`, `pitch_a.c`, `acelp_ca.c`, `cor_func.c`, `qua_gain.c`, `gainpred.c`, `taming.c`, `pre_proc.c`, `p_parity.c`

**Total**: ~40 functions, ~3,200 lines

**Tasks**:
1. Define `EncoderState` with all buffers and ITU initial values (PRD §2.2.1):
   - `lsp_old` = {30000, 26000, 21000, 15000, 8000, 0, -8000, -15000, -21000, -26000}
   - `lsp_old_q` = {30000, 26000, 21000, 15000, 8000, 0, -8000, -15000, -21000, -26000}
   - `sharp` = 3277 (SHARPMIN)
   - `past_qua_en` = {-14336, -14336, -14336, -14336}
   - `freq_prev[0..3]` = each row = {2339, 4679, 7018, 9358, 11698, 14037, 16377, 18717, 21056, 23396}
   - `L_exc_err` = {0x00004000, 0x00004000, 0x00004000, 0x00004000} (Q14 — initialized to 1.0 in Q14, NOT zero; see `TAMING.C:25-26`)
   - `old_A[M+1]` = {4096, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0} (Q12 — Levinson-Durbin fallback LP coefficients, used when |k_i| >= 1.0; see `LPC.C`)
   - `old_rc[2]` = {0, 0} (Q15 — Levinson-Durbin fallback reflection coefficients; see `LPC.C`)
   - `mem_w[M]` = {0, ..., 0} (weighting filter `W(z)` memory; see `COD_LD8A.C`)
   - `mem_zero[M]` = {0, ..., 0} (zero-input response computation memory; see `COD_LD8A.C`)
   - `pastVad` = 1, `ppastVad` = 1 (previous two VAD decisions for DTX state machine; see `COD_LD8A.C`)
   - `frame` = 0 (frame counter, incremented per frame via `frame = add(frame, 1)` at `coder.c` AFTER the call to `Coder_ld8a()`; passed to `vad()` as `frm_count` for VAD initialization period — first 33 frames (frm_count 0-32) use hard energy threshold instead of MakeDec discriminants. In the C reference, `frame` is a parameter of `Coder_ld8a()` maintained by the calling program which increments it AFTER the call returns. In Rust, this should be a field on `EncoderState` (`frame: Word16 = 0`) — the current value is passed to the VAD first, then incremented at the **end** of `encode()`, so the VAD sees frm_count values 0, 1, 2, ..., matching the C reference. **Critical:** incrementing BEFORE the VAD call would shift the initialization period by one frame and fail Annex B conformance)
   - `seed` = INIT_SEED (= 11111) (encoder-side CNG random seed; `static Word16 seed` in `COD_LD8A.C`, initialized to INIT_SEED in `Init_Coder_ld8a()` at `COD_LD8A.C:158`, passed to `Cod_cng()` and `Calc_exc_rand()` for encoder CNG excitation — same INIT_SEED constant as decoder's CNG `seed`; distinct from decoder's `seed_fer` (21845); gated behind `#[cfg(feature = "annex_b")]`)
   - All other filter memories = 0, all excitation buffers = 0
   - **Scratch buffer note:** The C encoder's `Coder_ld8a()` uses a stack-local `synth_buf[L_FRAME+M]` (=90 samples) for local synthesis (to update filter memories). This is NOT persisted across frames. In Rust, this can be either a local array in the encode function or a scratch field on `EncoderState` for `no_std` contexts where stack space is constrained.
   - **Encoder buffer pointer-to-index mapping** (C pointer aliases from `cod_ld8a.c:129-136` mapped to Rust index offsets, analogous to the decoder's `res2_buf` mapping in Phase 5):

     | C pointer | C definition | Rust offset constant | Value |
     |-----------|-------------|---------------------|-------|
     | `new_speech` | `old_speech + L_TOTAL - L_FRAME` | `NEW_SPEECH_OFFSET` | `L_TOTAL - L_FRAME` |
     | `speech` | `new_speech - L_NEXT` | `SPEECH_OFFSET` | `L_TOTAL - L_FRAME - L_NEXT` |
     | `p_window` | `old_speech + L_TOTAL - L_WINDOW` | `P_WINDOW_OFFSET` | `L_TOTAL - L_WINDOW` |
     | `wsp` | `old_wsp + PIT_MAX` | `WSP_OFFSET` | `PIT_MAX` |
     | `exc` | `old_exc + PIT_MAX + L_INTERPOL` | `EXC_OFFSET` | `PIT_MAX + L_INTERPOL` |

     In Rust, `old_speech`, `old_wsp`, `old_exc` are flat arrays on `EncoderState`. Index-based access uses the offset constants (e.g., `&old_speech[NEW_SPEECH_OFFSET..NEW_SPEECH_OFFSET+L_FRAME]` for the new speech window). Buffer shifts at frame end: `old_speech.copy_within(L_FRAME.., 0)` etc.
2. Implement pre-processing HP filter (coefficients from PRD §3.1): b140=[1899,-3798,1899] Q12, a140=[4096,7807,-3733] Q12. Filter state MUST use DPF split high/low precision for output history (y1_hi, y1_lo, y2_hi, y2_lo) per PRD §3.1 — plain 16-bit state variables will cause bit-exact failures from frame 1
3. Implement LP analysis: window -> autocorrelation (with overflow retry) -> lag window -> Levinson
4. Implement LSP quantization: `Qua_lsp` calls `Lsp_qua_cs` once (`QUA_LSP.C:32`); `Lsp_qua_cs` internally evaluates both MA modes via its `Relspwed()` call, which loops `for(mode=0; mode<MODE; mode++)` (`QUA_LSP.C:90`): for each mode, it computes residual via `Lsp_prev_extract()`, applies frequency-dependent weighting via `Get_wegt()`, searches 128-entry L1 codebook using `Lsp_pre_select()` then `Lsp_select_1()`/`Lsp_select_2()`, searches two 32-entry L2/L3 codebooks, enforces stability via `Lsp_expand_*()`, and computes total weighted distortion via `Lsp_get_tdist()`; after both modes, `Lsp_last_select()` picks the best mode, `Lsp_get_quant()` retrieves the final quantized LSP, and `Lsp_prev_update()` updates prediction memory
5. Compute bandwidth-expanded LP coefficients and weighted speech: call `Int_qlpc()` for per-subframe quantized LSP interpolation (SF1: 0.5×prev + 0.5×curr, SF2: curr) — Annex A only interpolates quantized LSPs (no `Int_lpc`), convert interpolated LSPs to LP via `Lsp_Az()`, call `Weight_Az(A, 0.75, Ap)` to compute bandwidth-expanded coefficients `Ap[i] = a[i] × 0.75^i`, compute weighted speech `wsp[]` for open-loop pitch search via a **3-step pipeline** (`COD_LD8A.C:343-359`): (a) compute LP residual `Residu(Aq, speech, exc)` using plain A(z), (b) build tilt-compensated filter `Ap1[i] = Ap[i] - 0.7*Ap[i-1]` where Ap = A(z/0.75) and 0.7 is a hardcoded constant 22938 in Q15 (`COD_LD8A.C:345-347`). **Important:** this 22938 is numerically equal to `GAMMA1_PST` (post-filter denominator gamma=0.70) but serves a completely different purpose (encoder-side weighted speech tilt compensation vs. decoder-side formant post-filter). Do NOT alias these — define a separate named constant (e.g., `TILT_WSP = 22938`) to avoid coupling, (c) synthesize `Syn_filt(Ap1, exc, wsp)` producing `wsp = A(z)/[A(z/0.75)*(1-0.7z^{-1})] * speech`. The LP residual is also stored in `exc[]` for each subframe (`COD_LD8A.C:350-395`)
6. Implement open-loop pitch: the Annex A `Pitch_ol_fast()` uses a two-level decimation scheme (verified against `PITCH_A.C`):
   - **Inner correlation loop**: ALL three sections use `j+=2` (decimation by 2 on samples) for the dot-product accumulation
   - **Outer lag loop**: Sections 1 (20-39) and 2 (40-79) test every lag (`i++`); Section 3 (80-143) tests every other lag (`i+=2`), then refines ±1 around the best T3
   - The ±1 refinement applies **only to Section 3** (Range 80-143), not to Sections 1-2
   - Energy normalization for each section's best candidate uses `i+=2` (decimation by 2) as well
   - Prefer submultiples via **correlation boosting**: the code adjusts (boosts) normalized correlation values of lower-delay candidates when they are near sub-harmonics of higher-delay candidates (`PITCH_A.C:216-247`), then selects the candidate with the highest (possibly boosted) correlation via **direct comparison** without any threshold (`PITCH_A.C:249-256`). Note: base G.729's `Pitch_ol` uses an explicit `THRESHP=0.85` threshold for submultiple preference — Annex A's `Pitch_ol_fast` does NOT use this mechanism
   - **PRD §3.7 erratum:** PRD §3.7's "decimate by factor 3" refers to the Annex A algorithm name (vs base G.729's full-rate search), not the literal loop step — the actual inner correlation loop step is `j+=2` (decimation by 2 on samples). The reference code (`PITCH_A.C`) is authoritative
   - **Pitch energy overflow handling** (`PITCH_A.C:55-69`): The energy accumulation loop uses `L_mac` which can overflow for high-energy signals. If `DspContext.overflow` is set after accumulation, the preprocessed speech signal `scaled_signal[]` is right-shifted by 3 and the correlation search restarts with reduced dynamic range. This is the second of three overflow check sites (autocorrelation retry in Phase 3, pitch energy here, decoder synthesis in Phase 5). Clear the overflow flag before each energy loop, check after completion
7. Implement impulse response computation (PRD §3.9): compute h(n) of weighted synthesis filter `H(z) = 1/Â(z/0.75)` (Annex A spec A.3.3: `W(z)/Â(z) = 1/Â(z/γ)` with γ=0.75), truncated to L_SUBFR=40 samples. Set `h[0]=4096` (Q12), rest to zero, filter through `Syn_filt(Ap, h, h, L_SUBFR, &h[1], 0)` where `Ap` = `Weight_Az(Aq, 0.75)` = Â(z/0.75) coefficients (`COD_LD8A.C:407-409`). Used by both adaptive and fixed codebook searches.
8. Implement target signal: weighted speech minus zero-input response. Compute via `Syn_filt(Ap, &exc[i_subfr], xn, L_SUBFR, mem_w0, 0)` (`COD_LD8A.C:415`)
9. Implement adaptive codebook search: SF1 (8-bit, fractional 1/3-sample for delays 20-84; integer-only for 85-143 — closed-loop fractional search is skipped entirely when open-loop estimate falls in range 85-143), SF2 (5-bit differential, **always** uses fractional 1/3-sample search regardless of open-loop estimate range — PRD §3.10.1). For pitch delays T0 < L_SUBFR, the LP residual (computed via `Residu()` and stored in `exc[]`) already occupies the needed buffer positions, providing the "future" excitation samples for `Pred_lt_3()` interpolation (Annex A spec A.3.7). Compute optimal pitch gain via `G_pitch()`: normalize `<xn,y1>` and `<y1,y1>` via `norm_l` to prevent overflow, extract high 16-bit parts, call `div_s(xy_norm, yy_norm)`, apply exponent correction to get `g_p` in Q14. Clamp result to `[0, CONST12=19661]` (Q14, 1.2). If denominator is zero, return 0. Reference: `PITCH_A.C:413-452`. Then update target: `xn2[i] = xn[i] - g_p * y1[i]` (PRD §3.10.4-5)
10. Implement ACELP fixed codebook search: first apply **pitch sharpening to the impulse response h(n)** — `h[i] += sharp*h[i-T0]` for `i = T0..L_SUBFR-1` when `T0 < L_SUBFR` (`ACELP_CA.C:65-68`), converting `sharp` from Q14 to Q15 — then compute correlation matrix Cor_h, backward filter target Cor_h_X, depth-first 4-pulse search (320 total candidate evaluations: 2 track assignments × 2 searches × (16 pre-selection + 64 full evaluation)). After codebook search, apply the same pitch sharpening to the output code vector (`ACELP_CA.C:89-91`)
11. Implement gain quantization: MA prediction of fixed gain, joint VQ search (4 best GA * 8 best GB). The codebook search has **two separate paths** controlled by `tameflag` (`QUA_GAIN.C:249-300`): when `tameflag == 1` (taming active): `best_gain[0]` is clipped to `GPCLIP2` during pre-selection (`QUA_GAIN.C:138-140`), candidates with `g_pitch >= GP0999` (16383 Q14, ~0.999) are skipped (`QUA_GAIN.C:253`), and `g_p <= GPCLIP` (15564 Q14) is enforced; when `tameflag == 0` (no taming): no pitch gain ceiling is applied — all codebook candidates are evaluated regardless of pitch gain value (`QUA_GAIN.C:282-300`)
12. Implement memory update: construct excitation `u(n) = g_p * v(n) + g_c * c(n)` (PRD §5.7, `COD_LD8A.C:504-514`), then run `Syn_filt(Aq, &exc[i_subfr], &synth[i_subfr], L_SUBFR, mem_syn, 1)` to produce local synthesis output and update the encoder's synthesis filter memory `mem_syn[M]` (`cod_ld8a.c:525`; `synth_buf` is function-local scratch, `mem_syn` is persistent state on `EncoderState`), update `exc_err` via `update_exc_err(gain_pit, T0)`, update `mem_w0[j] = xn[j] - g_p*y1[j] - g_c*y2[j]` for filter memory (`COD_LD8A.C:518-525`), shift speech/wsp/exc buffers left by L_FRAME. **Annex B integration point:** Include a `#[cfg(feature = "annex_b")]` call site for `Update_cng(rh_nbe, exp_R0, Vad)` **after the VAD call but before the VAD conditional** (`cod_ld8a.c:254` — between the `vad()` call and the `if (Vad == 0)` check). This runs **unconditionally** on every frame, regardless of VAD decision. Placement here is critical: if placed after the subframe loop, `Update_cng` would never execute on DTX frames (because `Cod_cng` returns early before the subframe loop), breaking autocorrelation accumulation for SID filter averaging. In Phase 6, this can be a `todo!()` stub or compile-gated no-op; Phase 8 will provide the real implementation. **Encoder CNG seed reset:** On every active speech frame (VAD == 1), the encoder resets `seed = INIT_SEED` (`cod_ld8a.c:312`). This must be in the active-frame branch after `*ana++ = 1`, mirroring the decoder's seed reset on ftyp==1. Without this reset, encoder CNG excitation after speech starts from a stale seed value, failing Annex B conformance tests (tstseq1-4)

**Static/helper functions included in tasks above** (for C-to-Rust mapping completeness):
- `Dot_Product(x, y, L)` in `PITCH_A.C`: energy normalization for open-loop pitch search (used in task 6). Map to inline helper in `pitch/open_loop.rs`
- `Corr_xy2(xn, y1, y2, g_coeff, exp_g_coeff)` in `COR_FUNC.C`: computes cross-correlation coefficients between target signal, filtered adaptive codebook vector, and filtered fixed codebook vector for gain quantization (called at `COD_LD8A.C:481`, used by task 11). Map to helper in `gain/quantize.rs`
- `Gbk_presel(best_gain, cand1, cand2, gcode0)` in `QUA_GAIN.C`: gain codebook pre-selection narrowing 128-entry joint search to 4×8 candidates (used in task 11). Map to helper in `gain/quantize.rs`
- State management accessors `Lsp_encw_reset()`, `Get_freq_prev()`, `Update_freq_prev()` in `QUA_LSP.C`: in Rust, these become methods on `EncoderState` (e.g., `encoder_state.freq_prev()`, `encoder_state.update_freq_prev()`). No standalone functions needed
- `perc_var()` (from base G.729's `PWF.C`, absent in g729ab_v14): **eliminated** — computes adaptive perceptual weighting gammas in base G.729; Annex A uses fixed gamma=0.75 (`GAMMA1=24576` Q15) instead, so `perc_var()` is never called and `PWF.C` does not exist in the combined AB codebase. No Rust mapping needed

**Test Plan**:

| Test | Input File | Expected Output | Validates |
|------|-----------|-----------------|-----------|
| SPEECH.IN -> SPEECH.BIT | ITU test vector | Bit-exact bitstream | Generic speech encoding |
| ALGTHM.IN -> ALGTHM.BIT | ITU test vector | Bit-exact bitstream | Algorithm path coverage |
| PITCH.IN -> PITCH.BIT | ITU test vector | Bit-exact bitstream | Pitch search |
| LSP.IN -> LSP.BIT | ITU test vector | Bit-exact bitstream | LSP quantization |
| FIXED.IN -> FIXED.BIT | ITU test vector | Bit-exact bitstream | Fixed codebook search |
| TAME.IN -> TAME.BIT | ITU test vector | Bit-exact bitstream | Taming procedure |
| TEST.IN -> TEST.BIT | ITU test vector | Bit-exact bitstream | Additional general coverage |
| Round-trip | Encode SPEECH.IN, decode result | Output matches SPEECH.PST | End-to-end |

**CONFORMANCE CHECKPOINT 2**: All 7 encoder test vectors pass bit-exactly. Encoder->Decoder round-trip matches reference.

**Exit Criteria**:
1. All 7 Annex A encoder test vectors pass bit-exactly
2. Encoder→Decoder round-trip produces output matching SPEECH.PST
3. CPU profile confirms no unexpected allocations or hot-path regressions
4. Taming procedure exercised and verified (TAME vector)

**TDD Workflow**:
1. **Write integration tests first**: Create `tests/integration/encoder_conformance.rs` with 7 test functions (one per encoder vector) plus a round-trip test. Each loads a `.IN` PCM file, encodes it, and compares the bitstream byte-by-byte against the reference `.BIT` file. All fail initially.
   ```
   cargo test --test encoder_conformance --features itu_serial  # expect: 0 passed
   ```
2. **Implement incrementally**: Pre-processing -> LP analysis pipeline -> LSP quantization -> open-loop pitch -> impulse response -> adaptive CB -> ACELP search -> gain quantization -> memory update -> main encoder loop. Start with SPEECH vector; once it passes, the others typically follow (TAME is usually last).
3. **Verify**:
   ```
   cargo test --test encoder_conformance --features itu_serial  # all 7 pass
   cargo test --test round_trip --features itu_serial           # encode->decode matches
   python tests/scripts/run_all_tiers.py --phase 6              # exits 0 (Gate 4)
   ```
4. **Gate 4 (Conformance Checkpoint 2)**: `run_all_tiers.py --phase 6` reports all 7 encoder vectors bit-exact plus all 10 decoder vectors still passing (no regressions).
