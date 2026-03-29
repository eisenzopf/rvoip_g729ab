> Part of [G.729AB Implementation Plan](README.md)

### Phase 9: Annex B — Comfort Noise Generation (CNG)

**Goal**: Implement decoder-side CNG that generates background noise during silence.

**Files to create**:
- `src/annex_b/cng/state.rs` — CngDecState struct (encoder-side CNG state lives on DtxState and EncoderState; no separate CngEncState needed)
- `src/annex_b/cng/decode.rs` — `Dec_cng()`: SID decoding + noise synthesis
- `src/annex_b/cng/excitation.rs` — `Calc_exc_rand()`: random excitation generation. Takes a `flag_cod` parameter (`FLAG_COD=1` for encoder, `FLAG_DEC=0` for decoder) that controls whether `update_exc_err()` is called inside the CNG loop to maintain taming state — encoder must update taming, decoder must not. Reference: `calcexc.c:59,258`
- (Also uses `src/annex_b/cng/sid.rs` — created in Phase 8, shared with Phase 9 for SID parameter handling)

**Reference**: `dec_sid.c`, `calcexc.c`

**Total**: ~5 functions, ~450 lines

**Tasks**:
1. Define CngDecState with SID parameters, gain smoothing state (`cur_gain = 0` initial value — C static default; affects gain smoothing on first noise frame), CNG seed (INIT_SEED=11111)
2. Implement SID frame decoding: `sid_lsfq_decode()` using `noise_fg` and `noise_fg_sum[MODE][M]` tables (from `tables/sid.rs`; `dec_sid.c:179-180` passes `noise_fg_sum[index[0]]` to `Lsp_prev_compose`) with separate stability enforcement (min spacing = 10 Q13). NOTE: The decoder's `sid_lsfq_decode()` in `dec_sid.c` uses an **inline gap enforcement loop** (`dec_sid.c:166-176`) that ensures minimum LSF spacing, followed by `Lsp_stability(lsfq)` (`dec_sid.c:186`). It does NOT call `Lsp_expand_1_2`. This differs from the encoder's `lsfq_noise()` in `qsidlsf.c` which calls `Lsp_expand_1_2(tmpbuf, 10)` (`qsidlsf.c:111`) for the same purpose. Both approaches are distinct from speech LSP stability which uses a **two-pass** approach (`Lsp_expand_1` with GAP1=10 then `Lsp_expand_2` with GAP2=5, in `lspgetq.c`). This is a cross-phase dependency on Phase 5's `lsp_quant/stability.rs`
3. Implement CNG excitation (`Calc_exc_rand` in `CALCEXC.C`): random pitch [40-103], Gaussian + ACELP, quadratic gain solve — PRD §8.3. The excitation generation follows a two-stage process per subframe:
   - **Stage 1:** Compose preliminary excitation `cur_exc[i] = Gp2*adaptive[i] + excg[i]`, rescale to `excs[]` for overflow avoidance, compute interaction term `b` from `excs[]` at pulse positions, then solve `4X^2 + 2bX + c = 0` for Gf where `c = Gp²*Ea² - K0*cur_gain²`. Select the root with the **lowest absolute value** (Annex B spec B.4.4)
   - **Discriminant-negative fallback** (`calcexc.c:208-232`): when `b² - 4c < 0`, the adaptive contribution is abandoned — `cur_exc[]` is overwritten with `excg[]` (Gaussian only), `Gp` is set to 0, `b` is recomputed from `excg[]` at pulse positions, and a second quadratic is solved using `delta = K0*k + b²` (always non-negative, since `K0*k >= 0` and `b² >= 0`). This path fires when adaptive+Gaussian energy exceeds the target energy budget
   - **Gain cap:** `|Gf| > G_MAX (=5000)` → bilateral clamp (`calcexc.c:240-245`)
   - **Stage 2:** Add signed ACELP pulses at gain Gf to `cur_exc[]` (`calcexc.c:248-256`)
   - The function takes a `flag_cod` parameter: `FLAG_COD=1` (encoder) or `FLAG_DEC=0` (decoder). When `flag_cod == FLAG_COD`, `update_exc_err(Gp, t0)` is called inside the CNG subframe loop (`calcexc.c:59,258`) to keep the taming state (`L_exc_err[4]`) current during DTX — without this, the encoder's taming behavior is incorrect when speech resumes after silence. When `flag_cod == FLAG_DEC`, the taming update is skipped. In Rust, map this to a `bool` parameter (e.g., `update_taming: bool`). The decoder calls with `false` (`dec_sid.c:113`), the encoder's `Cod_cng` calls with `true` (`dtx.c:216`). Static helper functions within `calcexc.c`: `Gauss(seed)` generates Gaussian random values via sum of 12 uniform randoms; `Sqrt(x)` computes integer square root. Both are internal to the excitation generation and map to helpers in `annex_b/cng/excitation.rs`
4. Implement gain smoothing: `cur_gain = mult_r(A_GAIN0, cur_gain) + mult_r(A_GAIN1, sid_gain)` where A_GAIN0=28672 (0.875 Q15), A_GAIN1=4096 (0.125 Q15). Both multiplications use `mult_r` (rounding multiply, `dec_sid.c:109-110`), not `mult` — using `mult` produces off-by-one errors causing bit-exact failures. After gain smoothing and `Calc_exc_rand()`, `Dec_cng()` calls `Int_qlpc(lsp_old, lspSid, A_t)` to interpolate between the previous LSP and the SID LSP for per-subframe LP coefficients (`dec_sid.c:116`), then copies `lspSid` to `lsp_old` (`dec_sid.c:117`: `Copy(lspSid, lsp_old, M)`) so subsequent frames interpolate from the current SID LSP. Both steps are essential for correct CNG synthesis
5. Integrate with decoder main loop:
   - `ftyp==1` -> speech decode + reset CNG seed to INIT_SEED (PRD §8.4.2)
   - `ftyp!=1` -> sharp=SHARPMIN + CNG decode
6. Handle transitions: first noise after speech (step gain), subsequent (smooth), speech resume (normal decode)
7. Handle `sid_sav`/`sh_sid_sav` recovery when first SID after speech is erased — PRD §6.5
8. Initialize `lspSid` to separate initial values: {31441, 27566, 21458, 13612, 4663, -4663, -13612, -21458, -27566, -31441} — PRD §12 note
9. Implement BFI handling with Annex B active (PRD §6.5): when `bfi==1 && past_ftyp==1`, set `ftyp = 1` (speech erasure path); when `bfi==1 && past_ftyp==0 or 2`, set `ftyp = 0` (DTX erasure, continue CNG with previous SID parameters). Write `*parm = ftyp` to update parm[1] (V1.3 maintenance update for correct DTX interaction). **No forced parity error is needed** — the pitch concealment path for subframe 1 is triggered naturally by `bad_pitch = add(bfi, parity_result)` which is non-zero whenever bfi=1, causing subframe 1 to use `old_T0` without decoding a corrupted value. The parity check itself is performed in `read_frame()` (bits.c) before the decoder is called. Reference: `dec_ld8a.c` lines 147-155 and 234-235

**Test Plan**:

| Test | Input | Expected | Validates |
|------|-------|----------|-----------|
| SID frame decode | Known SID parameters | Correct LSP + gain | SID parsing |
| CNG output | Extended silence | Stable comfort noise, no artifacts | CNG quality |
| Speech -> noise transition | Sequence with transition | Smooth gain transition | Transition handling |
| Noise -> speech transition | Sequence with resume | Immediate normal decode | Resume handling |
| tstseq1a.bit -> tstseq1a.out | Annex B decoder test | Bit-exact PCM | Decoder conformance |
| tstseq2a.bit -> tstseq2a.out | Annex B decoder test | Bit-exact PCM | Sequence 2 |
| tstseq3a.bit -> tstseq3a.out | Annex B decoder test | Bit-exact PCM | Sequence 3 |
| tstseq4a.bit -> tstseq4a.out | Annex B decoder test | Bit-exact PCM | Sequence 4 |
| tstseq5.bit -> tstseq5a.out | Decoder-only SID test | Bit-exact PCM | SID-only decode |
| tstseq6.bit -> tstseq6a.out | Decoder BFI+SID test | Bit-exact PCM | Erasure during DTX |

**CONFORMANCE CHECKPOINT 3**: All Annex B test vectors pass bit-exactly. This checkpoint is reached when Phases 7, 8, and 9 are all complete — the 4 encoder test vectors require VAD (Phase 7) + DTX (Phase 8), while the 6 decoder test vectors require CNG (Phase 9).

**Exit Criteria** (Phase 9 alone — CNG decoder):
1. All 6 Annex B **decoder** test vectors (tstseq1a-4a + tstseq5-6) pass bit-exactly
2. CNG seed reset behavior verified (reset on every ftyp==1 frame)
3. DTX/CNG transitions reproduce expected frame types and gains
4. OCTET_TX_MODE SID frames handled correctly (16-bit aligned)

**TDD Workflow**:
1. **Write tests first**: Add 6 decoder test functions to `annex_b_conformance.rs` (tstseq1a-4a + tstseq5-6). Each decodes a bitstream containing SID/no-tx frames and compares PCM output against reference. Write unit tests for CNG seed reset, SID frame decoding, and gain smoothing.
   ```
   cargo test --test annex_b_conformance --features annex_b,itu_serial  # 4 encoder pass, 6 decoder fail
   ```
2. **Implement**: CngDecState -> SID frame decode -> CNG excitation -> gain smoothing -> decoder integration (ftyp routing, seed reset, sid_sav recovery).
3. **Verify**:
   ```
   cargo test --test annex_b_conformance --features annex_b,itu_serial  # all 10 pass
   python tests/scripts/run_all_tiers.py --phase 9                      # exits 0 (Gate 5)
   ```
4. **Gate 5 (Conformance Checkpoint 3)**: `run_all_tiers.py --phase 9` reports all 10 Annex B vectors bit-exact, plus all 17 Annex A vectors still passing.
