> Part of [G.729AB Implementation Plan](README.md)

### Phase 5: Decoder (Core G.729A)

**Goal**: Complete decoder with post-processing. Implemented before encoder — simpler, independently testable with ITU decoder test vectors.

**Files to create**:
- `src/codec/state/decoder_state.rs` — DecoderState struct with all decoder buffers
- `src/lsp_quant/decode.rs` — `D_lsp()`, `Lsp_iqua_cs()`: LSP decoding with erasure handling
- `src/lsp_quant/prev.rs` — MA prediction state management
- `src/lsp_quant/stability.rs` — `Lsp_stability()`, `Lsp_expand_1()`, `Lsp_expand_2()`, `Lsp_expand_1_2()` (combined expand called in `lspgetq.c`, `qua_lsp.c`, `qsidlsf.c`)
- `src/pitch/lag_decode.rs` — `Dec_lag3()`: pitch lag decoding
- `src/pitch/parity.rs` — `Check_Parity_Pitch()`
- `src/fixed_cb/decode.rs` — `Decod_ACELP()`: reconstruct 4-pulse vector
- `src/fixed_cb/build_code.rs` — Build excitation vector from decoded pulse positions/signs
- `src/gain/decode.rs` — `Dec_gain()`: gain decoding with MA prediction
- `src/gain/predict.rs` — `Gain_predict()`, `Gain_update()`, `Gain_update_erasure()`
- `src/postfilter/pitch_pf.rs` — Pitch post-filter (integer delays, Annex A)
- `src/postfilter/formant.rs` — Formant post-filter (gamma_n=0.55, gamma_d=0.70)
- `src/postfilter/agc.rs` — Adaptive gain control
- `src/postfilter/pipeline.rs` — Orchestrate full post-filter chain
- `src/postproc.rs` — Post_Process: 2nd-order HP filter at ~100 Hz (b100=[7699,-15398,7699] Q13, a100=[8192,15836,-7667] Q13) + upscaling by 2. Filter state MUST use DPF split high/low precision (y1_hi, y1_lo, y2_hi, y2_lo) matching the pre-processing filter design — plain 16-bit state will cause bit-exact failures.
- `src/codec/erasure.rs` — Frame erasure concealment (gain attenuation, pitch continuation)
- `src/codec/decode.rs` — `Decod_ld8a()`: main frame decoder loop
- `src/codec/decode_sub.rs` — Per-subframe decoder logic

**Reference**: `dec_ld8a.c`, `lspdec.c`, `dec_lag3.c`, `de_acelp.c`, `dec_gain.c`, `postfilt.c`, `post_pro.c`

**Total**: ~30 functions, ~1,800 lines

**Tasks**:
1. Define `DecoderState` with all buffers and initialize to exact ITU values (PRD §2.2.1):
   - `lsp_old` = {30000, 26000, 21000, 15000, 8000, 0, -8000, -15000, -21000, -26000}
   - `sharp` = 3277 (SHARPMIN), `old_T0` = 60, `gain_pitch` = 0, `gain_code` = 0
   - `past_qua_en` = {-14336, -14336, -14336, -14336}
   - `freq_prev[0..3]` = each row = {2339, 4679, 7018, 9358, 11698, 14037, 16377, 18717, 21056, 23396}
   - `prev_ma` = 0 (previous MA predictor mode, used by frame erasure path to extract correct LSP prediction residual; see `LSPDEC.C`)
   - `prev_lsp[M]` = {2339, 4679, 7018, 9358, 11698, 14037, 16377, 18717, 21056, 23396} (Q13, = freq_prev_reset; used by frame erasure path in `Lsp_iqua_cs()` to extract MA prediction residual; distinct from `lsp_old` which is the previous decoded LSP in Q15; see `LSPDEC.C:40`)
   - `seed_fer` = 21845 (frame erasure LCG seed), `past_ftyp` = 1, `bad_lsf` = 0, CNG `seed` = INIT_SEED = 11111 (comfort noise LCG seed)
   - `sid_sav` = 0, `sh_sid_sav` = 1 (SID energy fallback state for CNG — used when first SID frame after speech is erased; see `DEC_LD8A.C:104-105`)
   - `sid_gain` = tab_Sidgain[0] (= 2) (initial CNG gain value; see `DEC_SID.C` Init_Dec_cng). **C call-site note:** `sid_gain`, `cur_gain` (=0), and `lspSid[M]` (={31441,...}) are initialized by `Init_Dec_cng()` in `dec_sid.c`, which is called from `decoder.c`'s `main()` — NOT by `Init_Decod_ld8a()` in `dec_ld8a.c`. In Rust, both C initialization call sites collapse into `DecoderState::new()`. `cur_gain` and `lspSid` are documented in Phase 9 (CNG) where their functional usage is specified
   - Post-filter state: `res2_buf[PIT_MAX+L_SUBFR]` = 0, `scal_res2_buf[PIT_MAX+L_SUBFR]` = 0, `mem_syn_pst[M]` = 0, `mem_pre` = 0 (tilt compensation preemphasis memory, persists across subframes/frames; `POSTFILT.C:351`), `past_gain` = 4096 (Q12, = 1.0 — AGC smoothed gain; see `POSTFILT.C`. **Verified:** `postfilt.c:384` explicitly declares `static Word16 past_gain=4096;` with comment `/* past_gain = 1.0 (Q12) */`, confirming initialization to unity gain). Buffer sliding-window pattern: C code uses `res2 = res2_buf + PIT_MAX` pointer arithmetic so `res2[-PIT_MAX..-1]` accesses past data for pitch post-filter correlation; `Copy()` at `postfilt.c:191-192` shifts the buffer after each subframe. Rust: use index-based access with constant offset `PIT_MAX` (e.g., `&res2_buf[PIT_MAX..PIT_MAX+L_SUBFR]` for current, `&res2_buf[PIT_MAX-t0_max..PIT_MAX]` for pitch search)
   - All other filter memories = 0, all excitation buffers = 0 (PRD §2.2.1)
2. Implement LSP decoding with frame erasure handling (repeat previous LSPs on BFI); `D_lsp()` receives `bfi | bad_lsf`. **`bad_lsf` behavior** (`decoder.c:27-38`): `bad_lsf` is a global variable initialized to 0 and is **never set by the codec itself** — it is an external hook for channel protection schemes. Per the reference code comment: "This variable should be always set to zero unless transmission errors in LSP indices are detected." For bit-exact conformance with ITU test vectors, `bad_lsf` must remain 0 throughout. In a production SIP deployment, the transport layer could set it to 1 when channel coding detects LSP index corruption (distinct from full frame erasure). The Rust implementation should expose `bad_lsf` as a field on `DecoderState` (default 0) or accept it as an optional parameter to `decode()`
3. Implement pitch delay decoding: P1 (8-bit) first subframe, P2 (5-bit differential) second subframe
4. Implement parity check on P1's 6 MSBs
5. Implement fixed codebook reconstruction from 13-bit position + 4-bit sign
6. Implement pitch sharpening: `code[i] += code[i-T0] * sharp` when T0 < 40
7. Implement gain decoding with MA prediction and erasure handling. During erasure, `Gain_update_erasure(past_qua_en)` IS called (`gainpred.c:137-158`): it computes `avg = mean(past_qua_en) - 4.0 dB` (Q10: subtract 4096), clamps to -14.0 dB (-14336 Q10), shifts the array, and stores the result in `past_qua_en[0]`. This is a different update path than normal `Gain_update()` — the actual attenuated concealment gains (0.9*g_p, 0.98*g_c) are NOT fed back into the MA predictor, but past_qua_en IS modified with a decaying average. **PRD §6.2 erratum:** PRD §6.2 states past_qua_en is "not updated" during erasure — this is incorrect. The array IS modified via `Gain_update_erasure()`, which computes a decaying average. The reference code (`gainpred.c:137-158`, called from `dec_gain.c:67`) is authoritative
8. Implement LP synthesis filter with overflow handling: trial synthesis, if overflow detected then scale excitation >>2 and re-synthesize (PRD §5.8.1)
9. Implement `sid_sav`/`sh_sid_sav` excitation energy computation (`DEC_LD8A.C:353-361`): after the subframe loop, on **every good frame** (`bfi == 0`, regardless of speech or SID frame type), compute excitation energy `L_temp = Σ L_mac(0, exc[i], exc[i])`, then `sh_sid_sav = norm_l(L_temp)`, `sid_sav = round(L_shl(L_temp, sh_sid_sav))`, `sh_sid_sav = sub(16, sh_sid_sav)`. These values provide a fallback energy estimate for CNG when the first SID frame after speech is erased (consumed by Phase 9 task 7). This computation MUST be in the decoder main loop (Phase 5), not deferred to CNG (Phase 9), because it runs on every good frame including speech frames
10. Implement post-filter chain operating in the **spectrally shaped residual domain** (PRD §5.9.1). The pipeline is NOT applied directly to synthesized speech — the critical multi-step flow is: (a) compute residual through `A(z/GAMMA2_PST)` (γ_n=0.55, numerator of H_f) via `Weight_Az(Az, GAMMA2_PST, Ap3); Residu(Ap3, synth, res2)` — NOT through plain `A(z)`, (b) scale residual by >>2 for overflow avoidance: `scal_res2[j] = shr(res2[j], 2)`, (c) apply pitch post-filter — search ±3 integer delays around decoded pitch in the **scaled** residual (clamped to PIT_MAX), compute correlation gain via `pit_pst_filt()` (`postfilt.c:296-334`): jointly normalize `cor_max`/`ener`/`ener0` via `norm_l`, test 3 dB threshold (`cmax² < 0.5*en*en0`), then derive gains — if `cmax > en` (pitch gain > 1): `g0 = INV_GAMMAP` (21845), `gain = GAMMAP_2` (10923); otherwise: `gain = div_s(cmax*GAMMAP/2, cmax*GAMMAP/2 + en/2)` (Q15), `g0 = 32767 - gain`; apply `signal_pst[i] = g0*signal[i] + gain*signal[i-T]` to the **unscaled** `res2`; when `Vad==0`, bypass pitch PF entirely (copy `res2` to `res2_pst`), (d) compute tilt compensation from the **combined post-filter impulse response** `h_f` of `A(z/GAMMA2_PST)/A(z/GAMMA1_PST)` (not from LP analysis reflection coefficient) — tilt is applied to the residual-domain signal `res2_pst` BEFORE synthesis (Annex A spec A.4.2.3: "The compensation filtering H_t(z) is performed before synthesis through 1/Â(z/γ_d)"; reference: `postfilt.c:154-184`), (e) re-synthesize through `1/A(z/GAMMA1_PST)` (γ_d=0.70, denominator of H_f) via `Syn_filt(Ap4, res2_pst, output, mem_syn_pst)` — NOT through plain `A(z)`, (f) apply AGC. `Post_Filter()` receives a `Vad` flag (PRD §5.9.5): during CNG (`Vad=0`), the pitch post-filter is bypassed entirely and `old_T0` from the last speech frame is preserved. **PRD §5.9.1-5.9.3 erratum (E14):** Three errors in PRD: (a) §5.9.1 says residual uses `A(z/gamma_d)` (0.70) but code uses `A(z/gamma_n)` (0.55, GAMMA2_PST). (b) §5.9.2 formula says `Syn_filt(A(z/gamma_n), ...)` but code uses `A(z/gamma_d)` (0.70, GAMMA1_PST); the table in §5.9.2 is correct, only the formula text has subscripts swapped. (c) §5.9 lists synthesis (§5.9.2) before tilt (§5.9.3), but reference code applies tilt BEFORE synthesis. The reference code (`postfilt.c:133-184`) and Annex A spec A.4.2.3 are authoritative
11. Implement frame erasure concealment behavior: the Annex A reference decoder (`DEC_LD8A.C`) has **no explicit voicing classification code**. The voiced/unvoiced distinction described in PRD §6.4 emerges naturally from the previous pitch gain magnitude — high previous `gain_pitch` produces pitch-coherent concealment, low gain produces noise-like output. There is no explicit ">3 dB prediction gain" check in the reference code. Random excitation during erasure uses `Random(&seed_fer)` (see below for seed handling). **PRD §6.4 erratum:** PRD §6.4 describes a voicing classification using ">3 dB prediction gain" — this check does not exist in the Annex A reference decoder. The voiced/unvoiced behavior is emergent from pitch gain magnitude, not a discrete classification
12. Implement frame erasure concealment: pitch gain `g_p = min(0.9 * g_p_prev, 0.9)` (multiplicative decay AND cap at 0.9 Q15=29491), code gain `g_c = 0.98 * g_c_prev` (Q15=32111, decay only), random codebook indices (`index = Random(&seed_fer) & 0x1FFF`, `sign = Random(&seed_fer) & 0x000F`). **Pitch continuation during erasure** (`DEC_LD8A.C:232-267`): the `old_T0` increment happens **inside** each subframe's bad-pitch branch, not after the frame. For a full frame erasure: SF1 uses `T0 = old_T0` (value X), then `old_T0 += 1` (now X+1); SF2 uses `T0 = old_T0` (value X+1), then `old_T0 += 1` (now X+2). Both subframes set `T0_frac = 0`. The net effect is old_T0 increments by **2 per erased frame** (1 per subframe), capped at PIT_MAX=143 after each increment. **PRD §6.2 erratum:** PRD §6.2 and the ITU specification text both describe this as "unchanged for both subframes, increment by 1 after frame" — this does not match the reference code. The reference code's per-subframe increment is authoritative for bit-exactness. **Random() seed handling**: The g729ab_v14 `Random(Word16 *seed)` takes a seed pointer parameter (`UTIL.C:55-61`). The decoder maintains **two separate seeds**: `seed_fer` (init=21845) for frame erasure concealment, and `seed` (init=INIT_SEED=11111) for CNG excitation. Both use the same LCG: `seed = extract_l(L_add(L_shr(L_mult(seed, 31821), 1), 13849))`
13. **Recovery after frame erasure** (PRD §6.3): The reference decoder (`DEC_LD8A.C`) has **no explicit gain limiting or smoothing** on the first good frame after erasure. Normal decoding resumes immediately — `D_lsp()` decodes LSPs normally, `Dec_gain()` calls `Gain_predict()` then `Gain_update()` with the actual quantized gain. Recovery stability comes from the `Gain_update_erasure()` behavior during erasure (task 7): `past_qua_en` converges toward a floor of -14.0 dB during erasure, so when normal decoding resumes, the MA predictor uses an energy level that has decayed toward the floor rather than tracking the artificial concealment gains. **PRD §6.3 erratum:** PRD §6.3 describes "gain limiting/smoothing during the first good frame" — this is emergent behavior from the `Gain_update_erasure()` mechanism (task 7), not a separate code path. The reference decoder has no explicit gain limiting or smoothing on recovery.

**Static/helper functions included in tasks above** (for C-to-Rust mapping completeness):
- State management accessors `Lsp_decw_reset()`, `Get_decfreq_prev()`, `Update_decfreq_prev()` in `LSPDEC.C`: in Rust, these become methods on `DecoderState` (e.g., `decoder_state.freq_prev()`, `decoder_state.update_freq_prev()`). No standalone functions needed
- **`prev_lsp` vs `lsp_old` distinction** (`LSPDEC.C`): `prev_lsp[M]` (initialized from `freq_prev_reset`) and `lsp_old[M]` serve different purposes. `lsp_old` is the previous frame's decoded LSP vector (used for interpolation). `prev_lsp` is specifically used by the frame erasure path in `Lsp_iqua_cs()` to extract the MA prediction residual from the last good frame. These must be separate fields in `DecoderState`; they cannot be merged without breaking frame erasure recovery

**Test Plan**:

| Test | Input File | Expected Output | Validates |
|------|-----------|-----------------|-----------|
| ALGTHM.BIT -> ALGTHM.PST | ITU test vector | Bit-exact PCM match | Algorithm coverage paths |
| SPEECH.BIT -> SPEECH.PST | ITU test vector | Bit-exact PCM match | Generic speech |
| ERASURE.BIT -> ERASURE.PST | ITU test vector | Bit-exact PCM match | Frame erasure concealment |
| OVERFLOW.BIT -> OVERFLOW.PST | ITU test vector | Bit-exact PCM match | Synthesis overflow handling |
| PARITY.BIT -> PARITY.PST | ITU test vector | Bit-exact PCM match | Parity error handling |
| PITCH.BIT -> PITCH.PST | ITU test vector | Bit-exact PCM match | Pitch decoding |
| LSP.BIT -> LSP.PST | ITU test vector | Bit-exact PCM match | LSP decoding |
| FIXED.BIT -> FIXED.PST | ITU test vector | Bit-exact PCM match | Fixed codebook |
| TAME.BIT -> TAME.PST | ITU test vector | Bit-exact PCM match | Taming procedure |
| TEST.BIT -> TEST.pst | ITU test vector | Bit-exact PCM match | Additional general coverage |

**Verification method**: Sample-by-sample comparison of 16-bit PCM output. **Zero tolerance — every sample must match exactly.**

> **Reference conformance note:** bcg729 is NOT a valid bit-exact reference for the OVERFLOW test vector. bcg729 handles synthesis overflow via per-sample saturation instead of the ITU reference's retry-with-scaling approach (detect Overflow flag, scale excitation >>2, re-synthesize). Only the ITU reference decoder output (`g729ab_v14`) is authoritative for all test vectors. Similarly, bcg729's encoder output will differ from the ITU reference due to a different fixed codebook search algorithm (~576 nested-loop candidate evaluations vs ~160 depth-first tree candidates). Use only the ITU reference code for bit-exact validation.

> **Note:** The test vector directory also contains an undocumented TEST vector set (`TEST.BIT`, `TEST.IN`, `TEST.pst`) referenced by `TEST.BAT` but not listed in `READMETV.txt`. This provides a 10th decoder test pair and 7th encoder test pair. Include it in conformance testing for additional coverage.

**CONFORMANCE CHECKPOINT 1**: All 10 decoder test vectors pass bit-exactly before proceeding to Phase 6.

**Exit Criteria**:
1. All 10 Annex A decoder test vectors pass bit-exactly (zero sample deviation)
2. Overflow handling path is covered and deterministic (OVERFLOW.BIT exercises it)
3. Frame erasure concealment matches reference exactly (ERASURE.BIT)
4. Parity error path tested (PARITY.BIT)

**TDD Workflow**:
1. **Write integration tests first**: Create `tests/integration/decoder_conformance.rs` with 10 test functions, one per ITU decoder test vector. Each loads a `.BIT` file, decodes it, and compares the output sample-by-sample against the `.PST` reference. All 10 tests fail initially.
   ```
   cargo test --test decoder_conformance --features itu_serial  # expect: 0 passed
   ```
2. **Implement incrementally**: Build decoder modules in dependency order — DecoderState -> LSP decode -> pitch decode -> parity -> fixed CB decode -> gain decode -> synthesis filter -> post-filter -> erasure concealment -> main decoder loop. Run a simpler vector (ALGTHM) first; once it passes, tackle harder ones (ERASURE, OVERFLOW).
3. **Verify**:
   ```
   cargo test --test decoder_conformance --features itu_serial  # all 10 pass
   python tests/scripts/run_all_tiers.py --phase 5              # exits 0 (Gate 3)
   ```
4. **Gate 3 (Conformance Checkpoint 1)**: `run_all_tiers.py --phase 5` reports all 10 decoder vectors bit-exact. Must pass before proceeding to Phase 6.
