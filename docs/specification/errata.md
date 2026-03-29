> Part of [Specification Plan](README.md)

# Errata Registries

## 12. PRD Errata Registry

The following corrections to `PRD.md` were discovered during implementation plan analysis. The C reference code is authoritative in all cases. Each spec document should reference the relevant errata in its Section 9 (PRD Errata).

**Application status:** These errata correct the PRD's algorithmic descriptions. Some corrections have been applied inline to `PRD.md` (e.g., §3.7 decimation, §3.10.1 fractional pitch, §6.2-6.4 erasure handling); others remain as-documented here with the original PRD text preserved for audit trail. The PRD is treated as a mostly-frozen requirements document — this registry is the authoritative source for corrections.

### E1: PRD §3.7 -- Open-Loop Pitch Decimation Factor

**PRD claim:** "Decimation by factor 3" for the Annex A open-loop pitch search.

**Correction:** "Decimate by factor 3" refers to the Annex A algorithm name (`Pitch_ol_fast`) vs base G.729's full-rate `Pitch_ol` search. The actual inner correlation loop step is `j+=2` (decimation by 2 on samples), not `j+=3`. The outer lag loop in Section 3 (delays 80-143) also uses `i+=2` (every other lag), then refines +/-1 around the best.

**Reference:** `PITCH_A.C` lines 112, 142, 172 (all use `j+=2`).

**Affects:** SPEC_PHASE_06_encoder.md

**Note:** The same "decimation by 3" wording also appeared in PRD §15 Phase 4. Both occurrences were incorrect — the literal sample step is 2, not 3.

**Status:** APPLIED to PRD §3.7 (corrected in Section 3 text) and PRD §15 Phase 4 (now reads "decimated even-sample search per Annex A `Pitch_ol_fast`").

### E2: PRD §6.2 -- Gain Prediction Memory During Frame Erasure

**PRD claim:** `past_qua_en` (gain prediction memory) is "not updated" during frame erasure.

**Correction:** `past_qua_en` IS updated during erasure via `Gain_update_erasure()` (`gainpred.c:137-158`, called from `dec_gain.c:67`). This function computes `avg = mean(past_qua_en) - 4.0 dB` (Q10: subtract 4096), clamps to -14.0 dB (-14336 Q10), shifts the history array, and stores the result in `past_qua_en[0]`. The attenuated concealment gains (0.9*g_p, 0.98*g_c) themselves are NOT fed back into the MA predictor, but `past_qua_en` IS modified with a decaying average.

**Reference:** `gainpred.c:137-158`, `dec_gain.c:67`.

**Affects:** SPEC_PHASE_05_decoder.md

**Status:** APPLIED — PRD §6.2 now correctly states `past_qua_en` IS updated during erasure via `Gain_update_erasure()`.

### E3: PRD §6.3 -- Gain Limiting on Recovery After Erasure

**PRD claim:** "Gain limiting/smoothing during the first good frame" after erasure.

**Correction:** The reference decoder (`DEC_LD8A.C`) has no explicit gain limiting or smoothing on the first good frame after erasure. Normal decoding resumes immediately. The apparent stability is emergent from the `Gain_update_erasure()` behavior during erasure (E2): `past_qua_en` converges toward a floor of -14.0 dB during erasure, so when normal decoding resumes, the MA predictor operates from an energy level that has decayed toward the floor.

**Reference:** `dec_ld8a.c` recovery path -- no explicit gain limiting code.

**Affects:** SPEC_PHASE_05_decoder.md

**Status:** APPLIED — PRD §6.3 now correctly describes emergent gain stability from `Gain_update_erasure()` floor convergence, with no explicit gain limiting.

### E4: PRD §6.4 -- Voicing Classification in Annex A Decoder

**PRD claim:** Describes a voicing classification using ">3 dB prediction gain" for frame erasure concealment.

**Correction:** The Annex A decoder has no explicit voicing classification. The Annex A specification (A.4.4) states: "no voicing detection is used." The voiced/unvoiced distinction during frame erasure is emergent from the previous pitch gain magnitude -- high previous `gain_pitch` produces pitch-coherent concealment, low gain produces noise-like output. There is no explicit ">3 dB prediction gain" check in the reference code.

**Reference:** Annex A spec A.4.4; `dec_ld8a.c` -- no voicing classification code.

**Affects:** SPEC_PHASE_05_decoder.md

**Status:** APPLIED — PRD §6.4 now correctly states no explicit voicing classification in Annex A decoder.

### E5: PRD §6.2 -- Pitch Continuation During Erasure

**PRD claim:** Pitch delay is "unchanged for both subframes, increment by 1 after frame."

**Correction:** The `old_T0` increment happens per-subframe inside each subframe's bad-pitch branch, not after the frame. For a full frame erasure: SF1 uses `T0 = old_T0` (value X), then `old_T0 += 1` (now X+1); SF2 uses `T0 = old_T0` (value X+1), then `old_T0 += 1` (now X+2). Net effect: old_T0 increments by **2 per erased frame** (1 per subframe), capped at PIT_MAX=143.

**Reference:** `dec_ld8a.c` lines 243-249 (SF1) and 259-265 (SF2).

**Affects:** SPEC_PHASE_05_decoder.md

**Status:** APPLIED — PRD §6.2 now correctly describes per-subframe pitch increment during erasure (old_T0 increments by 1 per subframe, 2 per erased frame).

### E6: PRD §9.1 -- Lag Window Coefficient Count

**PRD claim:** "Lag window coefficients (11 values)" in §9.1.

**Correction:** The reference code declares M+2=12 entries for the lag window DPF high/low pairs (`lag_h[12]`, `lag_l[12]` in `tab_ld8a.c`). The `Lag_window(m, r_h, r_l)` function accesses `lag_h[i-1]` for `i=1..m` (`LPC.C:111-115`). When Annex B is active, `Lag_window` is called with `m=NP=12`, using all 12 entries (`lag_h[0]` through `lag_h[11]`) for lag coefficients 1-12. Entry 11 is **NOT** a guard element — it is the active lag window coefficient for lag 12, required by the VAD's NP=12 order analysis. The table size M+2=12 is designed to accommodate `max(M, NP) = 12` lags due to the `i-1` indexing. The Rust implementation must use 12-element arrays matching the reference code.

**Reference:** `tab_ld8a.c` lag window array declarations, `LPC.C:111-115` (`Lag_window` loop bounds), `COD_LD8A.C:244` (`Lag_window(NP, ...)`).

**Affects:** SPEC_PHASE_02_tables.md, SPEC_PHASE_03_common_dsp.md

**Status:** APPLIED — PRD §9.1 now correctly states "12 values, M+2 entries" for lag window coefficients.

### E7: PRD §14 -- Crate File Structure

**PRD claim:** §14 suggests a flat ~20-file layout (`basic_ops.rs`, `encoder.rs`, `decoder.rs`, `lpc.rs`, etc.).

**Deviation:** The implementation plan deliberately uses a hierarchical ~99-file structure (~84 algorithm/data files + ~15 `mod.rs` re-export and infrastructure files) with 6 logical layers (`dsp/`, `tables/`, kernel modules, `codec/`, `bitstream/`, `api/`) to enforce the 200 LOC file-length target and improve navigability. The single-crate constraint from the PRD is preserved. This is a considered design decision, not a PRD error.

**Reference:** [Implementation Plan](../implementation/README.md) Crate Organization section.

**Affects:** All spec documents (module file mappings use hierarchical paths)

### E8: PRD §11.4 -- CLI Binary Structure

**PRD claim:** §11.4 specifies separate `g729-enc` and `g729-dec` binaries.

**Deviation:** The implementation uses a unified `g729-cli` binary with subcommands (`encode`, `decode`, `test-vectors`) for single-binary distribution. This is a deliberate design choice preferred for deployment simplicity.

**Reference:** [Implementation Plan Phase 10](../implementation/phase_10_api_cli.md) Task 4.

**Affects:** SPEC_PHASE_10_api_cli.md

### E9: PRD §15 -- Implementation Phase Structure

**PRD claim:** §15 defines a 6-phase structure: Phase 1 (Foundation: basic ops + filters + tables), Phase 2 (Decoder), Phase 3 (Decoder Post-Processing), Phase 4 (Encoder), Phase 5 (Annex B VAD/DTX/CNG), Phase 6 (Conformance and Optimization).

**Deviation:** The implementation plan restructures this into 10 finer-grained phases: tables separated from DSP math (Phases 1-2), bitstream as its own phase (Phase 4), decoder post-processing folded into the main decoder phase (Phase 5), VAD/DTX/CNG split across three phases (Phases 7-9), and API/CLI as its own hardening phase (Phase 10). This enables more granular conformance gates, clearer dependency tracking, and parallelizable batch scheduling.

**Reference:** [Implementation Plan](../implementation/README.md) Implementation Phases section.

**Affects:** All spec documents (batch schedule and dependency graph use the 10-phase structure)

### E10: PRD §5.9.2 -- Formant Post-Filter Gain Normalization

**PRD claim:** "Normalize gain: `g_f = 1 / SUM(|h_f(n)|)` where h_f is the truncated impulse response."

**Correction:** The gain factor `gf` does NOT apply to Annex A. Annex A spec A.4.2 explicitly states that `gf` (and `gt`) are "eliminated" compared to base G.729. The ITU reference code (`postfilt.c`) has no `gf` variable, no gain normalization computation, and no application of any normalization factor. The bcg729 implementation (`postFilter.c`) also omits the `gf` normalization factor. (Note: bcg729 does implement AGC per A.4.2.4, which is a separate gain mechanism from `gf`.) The formant post-filter applies spectral weighting via `A(z/GAMMA2_PST)` / `A(z/GAMMA1_PST)` only.

**Reference:** `reference/.../g729ab_v14/postfilt.c` (lines 130-184, no gf present); Annex A spec A.4.2.

**Affects:** SPEC_PHASE_05_decoder.md (`postfilter/formant.rs`)

**Status:** APPLIED — PRD §5.9.2 now correctly states that `gf` normalization is eliminated in Annex A.

### E11: PRD §5.9.3 -- Tilt Compensation Formula and Conditionality

**PRD claim:** `H_t(z) = 1 - mu * z^-1` where `mu = gamma_t * k'_1`; `gamma_t = tilt factor (constant, ~0.8)`.

**Correction (sign):** The PRD defines `mu = gamma_t * k'_1`. Since `k'_1 = -r_h(1)/r_h(0)` (Annex A eq A.14), this gives the wrong filter sign (low-pass instead of high-pass). The reference code (`postfilt.c:162-180`) computes `g = MU * r_h(1)/r_h(0)` (using `r_h(1)` directly, not `k'_1`), then applies `signal[i] = signal[i] - g * signal[i-1]`, yielding `H_t(z) = 1 - g*z^-1 = 1 + gamma_t * k'_1 * z^-1` (Annex A eq A.15). The correct formula in the PRD's notation would be `mu = -gamma_t * k'_1`.

**Correction (conditionality):** The PRD says gamma_t is constant (~0.8). The Annex A spec (A.4.2.3) and reference code (`postfilt.c:172-174`) apply gamma_t conditionally: `gamma_t = 0.8` when `k'_1 < 0` (equivalently, `r_h(1) > 0`), `gamma_t = 0.0` when `k'_1 >= 0` (equivalently, `r_h(1) <= 0`). The bcg729 implementation (`postFilter.c:245-260`) confirms this conditional behavior.

**Reference:** `reference/.../g729ab_v14/postfilt.c` (lines 162-180); Annex A spec A.4.2.3.

**Affects:** SPEC_PHASE_05_decoder.md (`postfilter/pipeline.rs`, tilt compensation stage)

**Status:** APPLIED — PRD §5.9.3 now correctly describes conditional gamma_t (0.8 when k'_1 < 0, 0.0 otherwise) and correct sign convention.

### E12: PRD §8.1.1 -- VAD Autocorrelation Coefficient Count

**PRD claim:** "Uses 11 autocorrelation coefficients (lags 0-10 from speech LP analysis, extended to NP=12 for VAD)."

**Correction:** When Annex B is active, the encoder's main `Autocorr()` is called with order NP=12 (not M=10), producing 13 autocorrelation lags (0-12) from the start. The VAD does NOT "extend" the M=10 result — the autocorrelation is directly computed at order NP=12 and shared. Levinson-Durbin uses only the first M+1=11 values for LP coefficient computation, while the VAD's energy features use all 13 values. The PRD's "11 (lags 0-10)" confuses the LP analysis order (M=10) with the actual autocorrelation computation order (NP=12). An implementer using only 11 lags would produce incorrect VAD energy estimates.

**Reference:** `COD_LD8A.C:231` (`r_h[NP+1]`, `r_l[NP+1]`), `COD_LD8A.C:242` (`Autocorr(p_window, NP, r_h, r_l, ...)`), `vad.h` (NP=12).

**Affects:** SPEC_PHASE_07_vad.md (`annex_b/vad/features.rs`)

**Status:** PARTIALLY APPLIED. PRD §8.1.1 has been corrected to read "Uses 13 autocorrelation coefficients (lags 0-12, NP=12 for VAD; distinct from the speech LP analysis which uses M=10, lags 0-10)." PRD §3.2.2 retains "Compute 11 autocorrelation coefficients (lags 0 through 10)" which is correct for the speech LP analysis case (M=10), but now includes a cross-reference note about the variable-order NP=12 when Annex B is active.

### E13: PRD §3.10.1 -- Fractional Pitch Delay Lower Bound

**PRD claim:** "Fractional pitch delays 20 to 84+2/3 use 1/3-sample resolution."

**Correction:** The fractional pitch delay range for subframe 1 starts at 19+1/3, not 20. The encoding formula `index = (T-19)*3 + frac - 1` where `T` ranges from 19 to 85 with `frac` in {-1, 0, 1} produces a minimum delay of 19+1/3 (when `index=0`: `T0=19`, `T0_frac=1`). `PIT_MIN=20` (`ld8a.h:29`) constrains the open-loop search minimum, not the closed-loop fractional refinement. The reference code comment (`pitch_a.c:476`) explicitly states "19 1/3 to 84 2/3 resolution 1/3".

**Reference:** `reference/.../g729ab_v14/pitch_a.c` (lines 475-483), `reference/.../g729ab_v14/dec_lag3.c` (lines 35-44).

**Affects:** SPEC_PHASE_06_encoder.md (`pitch/lag_encode.rs`), SPEC_PHASE_05_decoder.md (`pitch/lag_decode.rs`)

**Status:** APPLIED — PRD §3.10.1 now correctly states fractional pitch delay starts at 19+1/3.

### E14: PRD §5.9.1-5.9.3 -- Post-Filter Operation Order and Gamma Subscript Swap

**PRD claim (§5.9.1):** "Compute residual through `A(z/gamma_d)` (the formant post-filter denominator): `Residu(A(z/0.70), synth, res2, L_SUBFR)`."

**PRD claim (§5.9.2):** Formula says `Syn_filt(A(z/gamma_n), res2_pst, output, L_SUBFR, mem_syn_pst)`.

**PRD claim (§5.9 ordering):** §5.9.2 (Formant/synthesis) is listed before §5.9.3 (Tilt compensation).

**Correction (gamma subscripts):** The gamma subscripts are swapped in the formula text. The residual computation uses `A(z/gamma_n)` = `A(z/0.55)` (GAMMA2_PST), NOT `A(z/gamma_d)` (0.70). The synthesis uses `1/A(z/gamma_d)` = `1/A(z/0.70)` (GAMMA1_PST), NOT `1/A(z/gamma_n)`. The table in PRD §5.9.2 correctly assigns `gamma_n=0.55` for residual and `gamma_d=0.70` for synthesis — only the formula text has the subscripts swapped. Reference: `postfilt.c:133` (`Weight_Az(Az, GAMMA2_PST, M, Ap3)` for residual), `postfilt.c:184` (`Syn_filt(Ap4, ...)` where Ap4 uses GAMMA1_PST for synthesis).

**Correction (operation order):** The Annex A reference code (`postfilt.c:154-184`) applies tilt compensation to the residual-domain signal `res2_pst` (line 180: `preemphasis(res2_pst, temp2, L_SUBFR)`) BEFORE synthesis (line 184: `Syn_filt(Ap4, res2_pst, &syn_pst[i_subfr], ...)`). Annex A spec A.4.2.3 confirms: "The compensation filtering H_t(z) is performed before synthesis through 1/Â(z/γ_d)." PRD §5.9 lists the operations as §5.9.2 (synthesis) then §5.9.3 (tilt), which is the wrong order. The correct per-subframe pipeline is: (a) residual via A(z/γ_n), (b) scale, (c) pitch PF, (d) tilt compensation, (e) synthesis via 1/A(z/γ_d), (f) AGC.

**Reference:** `reference/.../g729ab_v14/postfilt.c` (lines 133-184); Annex A spec A.4.2.3.

**Affects:** SPEC_PHASE_05_decoder.md (`postfilter/pipeline.rs`, operation ordering and gamma assignments)

**Status:** APPLIED — (a) PRD §5.9.1 inline text corrected to `A(z/gamma_n)` with GAMMA2_PST=0.55. (b) PRD §5.9.2 formula text corrected from `Syn_filt(A(z/gamma_n), ...)` to `Syn_filt(A(z/gamma_d), ...)`. (c) PRD §5.9.3 h_f description corrected: `Syn_filt(A(z/gamma_d))` applied to `Residu(A(z/gamma_n))` (subscripts were swapped)

### E15: PRD §3.7 -- Open-Loop Pitch Submultiple Preference Mechanism

**PRD claim:** "a lower-delay candidate replaces a higher-delay one if its normalized correlation exceeds 0.85× the higher-delay correlation."

**Correction:** The 0.85× threshold (`THRESHP`) is the base G.729 `Pitch_ol` mechanism. Annex A's `Pitch_ol_fast` uses a different approach: it **boosts** normalized correlation values of lower-delay candidates when they are near sub-harmonics of higher-delay candidates (`pitch_a.c:216-247`), then selects the candidate with the highest (possibly boosted) correlation via **direct comparison** without a threshold (`pitch_a.c:249-256`). The [Implementation Plan Phase 6](../implementation/phase_06_encoder.md) Task 6 also incorrectly described this as "0.85x threshold for T2 vs T3, 0.20x for T1 vs T2."

**Reference:** `PITCH_A.C` lines 216-256.

**Affects:** SPEC_PHASE_06_encoder.md

**Status:** APPLIED to PRD §3.7 and [Implementation Plan Phase 6](../implementation/phase_06_encoder.md) Task 6.

### E16: IMPLEMENTATION_PLAN Phase 6 / SPECIFICATION_PLAN Phase 8 -- Encoder CNG Seed Initialization

**PRD/IMPL claim:** `seed = 0` for encoder-side CNG random seed (`COD_LD8A.C`).

**Correction:** `Init_Coder_ld8a()` explicitly sets `seed = INIT_SEED` (= 11111, from `dtx.h:76`) at `cod_ld8a.c:158`. The encoder CNG seed uses the same `INIT_SEED` constant as the decoder CNG seed (11111), NOT zero. Using zero would produce incorrect CNG excitation sequences from the first noise frame.

**Reference:** `cod_ld8a.c` `Init_Coder_ld8a()`: `seed = INIT_SEED;` (line 161); `dtx.h:76`: `#define INIT_SEED 11111`.

**Affects:** SPEC_PHASE_06_encoder.md (EncoderState initialization), SPEC_PHASE_08_dtx.md (encoder-side CNG seed)

**Status:** APPLIED to [Implementation Plan Phase 6](../implementation/phase_06_encoder.md) Task 1 and [Specification Plan Batch 5](batch_05_vad_dtx_cng.md). PRD §2.2.1 updated with encoder CNG seed row.

### E17: PRD §8.1.3 -- VAD Initialization Period Frame Count

**PRD claim:** "Initialization (frames 0-31):" implying 32 frames of initialization.

**Correction:** The reference code uses `sub(frm_count, INIT_FRAME) <= 0` with `INIT_FRAME=32` (`vad.h:12`, `vad.c:183`). With `frm_count` starting at 0, this check is true for frames 0, 1, 2, ..., 32 — a total of **33 frames**, not 32. Frame 32 is the last initialization frame, where the mean adjustment using `factor_fx[less_count]` and `shift_fx[less_count]` tables occurs and the initialization flag is cleared.

**Reference:** `vad.h:12` (`#define INIT_FRAME 32`), `vad.c:183` (`if (sub(frm_count, INIT_FRAME) <= 0)`).

**Affects:** SPEC_PHASE_07_vad.md (initialization period description, unit test for boundary frame 32)

**Status:** APPLIED to PRD §8.1.3 (changed to "frames 0-32, i.e., 33 frames total") and [Specification Plan Batch 5](batch_05_vad_dtx_cng.md).

### E18: IMPLEMENTATION_PLAN Phase 6 / SPECIFICATION_PLAN Phase 7 -- VAD Frame Counter Increment Timing

**IMPL claim:** "frame ... incremented internally at the start of each encode() call, then passed to the VAD."

**Correction:** In the C reference, `coder.c` increments `frame` before the call to `Coder_ld8a()` with a first-frame guard that skips the increment on the initial invocation (`coder.c:117`: `else frame++;` then `coder.c:120`: `Coder_ld8a(prm, frame, vad_enable);`). This produces the sequence 0, 1, 2, ... as seen by the VAD, which is equivalent to the Rust pattern of passing frame to VAD first, then incrementing at the end of `encode()`. If implemented with pre-increment on every frame (including the first), the VAD would see `frm_count` values offset by +1, shifting the initialization period by one frame.

**Reference:** `coder.c` main loop: `if(frame == 0) { ... } else frame++; Coder_ld8a(prm, frame, vad_enable);` — first-frame guard ensures frame=0 on first call, then 1, 2, ... on subsequent calls.

**Affects:** SPEC_PHASE_06_encoder.md (EncoderState frame counter), SPEC_PHASE_07_vad.md (initialization period boundary)

**Status:** APPLIED to [Implementation Plan Phase 6](../implementation/phase_06_encoder.md) Task 1 and [Specification Plan Batch 5](batch_05_vad_dtx_cng.md) (changed to "passed to VAD first, then incremented at end of encode()").

### E19: PRD §3.12.3 -- Taming `tab_zone` Negative Index Access Undocumented

**Issue:** The PRD described taming zone-based error tracking but did not document the boundary handling for `tab_zone` indices when pitch delays are below `L_SUBFR=40`. The C reference code (`taming.c`) has explicit guards: `test_err()` clamps negative indices to 0 (`taming.c:49`); `update_exc_err()` branches to an alternate code path when `T0 < L_SUBFR` (`taming.c:92`). In Rust, these guards are critical to prevent out-of-bounds panics.

**Affects:** PRD §3.12.3, SPEC_PHASE_06_encoder.md (taming function specs)

**Status:** APPLIED to PRD §3.12.3 (added boundary handling documentation) and [Specification Plan Batch 4](batch_04_encoder.md) (added taming `tab_zone` index boundary handling note).

### E20: PRD §8.1.2 -- MakeDec Conditions 8-10 Wrong Variable Name (dSLE vs dSE)

**PRD claim:** MakeDec conditions 8-10 use variables `dSLE` (low-band energy difference).

**Correction:** The C code at `vad.c:419,425,428` uses `dSE` (full-band energy), not `dSLE` (low-band energy). The C code **comments** at `vad.c:415` misleadingly say "dSLE vs dSZC", but the actual code variables are `dSE`. An implementer following the PRD would compute comparisons against the wrong VAD feature, producing incorrect voice/noise decisions.

**Reference:** `vad.c` lines 416-430: conditions 8/9 use `L_add(acc0, L_deposit_h(dSE))` and condition 10 uses `L_mult(dSE, 32767)`.

**Affects:** SPEC_PHASE_07_vad.md (MakeDec cross-verification table)

**Status:** APPLIED to PRD §8.1.2 (conditions 8-10 changed from `dSLE` to `dSE`).

### E21: PRD §8.1.2 -- MakeDec Conditions 1-4, 8-9 Wrong Operation Representation

**PRD claim:** Conditions 1-4, 8-9 use notation `+ L_shl(XX,N)` → shr N, implying the third term is shifted left and the entire expression is shifted right.

**Correction:** The actual code accumulates the first two L_mult/L_mac terms, applies `L_shr(acc0, N)` to the accumulator, THEN adds the third term via `L_add(acc0, L_deposit_h(XX))`. These are fundamentally different operations: `L_shl(SD, 8)` = SD × 256 (a Q23 value), while `L_deposit_h(SD)` = SD × 65536 (a Q31 value). The PRD's notation would produce incorrect discriminant values for all 6 affected conditions, potentially flipping voice/noise decisions.

**Reference:** `vad.c` lines 376-419: e.g., condition 1 at lines 378-379: `acc0 = L_shr(acc0, 8); acc0 = L_add(acc0, L_deposit_h(SD));`.

**Affects:** SPEC_PHASE_07_vad.md (MakeDec cross-verification table)

**Status:** APPLIED to PRD §8.1.2 (conditions 1-4, 8-9 changed to correct `L_shr` → `L_deposit_h` pattern; conditions 12-13 also corrected from `L_shl(dSLE,0)` to `L_deposit_h(dSLE)`).

### E22: PRD §8.1.3 -- Smoothing Stage 4 "Forced Noise" Conditions Inaccurate

**PRD claim:** Stage 4 described as "If VOICE, count_sil > 10, `ΔEf ≤ 614` → force NOISE, reset counter."

**Correction:** The actual reference code (`vad.c:270-272`) implements substantially different conditions: `(Ef - 614 < MeanSE) AND (frm_count > 128) AND (!v_flag) AND (rc < 19661)`. The PRD's original description was a simplified paraphrase that omitted the `frm_count > 128` initialization guard, the `v_flag` voice classification state check, and the reflection coefficient threshold `rc < 19661`. It also incorrectly described the energy condition as `ΔEf ≤ 614` (a delta) rather than `Ef - 614 < MeanSE` (comparison against running mean). The forced-NOISE override uses a stricter `rc` threshold (19661 ≈ 0.6 Q15) than the background noise update condition (24576 ≈ 0.75 Q15).

**Reference:** `vad.c:270-272`

**Affects:** SPEC_PHASE_07_vad.md (smoothing stage documentation), PRD §8.1.3

**Status:** APPLIED to PRD §8.1.3.

### E23: PRD §5.8.1 / §10.5 -- Wrong Line Reference for Decoder Synthesis Overflow

**PRD claim:** `DEC_LD8A.C:251-253` cited as the location of synthesis overflow handling.

**Correction:** Lines 251-253 are inside the pitch continuation logic (second subframe bad-pitch branch), not overflow handling. The actual synthesis overflow retry-with-scaling code is at lines 169-181 (non-active/CNG frames) and lines 331-344 (active speech frames). Both implement the identical pattern: `Overflow = 0; Syn_filt(..., 0); if(Overflow) { scale_exc >>2; Syn_filt(..., 1); }`.

**Reference:** `dec_ld8a.c` lines 169-181, 331-344.

**Affects:** SPEC_PHASE_05_decoder.md (synthesis overflow handling)

**Status:** APPLIED to PRD §5.8.1 (overflow code example), §10.5 (overflow flag table), and [Implementation Plan Phase 1](../implementation/phase_01_dsp_math.md) (DspContext overflow flag check sites).

### E24: PRD §5.9.5 -- Misleading Description of Post-Filter Vad=0 Behavior

**PRD claim:** "During comfort noise generation (Vad = 0), the long-term post-filter pitch delay comes from old_T0, which retains its value from the last speech frame."

**Correction:** The pitch post-filter is **bypassed entirely** when `Vad == 0` — `res2` is copied directly to `res2_pst` without any correlation search or pitch filtering (`postfilt.c:148-151`). The formant post-filter, tilt compensation, and AGC stages still apply. `old_T0` is preserved so that when speech resumes, the pitch PF has a valid delay to search around.

**Reference:** `postfilt.c` lines 148-151.

**Affects:** SPEC_PHASE_05_decoder.md (post-filter section)

**Status:** APPLIED to PRD §5.9.5.

### E25: PRD §3.11.2 / §13.5 -- bcg729 ACELP Candidate Count Overestimate

**PRD claim:** bcg729 uses "~2048 candidate combinations" for its fixed codebook search.

**Correction:** bcg729's `fixedCodebookSearch.c` uses 2 `m3Base` x 2 `mIndex` = 4 passes, each evaluating ~144 energy/correlation candidates, for a total of ~576 full evaluations — not ~2048.

**Reference:** bcg729 `src/fixedCodebookSearch.c`.

**Affects:** PRD §3.11.2 (ACELP search note), PRD §13.5 (interop testing note), [Implementation Plan Phase 6](../implementation/phase_06_encoder.md) (conformance note)

**Status:** APPLIED to PRD §3.11.2, PRD §13.5, and [Implementation Plan Phase 6](../implementation/phase_06_encoder.md).

### E26: PRD §10.2 -- Missing `shr_r`, `L_shr_r`, `L_deposit_h`, `L_deposit_l` Operations

**Issue:** The basic operations table in PRD §10.2 omitted `shr_r` (16-bit rounding right shift), `L_shr_r` (32-bit rounding right shift), `L_deposit_h` (deposit into upper 16 bits), and `L_deposit_l` (deposit into lower 16 bits). All four are defined in `basic_op.c` and used in the codec.

**Affects:** PRD §10.2, [Implementation Plan Phase 1](../implementation/phase_01_dsp_math.md) test table (L_deposit boundary tests)

**Status:** APPLIED to PRD §10.2 (added 4 operations to table) and [Implementation Plan Phase 1](../implementation/phase_01_dsp_math.md) test table (added 4 L_deposit boundary tests, test count 25→29).

### E27: PRD §8.3 -- CNG Gaussian Normalization Oversimplified

**Issue:** PRD §8.3 described the Gaussian excitation normalization as `excg[i] = gauss × cur_gain / sqrt(Eg)`, which is a mathematical abstraction that does not capture the actual fixed-point computation pipeline. The reference code (`calcexc.c:125-141`) uses `FRAC1=19043` to compute the normalization factor, and the `Sqrt()` function returns `sqrt(Num/2)` (not `sqrt(Num)`), with `FRAC1` compensating for the factor-of-2 difference.

**Affects:** PRD §8.3, SPEC_PHASE_09_cng.md (Calc_exc_rand normalization)

**Status:** APPLIED to PRD §8.3 (replaced simplified formula with actual fixed-point pipeline) and [Specification Plan Batch 5](batch_05_vad_dtx_cng.md) (added Sqrt() semantics and FRAC1 compensation documentation).

### E28: PRD §2.2 -- Encoder "Previous Pitch Delay" is Not Persistent State

**PRD claim:** §2.2 Encoder state list includes "Previous pitch delay (for second subframe differential encoding)."

**Correction:** The encoder does NOT maintain a persistent `old_T0` field across frames. In the reference code (`cod_ld8a.c`), the first subframe's `T0` is computed via open-loop + closed-loop pitch search, and the second subframe's search is constrained relative to this SF1 `T0` value. The `T0` variable is frame-local — it does not carry over from the previous frame. This is unlike the decoder, which DOES maintain persistent `old_T0 = 60` across frames for frame erasure concealment (pitch continuation). The PRD listing is misleading because it suggests the encoder needs a persistent `old_T0` field in `EncoderState`, which it does not. Neither the Implementation Plan's encoder init table (rows 1-31) nor this Specification Plan's encoder init table includes an `old_T0` field, and this omission is correct.

**Reference:** `cod_ld8a.c` — `T0` and `T0_frac` are local variables in `Coder_ld8a()`, not static/global state. The second subframe's pitch search range is derived from SF1's `T0` within the same frame, not from a previous frame's pitch delay.

**Affects:** PRD §2.2 (encoder state list), SPEC_PHASE_06_encoder.md (EncoderState field inventory)

**Status:** APPLIED — PRD §2.2 encoder state list updated to clarify that "Previous pitch delay" is a within-frame local variable (SF1 T0 used as SF2 search center), not persistent cross-frame state. The decoder's `old_T0 = 60` remains the only persistent pitch delay state.

### E29: PRD §5.3 -- Pitch Delay Decoding Formula Error

**PRD claim:** First subframe decoding formula: `T_int = (index + 59) / 3 + 19`, `T_frac = index + 59 - 3*T_int`.

**Correction:** The formula was doubly wrong: (1) the offset `59` should be `2`, and (2) the extra `+ 19` outside the division makes the result 19 too large. The correct formula from the C reference code (`DEC_LAG3.C:37-44`) is `T0 = (index + 2) / 3 + 19` (integer division), `T0_frac = index - T0*3 + 58` (yields T0_frac in {-1, 0, 1}). Additionally, the second subframe formula was vague ("differential with 1/3-sample resolution") and lacked the explicit formula: `i = (index + 2) / 3 - 1`, `T0 = i + T0_min`, `T0_frac = index - 2 - i*3`.

**Reference:** `dec_lag3.c` lines 35-44 (SF1), lines 71-80 (SF2).

**Affects:** SPEC_PHASE_05_decoder.md (`pitch/lag_decode.rs`), SPEC_PHASE_06_encoder.md (`pitch/lag_encode.rs`)

**Status:** APPLIED — PRD §5.3 corrected with exact C reference code formulas for both subframes.

---

## 13. ITU Specification Errata (Spec-vs-Code Discrepancies)

These are discrepancies between the **ITU specification text** and the **ITU reference code**, where the code is authoritative for bit-exact conformance. These are NOT PRD errors — the PRD already documents the correct (code-based) values.

### SE1: Annex B §B.4.4 -- CNG Excitation Mixture Alpha

**Spec claim (B.4.4):** The CNG excitation is mixed as `ex(n) = 0.6*ex1(n) + beta*ex2(n)`, implying alpha=0.6.

**Code reality:** The ITU reference code uses alpha=0.5: `K0=24576 = (1-0.5^2)*32768` (`dtx.h:96`); the `calcexc.c:122` comment explicitly states "alpha = 0.5". PRD §8.3 correctly documents `K0=24576 (= 1 - alpha^2 with alpha=0.5, in Q15)`.

**Impact:** An implementer following only the spec text without consulting the code would use alpha=0.6, producing non-bit-exact CNG output and failing Annex B conformance tests.

**Resolution:** Follow the reference code (alpha=0.5). This is already correctly handled in [Specification Plan Batch 5](batch_05_vad_dtx_cng.md) and PRD §8.3.

### SE2: Annex B §B.4.1.2 -- DTX Minimum SID Interval (Nmin)

**Spec claim (B.4.1.2):** "a minimum interval of Nmin = 2 frames is required" and equation B.11 shows `count_fr >= Nmin` with `Nmin=2`.

**Code reality:** `FR_SID_MIN=3` (`dtx.h:78`), checked as `sub(count_fr0, FR_SID_MIN) < 0` (`dtx.c:150`). SID emission requires `count_fr0 >= 3`, not `>= 2`.

**Internal spec inconsistency:** The Annex B specification has a subtle self-contradiction. The **prose text** in §B.4.1.2 says "greater than Nmin = 2", which with `count_fr >= 3` matches the code (i.e., more than 2 means at least 3). However, **equation B.11** shows `count_fr >= Nmin` with `Nmin = 2`, which would mean `count_fr >= 2` — this does NOT match the code's `FR_SID_MIN = 3` check. An implementer following the equation literally (rather than the text) would allow SID emission one frame too early.

**Impact:** An implementer following only the spec equation B.11 would allow SID emission after 2 no-tx frames instead of 3, producing different DTX timing and failing Annex B encoder conformance tests.

**Resolution:** Follow the reference code (`FR_SID_MIN=3`). The spec text "greater than Nmin=2" is consistent with the code; equation B.11 is not. Already correctly handled in [Specification Plan Batch 5](batch_05_vad_dtx_cng.md) and [Implementation Plan Phase 8](../implementation/phase_08_dtx.md).

### SE3: Annex B Table B.1 -- MakeDec Conditions 8-10 Variable Mismatch

**Spec claim (Table B.1):** Conditions 8 and 9 reference `delta_El` (low-band energy difference) combined with `delta_ZC` (zero-crossing rate difference). Condition 10 references `delta_El` alone. These are listed under the `delta_El, delta_ZC` and `delta_El` column groupings in the table.

**Code comments (`vad.c:415`):** Say "dSLE vs dSZC" — using `dSLE` (the code variable for low-band energy), consistent with the spec's `delta_El`.

**Code reality (`vad.c:419,425,428`):** The actual executable code uses **`dSE`** (full-band energy), NOT `dSLE`. This is a three-way discrepancy: the spec text says `delta_El`, the code comments say `dSLE`, but the executable code uses `dSE`.

**Impact:** An implementer following the Annex B specification Table B.1 literally (or trusting the code comments) would use low-band energy for conditions 8-10. This produces incorrect VAD decisions and fails Annex B conformance tests. The executable code is authoritative.

**Resolution:** Follow the reference code (`dSE` for conditions 8-10). PRD §8.1.2 correctly documents the code-based conditions (E20) with a cross-reference note to this erratum.

### SE4: Annex A §A.3.8.1 -- No Pseudocode for Depth-First Tree Search

**Spec claim (A.3.8.1):** The fixed codebook uses an "iterative depth-first, tree search approach" but provides no algorithmic pseudocode, candidate evaluation counts, or detailed description of the search strategy.

**Code reality:** The complete depth-first tree search algorithm is implemented in `ACELP_CA.C` with specific track assignments, pulse position tables, and evaluation logic that can only be determined by reading the C code.

**Impact:** An implementer cannot derive a bit-exact fixed codebook search from the Annex A specification alone. All implementation details (track assignments, search order, candidate evaluation strategy) must come from the reference C code. The PRD and [Implementation Plan](../implementation/README.md) correctly source from `ACELP_CA.C`.

**Resolution:** Document as a known spec gap. The PRD §3.11.2 and [Implementation Plan Phase 6](../implementation/phase_06_encoder.md) already correctly derive all fixed codebook search details from the reference C code rather than the specification text.

### SE5: Annex A §A.4.2 -- "gt Not Used" Ambiguity for Tilt Compensation

**Spec claim (A.4.2):** "the gain terms gf and gt are not used."

**Code reality:** In base G.729, `gt` is a separate normalization gain factor applied to the tilt filter output (distinct from the `mu` coefficient inside H_t(z) = 1 + mu*z^-1). Annex A eliminates this separate `gt` factor (implicitly 1.0), but the tilt filter H_t(z) itself is retained with conditional mu: `gamma_t = 0.8` when k'_1 < 0, `gamma_t = 0.0` otherwise (see `POSTFILT.C` and PRD errata E11). The spec statement is technically correct — `gt` the gain normalization factor is indeed not used — but an implementer reading "gt is not used" in isolation might incorrectly disable the entire tilt compensation stage.

**Impact:** Moderate. PRD errata E10 (gf elimination) and E11 (conditional gamma_t) already document the correct behavior. This SE entry provides the clarifying bridge between the spec text and the errata.

**Resolution:** The spec's "gt" refers specifically to the post-filter gain normalization factor from base G.729 eq. 79, not to the tilt filter H_t(z) itself. Annex A sets gt = 1.0 (eliminated) and gf = 1.0 (eliminated) while retaining H_t(z) with conditional mu. Follow the reference code and PRD E10/E11.

### SE6: Annex A §A.3.4 -- OCR Artifact in Open-Loop Pitch Search Range

**Spec text (A.3.4):** The third delay search range reads "i = 3: 80,...,43" in the PDF.

**Code reality (`PITCH_A.C`):** The correct third range is [80, 143]. The "1" before "43" was dropped during OCR/PDF rendering, producing the nonsensical range "80,...,43" (where the upper bound is less than the lower bound).

**Impact:** Low for this project (PRD and [Implementation Plan](../implementation/README.md) already use the correct [80, 143] range from the C code), but an implementer consulting the Annex A PDF directly could be misled.

**Resolution:** Document as a known OCR artifact. The C reference code `PITCH_A.C` confirms the correct range is [80, 143]. The three open-loop pitch search delay ranges are: i=1: [80, 143], i=2: [40, 79], i=3: [20, 39] — searched in that order with halving logic.

### SE7: Annex B §B.3.2 -- VAD Reflection Coefficient Index

**Spec text (B.3.2):** States "first reflection coefficient r_1" is used for the background noise update condition and the forced-noise override condition.

**Code reality (`COD_LD8A.C:251-252`):** The VAD function is called with `rc[1]` (the **second** reflection coefficient, k_2), not `rc[0]` (k_1):

```c
vad(rc[1], lsf_new, r_h, r_l, exp_R0, p_window, frame,
    pastVad, ppastVad, &Vad);
```

The VAD uses this value for two threshold comparisons: `rc < 24576` (background noise update condition in `MakeDec()`) and `rc < 19661` (forced-noise override). Using `rc[0]` (k_1) instead of `rc[1]` (k_2) would produce incorrect VAD decisions, breaking Annex B conformance.

**Impact:** An implementer reading only the Annex B specification would use the wrong reflection coefficient. The C reference code is authoritative for bit-exact conformance.

**Resolution:** Follow the reference code: pass `rc[1]` (second reflection coefficient, k_2, index 1 from Levinson's `rc[2]` output) to the VAD function. This is already documented correctly in Phase 7's parameter table (updated to reference SE7).
