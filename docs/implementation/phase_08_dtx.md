> Part of [G.729AB Implementation Plan](README.md)

### Phase 8: Annex B — Discontinuous Transmission (DTX)

**Goal**: Implement DTX state machine and SID frame encoding on encoder side.

**Files to create**:
- `src/annex_b/dtx/state.rs` — DtxState struct (autocorrelation history, filter averages)
- `src/annex_b/dtx/encode.rs` — `Cod_cng()`, `Update_cng()`: CNG encoding
- `src/annex_b/dtx/stationarity.rs` — Itakura distance, filter averaging, stationarity detection
- `src/annex_b/cng/sid.rs` — `lsfq_noise()`, `Qua_Sidgain()`: SID parameter quantization. Internal static helper `Qnt_e()` (codebook search within `lsfq_noise`, `qsidlsf.c`) maps to a private helper within the `lsfq_noise` implementation

**Reference**: `dtx.c`, `qsidlsf.c`, `qsidgain.c`

**Total**: ~15 functions, ~900 lines

**Tasks**:
1. Define DtxState with autocorrelation history (NB_SUMACF=3 sets x NB_CURACF=2 frames = 6-frame history). Initialize per `Init_Cod_cng()` in `DTX.C`. **Call-site note:** In C, `Init_Cod_cng()` is called from `coder.c` main(), separate from `Init_Coder_ld8a()`. In Rust, both initialization flows are collapsed into `G729Encoder::new()`, which constructs both `EncoderState` and `DtxState` with their respective initial values. Complete field list from `dtx.c` static variables:
   - `Acf[SIZ_ACF]` = 0 (autocorrelation history; `DTX.C:29`)
   - `sh_Acf[NB_CURACF]` = {40, 40} (`DTX.C:30`)
   - `sumAcf[SIZ_SUMACF]` = 0 (accumulated autocorrelation; `DTX.C:31`)
   - `sh_sumAcf[NB_SUMACF]` = {40, 40, 40} (`DTX.C:32`)
   - `ener[NB_GAIN]` = 0 (energy history; `DTX.C:34`)
   - `sh_ener[NB_GAIN]` = {40, 40} (`DTX.C:35`)
   - `lspSid_q[M]` = 0 (quantized LSP for SID frame; `DTX.C:25`)
   - `pastCoeff[MP1]` = 0 (past filter coefficients for stationarity check; `DTX.C:26`)
   - `RCoeff[MP1]` = 0 (autocorrelation of filter coefficients; `DTX.C:27`)
   - `sh_RCoeff` = 0 (scaling for RCoeff; `DTX.C:28`)
   - `fr_cur` = 0, `cur_gain` = 0, `flag_chang` = 0 (`DTX.C:36,38,39`)
   - `nb_ener` = 0 (energy sample count; incremented via `nb_ener = add(nb_ener, 1)` each frame in `Update_cng()` (`DTX.C:104`), capped at `NB_GAIN` (=2). Passed to `Qua_Sidgain(ener, sh_ener, nb_ener, ...)` as the valid energy count — when `nb_ener < NB_GAIN`, some energy slots in `ener[NB_GAIN]` are uninitialized, affecting SID gain quantization on the first few noise frames. Reference: `DTX.C:37,104`, `QSIDGAIN.C`)
   - `sid_gain` = 0 (encoder-side SID gain; `DTX.C:38` — distinct from decoder `sid_gain`)
   - `prev_energy` = 0 (previous SID energy for change detection; `DTX.C:40`)
   - `count_fr0` = 0 (frame count since last SID, controls FR_SID_MIN logic; `DTX.C:41`)
   - **Rust initialization note:** In C, `Init_Cod_cng()` (`DTX.C:57-75`) explicitly initializes **9 fields**: `sumAcf` (=0), `sh_sumAcf` (={40,40,40}), `Acf` (=0), `sh_Acf` (={40,40}), `sh_ener` (={40,40}), `ener` (=0), `cur_gain` (=0), `fr_cur` (=0), `flag_chang` (=0). The remaining **8 fields** — `lspSid_q`, `pastCoeff`, `RCoeff`, `sh_RCoeff`, `nb_ener`, `sid_gain`, `prev_energy`, `count_fr0` — are NOT touched by `Init_Cod_cng()` and rely on C's implicit zero-initialization for file-scope static variables. In Rust, `DtxState::new()` must explicitly initialize ALL fields, including those that are zero — Rust has no implicit zero-initialization for struct fields. Omitting any field from `new()` is a compile error, so this is enforced by the language, but the documentation here is important for understanding which C values are intentional vs. incidental
2. Implement DTX state machine: VOICE -> emit SID -> count_fr0 < FR_SID_MIN(3) -> no-tx -> stationarity check -> SID or no-tx — PRD §8.2
3. Implement autocorrelation averaging for SID filter. **Cod_cng() internal Levinson recursion** (`dtx.c:117-123`): After computing the averaged autocorrelation via `Calc_sum_acf()`, `Cod_cng()` calls `Levinson(curAcf, zero, curCoeff, bid, &ener[0])` to derive LP coefficients from the accumulated autocorrelation. This is a **separate** Levinson call from the main encoder LP analysis (which runs on per-frame windowed speech). The SID Levinson operates on averaged noise autocorrelation and produces `curCoeff` for stationarity analysis and `ener[0]` for SID gain quantization. The Rust implementation must call the same `levinson()` function from Phase 3's `lp/levinson.rs` with the accumulated autocorrelation
4. Implement Itakura distance stationarity detection (FRAC_THRESH1=4855, FRAC_THRESH2=3161)
5. Implement SID LSF quantization (`lsfq_noise` in `qsidlsf.c`) using reduced codebooks (PtrTab_1[32], PtrTab_2[2][16]) and `noise_fg_sum[MODE][M]` / `noise_fg_sum_inv[MODE][M]` tables (`tab_dtx.c`; used by `lsfq_noise` in `qsidlsf.c:101,115,241,250`). **Pre-VQ boundary enforcement** (`qsidlsf.c:81-89`): Before the VQ codebook search, the raw LSF vector must undergo boundary clamping: `lsf[0] >= L_LIMIT`, successive gaps `lsf[i+1] - lsf[i] >= 2*GAP3`, `lsf[M-1] <= M_LIMIT`, and a final back-off `lsf[M-2] = lsf[M-1] - GAP3` if ordering is violated. This occurs before `Get_wegt()` and `Lsp_prev_extract()`. Post-VQ, `Lsp_expand_1_2(tmpbuf, 10)` is called on the quantized error vector (`qsidlsf.c:111`), followed by `Lsp_stability(lsfq)` (`qsidlsf.c:121`)
6. Implement SID gain quantization (tab_Sidgain[32])
7. Pre-compute `noise_fg[1]` as a `const` array in `tables/sid.rs`: `noise_fg[1][i][j] = (19660*fg[0][i][j] + 13107*fg[1][i][j]) >> 15` — PRD §8.4.1. The C reference computes these at runtime in `Init_lsfq_noise()`; in Rust, the values are deterministic from the `fg` table and should be compile-time constants to avoid runtime initialization and mutable state
8. Implement encoder conditional processing during DTX (PRD §8.1.5, `COD_LD8A.C:260-302`). When `Vad==0 && vad_enable==1`, the encoder bypasses the normal subframe loop and instead:
   - **Still performed**: LP analysis (Autocorr, Lag_window, Levinson, Az_lsp) — these run unconditionally before the VAD check. VAD itself and `Update_cng(rh_nbe, exp_R0, Vad)` also run unconditionally.
   - **LSP prediction memory save/restore** (`cod_ld8a.c:262-264`): Before calling `Cod_cng()`, the encoder calls `Get_freq_prev(lsp_old_q)` (line 262) to save the LSP MA prediction memory (`freq_prev[MA_NP][M]`) into `lsp_old_q`. After `Cod_cng()` returns, `Update_freq_prev(lsp_old_q)` (line 264) writes the updated values back. This brackets the `Cod_cng()` call because `lsfq_noise` inside `Cod_cng()` calls `Lsp_prev_update` which modifies `freq_prev`. Without this save/restore, the LSP prediction memory is corrupted during DTX, causing incorrect LSP quantization when speech resumes after silence and failing Annex B encoder conformance tests (tstseq1-4). In Rust, this maps to explicit get/set operations on the shared `freq_prev` field of `LspQuantState`
   - **Replaced**: `Cod_cng()` is called instead of the subframe loop — it handles SID/no-tx frame generation using the DTX state machine. **Cross-phase dependency**: `Cod_cng()` internally calls `Calc_exc_rand(cur_gain, exc, &seed, FLAG_COD)` (`dtx.c:216`) to generate CNG excitation on the encoder side, then calls `Int_qlpc(lsp_old_q, lspSid_q, Aq)` (`dtx.c:218`) to interpolate SID LSP for per-subframe LP coefficients, followed by copying `lspSid_q` to `lsp_old_q` (`dtx.c:219-221`). `Calc_exc_rand` is defined in Phase 9 (`calcexc.c`). The `FLAG_COD=1` parameter controls the `update_exc_err()` call for taming state, which differs from the decoder path (`FLAG_DEC=0`). During Batch 5 skeleton generation, a `Calc_exc_rand` stub must exist before `Cod_cng` implementation begins
   - **Skipped entirely**: LSP speech quantization (`Qua_lsp`), open-loop pitch search (`Pitch_ol_fast`), target/impulse response computation, closed-loop pitch search (`Pitch_fr3_fast`), ACELP codebook search, gain quantization (`Qua_gain`)
   - **Still updated**: `wsp[]`, `mem_w` and `mem_w0` are updated via a residual+filtering loop (`COD_LD8A.C:270-291`) to keep filter memories consistent for when speech resumes
   - **Reset**: `sharp = SHARPMIN` after the inactive frame processing
   - **Buffer shifts**: `old_speech`, `old_wsp`, `old_exc` are shifted left by L_FRAME as normal
   - The function returns early after `Cod_cng()` — the active-frame subframe loop at `COD_LD8A.C:400-528` is never entered

**Static/helper functions included in tasks above** (for C-to-Rust mapping completeness):
- `Calc_pastfilt()`, `Calc_RCoeff()`, `Cmp_filt()` in `DTX.C`: stationarity analysis helpers called by `Cod_cng()`/`Update_cng()` (tasks 3-4). Map to private fns in `annex_b/dtx/stationarity.rs`
- `Calc_sum_acf()`, `Update_sumAcf()` in `DTX.C`: autocorrelation accumulation helpers (task 3). Map to private fns in `annex_b/dtx/encode.rs`
- `Qnt_e()`, `New_ML_search_1()`, `New_ML_search_2()` in `qsidlsf.c`: ML search helpers within `lsfq_noise()` (task 5). Map to private fns in `annex_b/cng/sid.rs`
- `Quant_Energy()` in `qsidgain.c`: energy quantization helper within `Qua_Sidgain()` (task 6). Map to private fn in `annex_b/cng/sid.rs`

**Test Plan**:

| Test | Input | Expected | Validates |
|------|-------|----------|-----------|
| Frame type transitions | Speech -> silence -> speech | 1 -> 2 -> 0 -> 0 -> 2 -> ... -> 1 | State machine |
| SID frame content | First SID after speech | Correct LSF + gain indices | SID encoding |
| Stationarity detection | Stationary noise | No SID updates (no-tx) | Itakura distance |
| Non-stationary noise | Changing noise | Periodic SID updates | Change detection |
| tstseq1.bin -> tstseq1a.bit | Annex B encoder test | Bit-exact bitstream | Full encoder conformance |
| tstseq2.bin -> tstseq2a.bit | Annex B encoder test | Bit-exact bitstream | Sequence 2 |
| tstseq3.bin -> tstseq3a.bit | Annex B encoder test | Bit-exact bitstream | Sequence 3 |
| tstseq4.bin -> tstseq4a.bit | Annex B encoder test | Bit-exact bitstream | Sequence 4 |

**Exit Criteria**:
1. All 4 Annex B encoder test vectors pass bit-exactly
2. DTX state machine transitions verified against reference
3. SID frame content (LSF + gain indices) matches reference exactly

**TDD Workflow**:
1. **Write tests first**: Create `tests/integration/annex_b_conformance.rs` with 4 encoder test functions (tstseq1-4). Each encodes a `.bin` PCM input with VAD enabled and compares the bitstream against the reference `a.bit` file. Also write unit tests for DTX state machine transitions and SID frame content.
   ```
   cargo test --test annex_b_conformance --features annex_b,itu_serial  # expect: 0 passed
   ```
2. **Implement**: DtxState -> state machine -> autocorrelation averaging -> Itakura distance -> SID LSF/gain quantization -> Cod_cng -> encoder integration (VAD/DTX bypass of subframe loop).
3. **Verify**:
   ```
   cargo test --test annex_b_conformance --features annex_b,itu_serial  # 4 encoder tests pass
   python tests/scripts/run_all_tiers.py --phase 8                      # Annex B encoder vectors pass
   ```
4. **Gate**: All 4 Annex B encoder vectors pass bit-exactly. All Annex A vectors still pass (no regressions).
