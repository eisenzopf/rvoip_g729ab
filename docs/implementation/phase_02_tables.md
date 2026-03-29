> Part of [G.729AB Implementation Plan](README.md)

### Phase 2: Codec Constants and Tables

**Goal**: Transcribe all ROM data from `TAB_LD8A.C` and `tab_dtx.c` into `const` Rust arrays.

**Files to create**:
- `src/tables/window.rs` — `hamwindow[240]`, `lag_h[M+2=12]`, `lag_l[M+2=12]` (DPF high/low parts; reference code uses M+2=12 entries. PRD §9.1 says "11" which counts only the M+1=11 lag coefficients for speech LP analysis. Entry 11 (index [11]) is NOT a guard element — it is the active lag window coefficient for lag 12, required by the VAD's NP=12 order autocorrelation analysis when Annex B is enabled. See erratum E6 in SPECIFICATION_PLAN.md)
- `src/tables/lsp.rs` — `lspcb1[128][10]`, `lspcb2[32][10]`, `fg[2][4][10]`, `fg_sum[2][10]`, `fg_sum_inv[2][10]`, `freq_prev_reset[10]`
- `src/tables/gain.rs` — `gbk1[8][2]`, `gbk2[16][2]`, `coef[2][2]`, `L_coef[2][2]`, `thr1[4]`, `thr2[10]`, `pred[4]`, `past_qua_en_reset[4]`
- `src/tables/pitch.rs` — `inter_3l[FIR_SIZE_SYN]` where `FIR_SIZE_SYN = UP_SAMP*L_INTER10+1 = 31` (sinc interpolation filter for 1/3-sample pitch interpolation)
- `src/tables/misc.rs` — `grid[GRID_POINTS+1=51]` (Chebyshev grid, Q15: grid[0]=32760, grid[50]=-32760), `table[65]` (cosine), `slope[64]`, `table2[64]`, `slope_cos[64]`, `slope_acos[64]`, `tabpow[33]`, `tablog[33]`, `tabsqr[49]`, `tab_zone[PIT_MAX+L_INTERPOL-1=153]` (pitch zone mapping for taming)
- `src/tables/bitstream.rs` — `bitsno[11]`, `bitsno2[4]`, frame size constants, RATE_8000, RATE_SID, RATE_SID_OCTET, RATE_0
- `src/tables/postfilter.rs` — Post-filter gamma power tables
- `src/tables/vad.rs` — `lbf_corr[13]`, `shift_fx[33]`, `factor_fx[33]` [cfg(annex_b)]
- `src/tables/sid.rs` — `tab_Sidgain[32]`, `noise_fg[2][4][10]`, `noise_fg_sum[MODE][M]` (pre-computed row sums of `noise_fg`, used by `sid_lsfq_decode` and `lsfq_noise`), `noise_fg_sum_inv[MODE][M]` (pre-computed inverse row sums, used by `lsfq_noise`), `PtrTab_1[32]`, `PtrTab_2[2][16]`, `Mp[2]` [cfg(annex_b)]. Reference: `tab_dtx.c`
- `src/tables/mod.rs` — Re-exports

**Reference**: `reference/itu_reference_code/g729ab_v14/tab_ld8a.c` (~30 KB), `tab_dtx.c`

**Total**: ~40 const arrays, ~1,800 lines (mostly data)

**Tasks**:
1. Transcribe core tables from `tab_ld8a.c` — use automated script to convert C arrays to Rust const
2. Transcribe Annex B tables from `tab_dtx.c`
3. Define all codec constants in appropriate modules (L_FRAME=80, L_SUBFR=40, M=10, PIT_MIN=20, PIT_MAX=143, etc.) including:
   - `SERIAL_SIZE = 82` (ITU serial format buffer: bfi + 80 speech bits + frame type)
   - `DIM_RR = 616` (correlation matrix size for ACELP search)
   - `MSIZE = 64` (cross-correlation vector size)
   - `NC = 5` (M/2, used in LSP polynomial evaluation)
   - `L_LIMIT = 40`, `M_LIMIT = 25681` (Q13, LSP boundary constraints)
   - `GAP1 = 10` (Q13, stability gap for `Lsp_expand_1` — minimum LSP spacing pass 1)
   - `GAP2 = 5` (Q13, stability gap for `Lsp_expand_2` — minimum LSP spacing pass 2)
   - `GAP3 = 321` (Q13, stability gap for boundary constraints)
   - `GP0999 = 16383` (Q14, maximum pitch gain ~0.999, used in gain quantization)
   - `GPCLIP = 15564` (Q14, maximum pitch gain when taming is active)
   - `GPCLIP2 = 481` (Q9, used by gain pre-selection during taming: `best_gain[0]` is clipped to GPCLIP2 in `QUA_GAIN.C:138-140` when `tameflag == 1`)
   - `CONST12 = 19661` (Q14, 1.2, pitch gain upper bound for `G_pitch()` in `PITCH_A.C`)
   - `L_THRESH_ERR = 983040000` (taming error threshold, 16384*60000)
   - `INV_COEF = -17103` (Q19, gain prediction mean energy constant)
   - `L_TOTAL = 240` (total speech buffer size)
   - `L_NEXT = 40` (look-ahead samples for encoder analysis window; equals `L_SUBFR` for G.729A; used in encoder buffer pointer offsets: `SPEECH_OFFSET = L_TOTAL - L_FRAME - L_NEXT`)
   - Post-filter constants: `L_H = 22` (truncated impulse response length), `GAMMA2_PST = 18022` (0.55 Q15, numerator), `GAMMA1_PST = 22938` (0.70 Q15, denominator), `MU = 26214` (0.8 Q15, tilt compensation), `AGC_FAC = 29491` (0.9 Q15), `AGC_FAC1 = 3276` (1-AGC_FAC Q15), `GAMMAP = 16384` (0.5 Q15), `INV_GAMMAP = 21845` (1/(1+GAMMAP) Q15), `GAMMAP_2 = 10923` (GAMMAP/(1+GAMMAP) Q15)
   - Pre-processing filter: `b140[3] = {1899, -3798, 1899}` Q12, `a140[3] = {4096, 7807, -3733}` Q12 (from `pre_proc.c`, function-local — NOT in `tab_ld8a.c`)
   - Post-processing filter: `b100[3] = {7699, -15398, 7699}` Q13, `a100[3] = {8192, 15836, -7667}` Q13 (from `post_pro.c`, function-local — NOT in `tab_ld8a.c`)
4. Define Annex B constants from `dtx.h` and `octet.h` in feature-gated modules (`tables/sid.rs`, `tables/vad.rs`):
   - DTX sizing: `NB_CURACF = 2`, `NB_SUMACF = 3`, `NB_GAIN = 2`, `SIZ_ACF = NB_CURACF * MP1 = 22`, `SIZ_SUMACF = NB_SUMACF * MP1 = 33`
   - DTX timing: `FR_SID_MIN = 3` (minimum frames between SID emissions)
   - Itakura distance thresholds: `FRAC_THRESH1 = 4855` (Q15), `FRAC_THRESH2 = 3161` (Q15)
   - CNG gain smoothing: `A_GAIN0 = 28672` (Q15, ~0.875), `A_GAIN1 = 4096` (Q15, 32768 − A_GAIN0)
   - CNG excitation generation: `K0 = 24576` (Q15, 1 − α², α=0.5), `FRAC1 = 19043` (Q15, Gaussian normalization: (√40·α/2 − 1)·32768), `G_MAX = 5000` (Q0, maximum fixed codebook gain for CNG)
   - Flags and seed: `FLAG_COD = 1`, `FLAG_DEC = 0` (encoder/decoder flag for `Calc_exc_rand`), `INIT_SEED = 11111` (CNG random generator initial seed)
   - Rate constants (shared with `tables/bitstream.rs`): `RATE_8000 = 80`, `RATE_SID = 15`, `RATE_SID_OCTET = 16` (from `octet.h`), `RATE_0 = 0`
5. Verify multi-dimensional array layouts match C memory order
6. Include all constants from PRD §12 and Appendix: Verified Constants

**Test Plan**:

| Test | Method | Validates |
|------|--------|-----------|
| Table checksums | CRC32 of each Rust array vs C array byte content | Transcription accuracy |
| `hamwindow` spot check | Verify `hamwindow[0]`, `hamwindow[119]`, `hamwindow[239]` | Window endpoints |
| `lspcb1` dimensions | Assert `lspcb1.len() == 128`, each entry `.len() == 10` | Codebook shape |
| `grid` endpoints | `grid[0] == 32760`, `grid[50] == -32760`, `grid.len() == 51` | Grid range (GRID_POINTS+1=51 entries, Q15) |
| `inter_3l` symmetry | Verify filter symmetry properties | Sinc filter |
| `bitsno` sum | `bitsno.iter().sum() == 80` | Bit allocation totals 80 |
| `bitsno2` sum | `bitsno2.iter().sum() == 15` | SID bit allocation |
| Constants match LD8A.H | Assert L_FRAME==80, L_SUBFR==40, M==10, PIT_MIN==20, PIT_MAX==143 | Header constants |

**Automated validation**: Write a test-time script that parses `tab_ld8a.c` and compares every numeric literal against the Rust const arrays. This catches any transcription error.

**Exit Criteria**:
1. All table checksums match between C source and Rust `const` arrays
2. Automated parser confirms every numeric literal from `tab_ld8a.c` and `tab_dtx.c` is present
3. All constants from PRD §12 / Appendix verified via assertions

**TDD Workflow**:
1. **Write tests first**: Create `#[cfg(test)]` modules in each table file with spot-check assertions (endpoints, dimensions, sums) and a CRC32 checksum test against the C source. Write the automated table parser test that reads `tab_ld8a.c` and verifies every literal.
   ```
   cargo test --lib tables::                        # expect: compile errors (tables not yet defined)
   ```
2. **Transcribe tables**: Use an automated script to convert C arrays to Rust const. Fill in one file at a time — tests become green as each table is transcribed.
3. **Verify**:
   ```
   cargo test --lib tables::                        # all checksum + spot-check tests pass
   python tests/scripts/run_all_tiers.py --phase 2  # exits 0 (Gate 2)
   ```
4. **Gate 2**: `run_all_tiers.py --phase 2` reports `PASS` for Gate 2.
