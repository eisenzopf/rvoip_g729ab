> Part of [Specification Plan](README.md)

### Batch 5: Phase 7 (VAD) + Phase 8 (DTX) + Phase 9 (CNG)

#### SPEC_PHASE_07_vad.md

**Sources:**

| Source | What it provides |
|--------|-----------------|
| [Implementation Plan Phase 7](../implementation/phase_07_vad.md) | Function list (~4 functions), TDD workflow |
| `reference/.../g729ab_v14/vad.c` | VAD implementation |
| `PRD.md` §8.1 | VAD requirements |

**Key decisions:**

- VadState struct — complete field list from `vad.c` static variables (initialized by `vad_init()` and C static zero-init):
  - `MeanLSF[M]` = 0 (running mean of LSF coefficients; `vad.c`)
  - `MeanSE` = 0 (running mean of full-band energy; `vad.c`)
  - `MeanSLE` = 0 (running mean of low-band energy; `vad.c`)
  - `MeanE` = 0 (running mean energy, used during initialization; `vad.c`)
  - `MeanSZC` = 0 (running mean of zero-crossing rate; `vad.c`)
  - `prev_energy` = 0 (previous frame energy for hangover detection; `vad.c`)
  - `count_sil` = 0 (silence frame counter for forced noise; `vad.c`)
  - `count_update` = 0 (background noise update frame counter; `vad.c`)
  - `count_ext` = 0 (extension counter for smoothing stage 3; `vad.c`)
  - `less_count` = 0 (frames below energy threshold during init; `vad.c`)
  - `flag` = 1 (initialization-in-progress flag; cleared to 0 at frame 32 when init completes; `vad.c:57`)
  - `v_flag` = 0 (smoothing override flag; `vad.c:34`). Set to 1 within the current frame when the smoothing pipeline overrides a MakeDec NOISE decision back to VOICE — specifically in the "energy hangover" stage (`vad.c:233-237`: `prev_marker==VOICE && marker==NOISE && dSE < -410 && ENERGY > 3072`) and the "extension" stage (`vad.c:244-246`: pprev/prev both VOICE, marker NOISE, energy stable). Checked by the forced-NOISE override (`vad.c:271`: `!v_flag`) to prevent re-overriding frames that were already forced to VOICE by smoothing. Reset to 0 at the start of each frame's smoothing pipeline (`vad.c:232`)
  - `Min_buffer[16]` = 0 (8-frame sliding minimum energy buffer; `vad.c`)
  - `Prev_Min` = 0, `Next_Min` = 0, `Min` = MAX_16 (32767) (minimum energy tracking; `vad.c:58`. Prev_Min, Next_Min, and Min_buffer rely on C static zero-init; Min is explicitly set in vad_init())
  - **C init boundary note:** In the C reference, `vad_init()` explicitly initializes `MeanLSF` (via `Set_zero`), `MeanSE`, `MeanSLE`, `MeanE`, `MeanSZC`, `count_sil`, `count_update`, `count_ext`, `less_count`, `flag` (=1), and `Min` (=MAX_16). The remaining fields (`Prev_Min`, `Next_Min`, `Min_buffer`, `prev_energy`, `v_flag`) rely on C static zero-initialization and are NOT touched by `vad_init()`. In Rust, all fields must be explicitly initialized in `VadState::new()`.
- **Encoder frame counter for VAD**: The `vad()` function receives `frm_count` (frame counter) as a parameter, used to distinguish the initialization period (frames 0-32, i.e., 33 frames where `sub(frm_count, INIT_FRAME) <= 0` with INIT_FRAME=32) from steady-state operation. In the C reference, this counter is maintained by the calling program (`coder.c`) and passed into `Coder_ld8a(ana[], frame, vad_enable)` as the `frame` parameter, which is then forwarded to `vad()` at `COD_LD8A.C:257`. The counter is incremented AFTER the `Coder_ld8a()` call returns: `frame = add(frame, 1)` (in `coder.c`). In the Rust implementation, this should be a field on `EncoderState` (`frame: Word16 = 0`) — the current value is passed to the VAD first, then incremented at the **end** of `encode()`, so the VAD sees frm_count values 0, 1, 2, ..., matching the C reference. **Critical:** incrementing BEFORE the VAD call would shift the initialization period by one frame and fail Annex B conformance. It is NOT part of `VadState` because it is maintained by the encoder main loop, not by `vad_init()`.
- Feature extraction: Ef (full-band), El (low-band via `lbf_corr`), SD (LSF distance), ZC (zero-crossing). When Annex B is active, the encoder's main `Autocorr()` is called with order NP=12 (not M=10), producing 13 autocorrelation lags shared between LP analysis (first 11 values) and VAD energy features (all 13 values). The VAD does NOT compute its own autocorrelation. Reference: `COD_LD8A.C:231,242`. When `annex_b` feature is disabled, `Autocorr` uses order M=10
- MakeDec: 14 linear discriminant conditions (any match = VOICE). **Cross-verification requirement:** `SPEC_PHASE_07_vad.md` must include a table mapping each Annex B spec constant (`a1`-`a14`, `b1`-`b14` from Table B.1, e.g., `a1=23488`, `b1=28521`) to the corresponding C code `L_mult` operands in `MakeDec()` (e.g., `-14680` for `dSZC`, `-28521` for threshold), with Q-format scaling factors documented. Each row of the mapping table must include: (a) Annex B spec constant name and floating-point value from Table B.1, (b) C code function name and line reference, (c) C code fixed-point operand values, (d) Q-format relationship between spec and code domains, (e) a verification formula showing mathematical equivalence (e.g., `spec_a1 = 23488 → code_operand = 23488 / 1.6 = 14680 (Q15 scaling factor)`). The mapping involves sign conventions and Q-format shifts that are non-trivial; without explicit verification formulae, transcription errors are likely. **PRD errata E20/E21:** PRD §8.1.2 conditions 8-10 previously used the wrong variable name (`dSLE` instead of `dSE`) — the C code comments at `vad.c:415` say "dSLE vs dSZC" but the actual code uses `dSE`. Conditions 1-4, 8-9 also had an incorrect operation representation (`L_shl` instead of the correct `L_shr` + `L_deposit_h` pattern). Both are now corrected in the PRD. The cross-verification table must verify which C variable (`dSE` vs `dSLE`) each condition actually uses, since the C comments are misleading
- Initialization (frames 0-32, i.e., 33 frames total where `sub(frm_count, INIT_FRAME) <= 0` with INIT_FRAME=32; `vad.h:12`, `vad.c:183`): hard threshold `Ef < 3072` classifies as NOISE, `Ef >= 3072` classifies as VOICE; statistics accumulated only on VOICE frames (frames above threshold contribute to running mean estimates; frames below threshold increment `less_count` for mean adjustment at frame 32, the final initialization frame). Reference: `vad.c:183-201`. NOTE: PRD §8.1.3 previously said "frames 0-31" (32 frames) — this was off by one; see erratum E17
- 4-stage smoothing: inertia (6 frames), energy hangover (2dB), extension (4 frames), forced noise
- **bcg729 VAD smoothing stage 4 omission:** bcg729's `vad.c` (line 340) explicitly notes that the fourth smoothing stage (forced-NOISE override) is not implemented. This means bcg729 VAD decisions will differ from the ITU reference for frames where stage 4 would trigger. If bcg729 VAD output is used as a secondary reference during debugging, this omission could cause false-alarm discrepancies
- Background noise update with rate-adaptive coefficients from PRD §8.1.4 tables

**`vad()` C function signature** (`vad.c`): The spec document must map all 10 parameters to the Rust equivalent:

| C Parameter | Type | Description | Source in Encoder |
|-------------|------|-------------|-------------------|
| `rc` | `Word16` | Second reflection coefficient (k_2, index 1 from Levinson `rc[2]` output) for background noise update condition `rc < 24576` | `LPC.C` via `COD_LD8A.C:251-252` (note: Annex B spec B.3.2 says "first reflection coefficient r_1" but code passes `rc[1]`; see SE7) |
| `lsf` | `Word16 *` (M values) | LSF vector for spectral distortion feature (SD) | `Lsp_lsf(lsp_new, lsf_new, M)` at `COD_LD8A.C:250` |
| `r_h` | `Word16 *` (NP+1 values) | Autocorrelation DPF high parts for energy features (Ef, El) | Shared from `Autocorr()` |
| `r_l` | `Word16 *` (NP+1 values) | Autocorrelation DPF low parts for energy features | Shared from `Autocorr()` |
| `exp_R0` | `Word16` | Exponent of r[0] for energy normalization | From `Autocorr()` |
| `sigpp` | `Word16 *` (L_FRAME values) | Preprocessed speech for zero-crossing rate (ZC) | `new_speech` at `COD_LD8A.C:257` |
| `frm_count` | `Word16` | Frame counter for initialization period (see encoder `frame` state field) | `EncoderState::frame` |
| `prev_marker` | `Word16` | Previous VAD decision for smoothing stages | `EncoderState::pastVad` |
| `pprev_marker` | `Word16` | Two-frames-ago VAD decision for smoothing stage 2 | `EncoderState::ppastVad` |
| `marker` | `Word16 *` | Output: VAD decision (VOICE=1 / NOISE=0) | Returned to encoder main loop |

**Rust transformation for `marker`:** In the C signature, `marker` is a `Word16 *` output parameter. In Rust, this becomes the function return value (`-> Word16` or a `VadDecision` enum). The `prev_marker` and `pprev_marker` input parameters remain as `Word16` arguments. The caller (`encode()`) stores the returned value into `EncoderState::pastVad` and shifts the previous value to `ppastVad`.

**Module file mapping:** `annex_b/vad/state.rs`, `annex_b/vad/features.rs`, `annex_b/vad/decision.rs`, `annex_b/vad/detect.rs`

**TDD requirements:**

- Unit tests for feature extraction (silence -> low energy, speech -> high energy)
- MakeDec discriminant boundary tests
- Initialization period: first 33 frames (frames 0-32) use energy threshold (Ef >= 3072 -> VOICE, Ef < 3072 -> NOISE)
- Smoothing stage tests
- VAD alone cannot be end-to-end tested against ITU vectors (requires DTX in Phase 8)

**Mandatory spec deliverables checklist (Batch 5):**

- [ ] MakeDec cross-verification table: map each Annex B Table B.1 constant (`a1`-`a14`, `b1`-`b14`) to C code `L_mult`/`L_mac`/`L_shr`/`L_deposit_h` operands with Q-format scaling factors and mathematical equivalence formulae. This table is required before implementing `MakeDec()` to prevent transcription errors in the 14 discriminant conditions (see errata E20, E21, SE3)

**PRD errata to document (see Section 12):**

- PRD §8.1.1 (E12): VAD uses 13 autocorrelation coefficients (lags 0-12, NP=12), not "11 autocorrelation coefficients (lags 0-10)" as the PRD states. The speech LP analysis uses M=10 (11 lags), but the VAD extends to NP=12 (13 lags). Reference: `vad.h` NP=12, `vad.c` autocorrelation loop

#### SPEC_PHASE_08_dtx.md

**Sources:**

| Source | What it provides |
|--------|-----------------|
| [Implementation Plan Phase 8](../implementation/phase_08_dtx.md) | Function list (~15 functions), TDD workflow |
| `reference/.../g729ab_v14/dtx.c` | DTX state machine |
| `reference/.../g729ab_v14/qsidlsf.c`, `qsidgain.c` | SID quantization |
| `reference/.../g729ab_v14/cod_ld8a.c` lines 260-302 | Encoder conditional processing during DTX |
| `PRD.md` §8.2 | DTX requirements |

**Key decisions:**

- DTX state machine: VOICE -> emit SID -> count_fr0 < FR_SID_MIN(3) -> no-tx -> stationarity check -> SID or no-tx. NOTE: Annex B spec has an internal inconsistency -- text (B.4.1.2) says "greater than Nmin=2" (count_fr >= 3), but equation B.11 appears to show "count_fr >= Nmin=2" (count_fr >= 2). Code uses FR_SID_MIN=3 with `count_fr0 < 3` check (`dtx.h:78`, `dtx.c:150`), matching the textual interpretation. Code is authoritative
- Itakura distance thresholds: FRAC_THRESH1=4855, FRAC_THRESH2=3161
- `noise_fg` table derivation: `noise_fg[0][i][j] = fg[0][i][j]` (identity copy) and `noise_fg[1][i][j] = (19660*fg[0][i][j] + 13107*fg[1][i][j]) >> 15` (weighted combination). Compute both as `const` arrays in `tables/sid.rs` (pre-computed at compile time from `fg` table values). The C reference uses runtime `Init_lsfq_noise()`, but the values are deterministic from the `fg` table, so a Rust `const` avoids runtime initialization and mutable state. NOTE: In C, `noise_fg` is **declared** in `tab_dtx.c` (as a global mutable array) but **initialized** by `Init_lsfq_noise()` defined in `dec_sid.c`. In Rust, both declaration and initialization collapse into a single `const` in `tables/sid.rs`. **Computation equivalence note:** The C code computes `noise_fg[1]` via `L_mult(fg[0][i][j], 19660)` + `L_mac(fg[1][i][j], 13107)` + `extract_h()`. The `L_mult`/`L_mac`/`extract_h` sequence is equivalent to `(a * 19660 + b * 13107) >> 15` (truncation, no rounding — uses `extract_h`, not `round`). Derivation: `L_mult(a, 19660)` = `a*19660*2`; `L_mac(acc, b, 13107)` = `acc + b*13107*2`; `extract_h(acc)` = `acc >> 16` (truncation); combined: `(a*19660 + b*13107)*2 >> 16` = `(a*19660 + b*13107) >> 15`. All input values from `fg[]` are in Q15 range and the intermediate products never overflow i32, so the `const` pre-computation `(a * 19660 + b * 13107) >> 15` is safe. **Note:** `noise_fg[0][i][j] = fg[0][i][j]` (identity copy, Step 1 of `Init_lsfq_noise`); only `noise_fg[1]` uses the weighted formula
- **`tables/sid.rs` auditability requirement:** Since the Rust `const` replaces C's runtime `Init_lsfq_noise()`, the `tables/sid.rs` file must include a derivation comment: `// NOISE_FG[1][i][j] = (fg[0][i][j] * 19660 + fg[1][i][j] * 13107) >> 15 (truncation via extract_h, NOT rounding)`. This allows future maintainers to regenerate or verify the table without consulting the C source. See also Phase 2 content outline for the same requirement stated from the table-inventory perspective
- **Rust `const`-evaluability confirmed:** The `noise_fg`, `noise_fg_sum`, and `noise_fg_sum_inv` computations require only basic `i32` multiply, add, and shift operations — all `const`-evaluable in Rust stable since 1.46 (basic integer arithmetic in `const fn`). No build-time codegen step is needed. Example pattern: `const fn compute_noise_fg_entry(a: i32, b: i32) -> i16 { ((a * 19660 + b * 13107) >> 15) as i16 }` (truncation — do NOT add `+ 16384` rounding; the C code uses `extract_h`, not `round`). The TDD verification test (Phase 8) confirms this equivalence
- `noise_fg_sum[MODE][M]` and `noise_fg_sum_inv[MODE][M]` tables (`tab_dtx.c`): pre-computed row sums and inverse row sums of `noise_fg`, consumed by `lsfq_noise()` (`qsidlsf.c:101,115,241,250`) for SID LSF encoding, and by `sid_lsfq_decode()` (`dec_sid.c:179-180`) for SID LSF decoding. Must be included in `tables/sid.rs` alongside `noise_fg`. **Provenance and transcription strategy:** The derivation chain is `fg` → `noise_fg` → `noise_fg_sum` → `noise_fg_sum_inv`. While all tables could theoretically be derived at compile time from `fg`, transcribe `noise_fg_sum_inv` directly from `tab_dtx.c` as the primary source to match the C reference exactly. Add a TDD verification test that independently derives `noise_fg_sum_inv` from the `noise_fg` const values and asserts equality, catching any transcription error in either table
- **Encoder-side SID LSF gap enforcement (`lsfq_noise()`):** Two separate gap enforcement stages exist in `qsidlsf.c`:
  1. **Pre-VQ boundary enforcement** (`qsidlsf.c:81-89`): Before the VQ codebook search, the raw LSF vector undergoes boundary clamping: `lsf[0] >= L_LIMIT`, successive gaps `lsf[i+1] - lsf[i] >= 2*GAP3`, `lsf[M-1] <= M_LIMIT`, and a final back-off `lsf[M-2] = lsf[M-1] - GAP3` if ordering is violated. This operates on the input LSFs before `Get_wegt()` and `Lsp_prev_extract()`.
  2. **Post-VQ gap enforcement** (`qsidlsf.c:111`): After the VQ codebook search (`Qnt_e`), calls `Lsp_expand_1_2(tmpbuf, 10)` on the quantized prediction error vector, then `Lsp_stability(lsfq)` at line 121 on the composed output. This is asymmetric with the decoder's `sid_lsfq_decode()` which uses an inline loop for the same purpose (see Phase 9 key decisions). The Rust encoder must call `lsp_expand_1_2` here, not replicate the decoder's inline loop
- **Cod_cng() internal Levinson recursion** (`dtx.c:117-123`): After computing the averaged autocorrelation via `Calc_sum_acf()`, `Cod_cng()` calls `Levinson(curAcf, zero, curCoeff, bid, &ener[0])` to derive LP coefficients from the accumulated noise autocorrelation. This is separate from the main encoder LP analysis (which operates on per-frame windowed speech). The SID Levinson produces `curCoeff` for stationarity analysis (`Cmp_filt`, `Calc_pastfilt`) and `ener[0]` for SID gain quantization (`Qua_Sidgain`). The Rust implementation must reuse the same `levinson()` function from Phase 3's `lp/levinson.rs`
- **`Get_freq_prev`/`Update_freq_prev` around `Cod_cng`** (`cod_ld8a.c:262-264`): The encoder's DTX branch brackets the `Cod_cng()` call with `Get_freq_prev(lsp_old_q)` (line 262) and `Update_freq_prev(lsp_old_q)` (line 264). `Get_freq_prev` saves the LSP MA prediction memory (`freq_prev[MA_NP][M]`) into `lsp_old_q` before `Cod_cng()` potentially modifies it via `lsfq_noise → Lsp_prev_update`, and `Update_freq_prev` writes the updated values back after `Cod_cng()` returns. The Rust implementation must replicate this save/restore pattern on the shared `freq_prev` state to prevent corruption of the LSP prediction memory during DTX, which would cause incorrect LSP quantization when speech resumes after silence
- Encoder conditional processing during VAD=NOISE (PRD §8.1.5): LP analysis still runs, subframe loop is replaced by `Cod_cng()`, filter memories updated via residual+filtering loop, `sharp = SHARPMIN` after inactive frame. `Cod_cng()` internally calls `Calc_exc_rand(cur_gain, exc, seed, FLAG_COD)` (`dtx.c:216`) — the `FLAG_COD=1` parameter causes `update_exc_err(Gp, t0)` to be called inside `Calc_exc_rand` for each CNG subframe, keeping the encoder's taming state (`L_exc_err[4]`) current during DTX. This is a cross-phase dependency: Phase 8's `Cod_cng` depends on Phase 9's `Calc_exc_rand` in `annex_b/cng/excitation.rs`
- **Encoder-side filter memory update during DTX** (`COD_LD8A.C:270-291`): After `Cod_cng()` returns, the encoder runs a per-subframe loop to keep `wsp`, `mem_w`, and `mem_w0` consistent for when speech resumes. **Critical:** the LP coefficient pointer must be initialized from `Int_qlpc` output and advanced per subframe. Pseudocode:
  ```
  Aq = Aq_t;                  // Initialize from Int_qlpc output (cod_ld8a.c:270)
  for i_subfr in (0, L_SUBFR):
  ```
  1. `Residu(Aq, &speech[i_subfr], xn, L_SUBFR)` — compute LP residual into `xn`
  2. Build tilt-compensated filter: `Weight_Az(Aq, GAMMA1, M, Ap_t)`, then `Ap[0] = 4096; Ap[i] = Ap_t[i] - mult(Ap_t[i-1], 22938)` for i=1..M (22938 = 0.7 Q15)
  3. `Syn_filt(Ap, xn, &wsp[i_subfr], L_SUBFR, mem_w, 1)` — filter with state update to produce weighted speech and update `mem_w`
  4. Compute `mem_w0` update: `xn[i] = xn[i] - exc[i_subfr+i]` (residual minus excitation), then `Syn_filt(Ap_t, xn, xn, L_SUBFR, mem_w0, 1)` with state update
  ```
  Aq += MP1;                  // Advance to next subframe's coefficients (cod_ld8a.c:298)
  ```
  Without the `Aq += MP1` advancement, the second subframe uses subframe 1's LP coefficients, producing incorrect `mem_w` and `mem_w0` state that corrupts weighted speech when speech resumes after DTX
- **`Cod_cng` `lspSid_q` data flow:** `lspSid_q` is zero-initialized (C static default), then first written by `lsfq_noise(lsp_new, lspSid_q, freq_prev, &ana[1])` at `dtx.c:198` (quantizes noise LSFs into `lspSid_q`). Subsequently used by `Int_qlpc(lsp_old_q, lspSid_q, Aq)` at `dtx.c:218` (interpolates SID LSPs for per-subframe LP coefficients), then copied to `lsp_old_q` at `dtx.c:220` (updates persistent quantized LSP memory). Complete data flow: `0 (init) → lsfq_noise writes → Int_qlpc reads → Copy to lsp_old_q`. The `lsfq_noise` call only executes on SID emission frames (when the DTX state machine decides to emit a SID); on no-tx frames, `lspSid_q` retains its previous value
- **`Cod_cng()` pastVad parameter:** The call signature is `Cod_cng(exc, pastVad, lsp_old_q, Aq_t, ana, lsfq_mem, &seed)` (`cod_ld8a.c:263`). The `pastVad` parameter (previous frame's VAD decision) controls whether `flag_chang` triggers immediate SID emission: when `pastVad == 1` (previous frame was speech), the current frame is the first silence frame after speech, so `Cod_cng()` forces SID emission regardless of the frame count (`dtx.c:152-155`). When `pastVad == 0` (continuing silence), the normal `count_fr0 < FR_SID_MIN` / stationarity-check logic governs SID emission. The `pastVad` and `ppastVad` values are updated at `cod_ld8a.c:265-266,313-314` at the end of frame processing (after `Cod_cng` or active-frame encoding), ensuring the next frame sees the correct previous VAD state
- **`Cod_cng()` output to `ana[]` array:** `Cod_cng()` writes the parameter array `ana[]` that is subsequently packed into the bitstream (Phase 4 integration). The output format depends on the DTX state machine decision:
  - **SID frame (RATE_SID):** `ana[0]` = `L0` (1 bit, MA predictor mode), `ana[1..5]` = SID LSF VQ indices (per `bitsno2[4] = {1,5,4,5}` — 15 bits total), `ana[6]` = SID gain index (5 bits). Total: 21 bits. Written by `lsfq_noise()` (indices) and `Qua_Sidgain()` (gain index) inside `Cod_cng()`
  - **No-transmission frame (RATE_0):** `Cod_cng()` sets the rate indicator to `RATE_0`, signaling `prm2bits_ld8k()` to emit an empty frame (2 sync bits only in serial format). No `ana[]` elements are written
  This output format must be consistent with the bitstream packing tables in Phase 4. Reference: `dtx.c` `Cod_cng()` function body, `tab_dtx.h` for `bitsno2` table
- DtxState initialization (`Init_Cod_cng` in `DTX.C`): Complete field list from `dtx.c` static variables. **Sizing constants** (from `dtx.h`): `SIZ_ACF = NB_CURACF * (M+1) = 2 * 11 = 22`, `SIZ_SUMACF = NB_SUMACF * (M+1) = 3 * 11 = 33`:
  - `Acf[SIZ_ACF]` = 0 (autocorrelation history buffer, 22 elements; `DTX.C:29`)
  - `sh_Acf[NB_CURACF]` = {40, 40} (`DTX.C:30`)
  - `sumAcf[SIZ_SUMACF]` = 0 (accumulated autocorrelation, 33 elements; `DTX.C:31`)
  - `sh_sumAcf[NB_SUMACF]` = {40, 40, 40} (`DTX.C:32`)
  - `ener[NB_GAIN]` = 0 (energy history; `DTX.C:34`)
  - `sh_ener[NB_GAIN]` = {40, 40} (`DTX.C:35`)
  - `lspSid_q[M]` = 0 (quantized LSP for SID frame; `DTX.C:25`)
  - `pastCoeff[MP1]` = 0 (past filter coefficients for stationarity; `DTX.C:26`)
  - `RCoeff[MP1]` = 0 (autocorrelation of filter coefficients; `DTX.C:27`)
  - `sh_RCoeff` = 0 (scaling for RCoeff; `DTX.C:28`)
  - `fr_cur` = 0, `cur_gain` = 0, `flag_chang` = 0 (`DTX.C:36,38,39`)
  - `nb_ener` = 0 (energy sample count, incremented up to NB_GAIN; `DTX.C:37`)
  - `sid_gain` = 0 (encoder-side SID gain; `DTX.C:38` — distinct from decoder `sid_gain` in `DEC_SID.C`)
  - `prev_energy` = 0 (previous SID energy for change detection; `DTX.C:40`)
  - `count_fr0` = 0 (frame count since last SID, controls FR_SID_MIN logic; `DTX.C:41`)
  - **C init boundary note:** In the C reference, `Init_Cod_cng()` (`DTX.C:57-75`) explicitly initializes **9 fields**: `sumAcf` (=0), `sh_sumAcf` (={40,40,40}), `Acf` (=0), `sh_Acf` (={40,40}), `sh_ener` (={40,40}), `ener` (=0), `cur_gain` (=0), `fr_cur` (=0), `flag_chang` (=0). The remaining **8 fields** — `lspSid_q`, `pastCoeff`, `RCoeff`, `sh_RCoeff`, `nb_ener`, `sid_gain`, `prev_energy`, `count_fr0` — are NOT touched by `Init_Cod_cng()` and rely on C's implicit zero-initialization for file-scope static variables. In Rust, all fields must be explicitly initialized in `DtxState::new()`.
- **Encoder-side CNG seed** (separate from DtxState, lives on `EncoderState`): `seed: Word16 = INIT_SEED (= 11111)` (`static Word16 seed` in `COD_LD8A.C`, initialized to `INIT_SEED` in `Init_Coder_ld8a()` at `COD_LD8A.C:158`). Passed to `Cod_cng()` and `Calc_exc_rand()` for encoder CNG excitation generation. Same `INIT_SEED` constant as decoder's CNG `seed` (11111 in `CngDecState`); distinct from decoder's `seed_fer` (21845 in `DecoderState`). Gated behind `#[cfg(feature = "annex_b")]` on `EncoderState`. **Per-active-frame reset:** On every active speech frame (VAD==1), the encoder resets `seed = INIT_SEED` (`cod_ld8a.c:312`, in the active-speech branch after `*ana++ = 1`). This mirrors the decoder's seed reset on ftyp==1 (`dec_ld8a.c:197`) and ensures deterministic CNG generation after each speech segment. Without this reset, encoder CNG excitation after speech starts from a stale seed value, failing Annex B conformance tests (tstseq1-4)

**Module file mapping:** `annex_b/dtx/state.rs`, `annex_b/dtx/encode.rs`, `annex_b/dtx/stationarity.rs`, `annex_b/cng/sid.rs` (shared -- created in Phase 8 with `lsfq_noise` and `Qua_Sidgain`; consumed by Phase 9's `sid_lsfq_decode` in `annex_b/cng/decode.rs`, which uses `noise_fg` tables from this module).

**`lsfq_noise()` cross-phase dependencies:** The encoder-side `lsfq_noise()` in `qsidlsf.c` calls functions from several earlier phases that must be available:
- `Get_wegt()` from Phase 6's `lsp_quant/helpers.rs` (`qsidlsf.c:90`)
- `Lsp_prev_extract()`, `Lsp_prev_compose()`, `Lsp_prev_update()` from Phase 5's `lsp_quant/prev.rs` (`qsidlsf.c:93,115-116,120`)
- `Lsp_expand_1_2()`, `Lsp_stability()` from Phase 5's `lsp_quant/stability.rs` (`qsidlsf.c:111,121`)
- `Lsf_lsp2()` from Phase 3's `lp/lsf.rs` (`qsidlsf.c:119`)

**Static/helper functions for C-to-Rust mapping** (private functions within the modules above):
- `Calc_pastfilt(DtxState)` in `DTX.C`: computes past filter coefficients for stationarity check, called by `Cod_cng()`. Map to private fn in `annex_b/dtx/stationarity.rs`
- `Calc_RCoeff(coeff, hi, lo)` in `DTX.C`: computes autocorrelation of filter coefficients for Itakura distance. Map to private fn in `annex_b/dtx/stationarity.rs`
- `Cmp_filt(RCoeff, sh_RCoeff, Acf, alpha, FracThresh)` in `DTX.C`: computes Itakura distance and compares against threshold, called by `Cod_cng()`. Map to private fn in `annex_b/dtx/stationarity.rs`
- `Calc_sum_acf(Acf, sh_Acf, sumAcf, sh_sumAcf, NbAcf)` in `DTX.C`: accumulates autocorrelation frames for SID filter estimation. Map to private fn in `annex_b/dtx/encode.rs`
- `Update_sumAcf(DtxState)` in `DTX.C`: shifts autocorrelation history buffer after SID frame emission. Map to private fn in `annex_b/dtx/encode.rs`
- `Qnt_e(errcl)` in `qsidlsf.c`: static helper performing actual codebook search within `lsfq_noise()`. Map to private fn or closure within `lsfq_noise` in `annex_b/cng/sid.rs`
- `New_ML_search_1(d_data, J, new_d, new_J, ncd)` in `qsidlsf.c`: static helper for stage-1 ML search within `lsfq_noise()`. Map to private fn in `annex_b/cng/sid.rs`
- `New_ML_search_2(d_data, weight, J, new_d, new_J, ncd)` in `qsidlsf.c`: static helper for stage-2 weighted ML search within `lsfq_noise()`. Map to private fn in `annex_b/cng/sid.rs`
- `Quant_Energy(prev_energy, cur_energy, sid_gain)` in `qsidgain.c`: static helper for energy quantization within `Qua_Sidgain()`. Map to private fn in `annex_b/cng/sid.rs`
- `Init_lsfq_noise()` in `dec_sid.c:131`: **eliminated in Rust** — computes `noise_fg[1]` from `fg` table at runtime. Called from BOTH `cod_ld8a.c:160` (`Init_Coder_ld8a`) and `dec_ld8a.c:106` (`Init_Decod_ld8a`), so both encoder and decoder init paths are affected. **Verified line references**: declaration in `sid.h:14`, implementation at `dec_sid.c:131-145`, encoder call site at `cod_ld8a.c:160`, decoder call site at `dec_ld8a.c:106`. Values are deterministic from the `fg` table, so pre-computed as `const` arrays in `tables/sid.rs` — both call sites are eliminated. **Initialization dependency change**: In C, the `noise_fg` global array (`tab_dtx.c`) is mutable and written at runtime by `Init_lsfq_noise()`. In Rust, it becomes an immutable `const` — this removes `Init_lsfq_noise` from the initialization dependency graph entirely, so neither `EncoderState::new()` nor `DecoderState::new()` need to call it

**Annex B spec Table B.3 note:** The specification lists `Miscel.c` / `Miscel.h` (Miscellaneous Calculations) as Annex B modules, but these files do not exist in the g729ab_v14 reference code or in the Release 3 `g729AnnexB/c_codeBA/` directory. Their functions were absorbed into other files in the combined AB codebase. No Rust equivalent is needed.

**Cross-phase dependency (bidirectional):** Phase 8 creates `annex_b/cng/sid.rs` (SID frame encode/decode: `lsfq_noise`, `Qua_Sidgain`) which Phase 9 consumes. Phase 9 creates `annex_b/cng/state.rs` (`CngDecState`) which Phase 8's `Cod_cng()` requires for decoder-side CNG. Phase 8's `Cod_cng()` also calls Phase 9's `Calc_exc_rand` in `annex_b/cng/excitation.rs` (with `FLAG_COD`; see Key decisions). Since Phases 7-9 are all in Batch 5, the skeleton generation for B5 must create all CNG type definitions and function stubs (`state.rs` + `sid.rs` + `excitation.rs`) in Pass 2 before either Phase 8 or Phase 9 TDD begins.

**DtxState vs CngDecState partition (CngEncState eliminated):** All `dtx.c` static variables (listed in DtxState above) belong to DtxState. `CngEncState` is **unnecessary** as a separate struct — all encoder-side CNG state originates from either `dtx.c` (in `DtxState`) or `cod_ld8a.c` (in `EncoderState`, e.g., `seed`). The encoder's `L_exc_err[4]` is already on `EncoderState` (Phase 6 init table row 16). `Calc_exc_rand(FLAG_COD)` accesses `L_exc_err` through the encoder state, not through a separate CNG state. Phase 9 creates only `CngDecState` in `annex_b/cng/state.rs` (no `CngEncState`). CngDecState holds decoder-side CNG state from `dec_sid.c`: `lspSid[M]`, `sid_gain`, `cur_gain`, `noise_fg_sum` access, `seed` (= INIT_SEED=11111), and `sid_sav`/`sh_sid_sav` (energy fallback state already in DecoderState). The boundary is: DtxState = encoder-side DTX/stationarity/SID-generation state; CngDecState = decoder-side SID-consumption/comfort-noise-synthesis state.

**TDD requirements:**

- Integration tests in `g729/tests/integration/annex_b_conformance.rs`: 4 encoder test functions (tstseq1-4)
- DTX state machine transition unit tests
- SID frame content (LSF + gain indices) verification against reference
- `noise_fg` pre-computation verification: a unit test that independently computes `noise_fg[1][i][j] = (19660*fg[0][i][j] + 13107*fg[1][i][j]) >> 15` at test time using the crate's DSP math functions and compares against the `const` values in `tables/sid.rs`. This catches any transcription error in the pre-computed table vs the `Init_lsfq_noise()` C runtime computation

#### SPEC_PHASE_09_cng.md

**Sources:**

| Source | What it provides |
|--------|-----------------|
| [Implementation Plan Phase 9](../implementation/phase_09_cng.md) | Function list (~5 functions), TDD workflow |
| `reference/.../g729ab_v14/dec_sid.c` | SID decoding |
| `reference/.../g729ab_v14/calcexc.c` | Random excitation generation |
| `PRD.md` §8.3 | CNG requirements |

**Key decisions:**

- CNG seed handling: INIT_SEED=11111, reset on **every** ftyp==1 frame (not just voice->noise transitions)
- Gain smoothing with first-frame conditional (`dec_sid.c:105-113`): when `past_ftyp == 1` (first noise frame after speech), `cur_gain = sid_gain` (step change, no smoothing — `dec_sid.c:106`). On subsequent noise frames (`past_ftyp != 1`): `cur_gain = mult_r(A_GAIN0, cur_gain) + mult_r(A_GAIN1, sid_gain)` where A_GAIN0=28672 (0.875 Q15), A_GAIN1=4096 (0.125 Q15). Both multiplications use `mult_r` (rounding multiply, `dec_sid.c:109-110`), not `mult` — using `mult` produces off-by-one errors causing bit-exact failures. `cur_gain = 0` initial value (C static default in `dec_sid.c`; affects gain smoothing behavior on the first noise frame). The TDD spec must test both paths: step assignment on voice-to-noise transition, and smooth convergence on consecutive noise frames
- CNG gain cap: `G_MAX = 5000` (`ld8a.h`) — maximum allowed CNG fixed codebook gain. Applied **inside** `Calc_exc_rand()` (`calcexc.c:240-245`) to the computed fixed codebook gain `g`, NOT to `cur_gain` in `Dec_cng()`. The cap is bilateral: `if(g >= 0) { if(sub(g, G_MAX) > 0) g = G_MAX; } else { if(add(g, G_MAX) < 0) g = negate(G_MAX); }`. Note: `Dec_cng()` does NOT cap `cur_gain` before passing it to `Calc_exc_rand()` — the gain smoothing output flows directly into `Calc_exc_rand(cur_gain, exc, seed, FLAG_DEC)` at `dec_sid.c:113`
- `lspSid` initial values: {31441, 27566, 21458, 13612, 4663, -4663, -13612, -21458, -27566, -31441} (different from speech decoder)
- `sid_lsfq_decode()` uses `noise_fg_sum[MODE][M]` from `tables/sid.rs` (via `Lsp_prev_compose`): `dec_sid.c:179-180` passes `noise_fg_sum[index[0]]` as the weighting vector. This table is defined in `tab_dtx.c` and must be available in Phase 9
- **SID LSP stability (encoder/decoder asymmetry):** The decoder's `sid_lsfq_decode()` in `dec_sid.c` enforces a minimum LSF gap using an **inline loop** (`dec_sid.c:166-176`, gap=10 in Q13 via `L_mult`/`L_mac`), then calls `Lsp_stability(lsfq)` (`dec_sid.c:186`). The encoder's `lsfq_noise()` in `qsidlsf.c` instead calls `Lsp_expand_1_2(tmpbuf, 10)` (`qsidlsf.c:111`) for the same gap enforcement, then `Lsp_stability(lsfq)` (`qsidlsf.c:121`). The Rust implementation must replicate this asymmetry: use the inline loop logic for the decoder's `sid_lsfq_decode`, and call `lsp_expand_1_2` for the encoder's `lsfq_noise`
- **Decoder inline gap loop exact algorithm (`dec_sid.c:166-176`):** SPEC_PHASE_09 must document this pseudocode verbatim, since an implementer must NOT substitute `Lsp_expand_1_2`:
  ```
  for j in 1..M {
      acc0 = L_mult(tmpbuf[j-1], 16384);      // Word32: tmpbuf[j-1] * 32768
      acc0 = L_mac(acc0, tmpbuf[j], -16384);   // -= tmpbuf[j] * 32768
      acc0 = L_mac(acc0, 10, 16384);           // += 10 * 32768 (gap=10 in Q13)
      k = extract_h(acc0);                     // k = (tmpbuf[j-1] - tmpbuf[j] + 10) / 2 (truncation)
      if k > 0 {
          tmpbuf[j-1] = sub(tmpbuf[j-1], k);  // push lower element down
          tmpbuf[j] = add(tmpbuf[j], k);       // push upper element up
      }
  }
  ```
  **Precision difference vs `Lsp_expand_1_2`:** The inline loop performs intermediate arithmetic in Word32 via `L_mult`/`L_mac`/`extract_h`, while `Lsp_expand_1_2` (`lspgetq.c:93-108`) uses Word16 via `sub`/`add`/`shr`: `diff = sub(buf[j-1], buf[j]); tmp = shr(add(diff, gap), 1)`. Both compute `floor((buf[j-1] - buf[j] + gap) / 2)` and are numerically equivalent for non-saturating inputs (typical LSF values in Q13 are well within Word16 range). However, the 32-bit path avoids potential saturation for extreme edge-case LSF values. The Rust decoder must use the 32-bit formulation to match `dec_sid.c` bit-exactly. **TDD test:** Include a test case with closely-spaced LSF pairs where `buf[j-1] - buf[j] + gap` exceeds 32767 to confirm the 32-bit and 16-bit paths diverge, proving the distinction matters for robustness
- `sid_sav`/`sh_sid_sav` recovery: consumes energy saved by Phase 5 decoder when first SID frame after speech is erased
- BFI handling with Annex B: `bfi==1 && past_ftyp==1` -> speech erasure; `bfi==1 && past_ftyp!=1` -> DTX erasure (continue CNG). Write `*parm = ftyp` (V1.3 update)
- **`Dec_cng()` LP interpolation and LSP memory update:** After gain smoothing, `Dec_cng()` calls `Int_qlpc(lsp_old, lspSid, A_t)` (`dec_sid.c:116`) to interpolate between the previous quantized LSP (`lsp_old`) and the current SID LSP (`lspSid`) for per-subframe LP coefficients, then copies `lspSid` into `lsp_old` (`dec_sid.c:117`) to update the persistent quantized LSP memory. These two lines are critical for correct LP filter continuity across CNG frames
- **`Dec_cng()` C-to-Rust parameter mapping** (`dec_sid.c:63-74`):

| # | C Parameter | C Type | I/O | Q-format | Description | Rust Equivalent |
|---|------------|--------|-----|----------|-------------|-----------------|
| 1 | `past_ftyp` | `Word16` | i | integer | Previous frame type (1=speech triggers step gain assignment) | `past_ftyp: i16` from `DecoderState` |
| 2 | `sid_sav` | `Word16` | i | varies | Saved SID energy for gain recovery on erased first-SID frames | `sid_sav: i16` from `DecoderState` |
| 3 | `sh_sid_sav` | `Word16` | i | integer | Scaling factor for `sid_sav` | `sh_sid_sav: i16` from `DecoderState` |
| 4 | `*parm` | `Word16 *` | i | integer | Coded SID parameters (4 values from `parm[2..5]` after BFI/ftyp) | `parm: &[i16]` (SID parameter slice) |
| 5 | `*exc` | `Word16 *` | i/o | Q0 | Excitation buffer (written with CNG excitation via `Calc_exc_rand`) | `&mut exc[exc_offset..]` slice of `DecoderState.old_exc` |
| 6 | `*lsp_old` | `Word16 *` | i/o | Q15 | Previous quantized LSP vector (updated to `lspSid` after interpolation) | `&mut DecoderState.lsp_old` |
| 7 | `*A_t` | `Word16 *` | o | Q12 | Output: 2×(M+1) interpolated LP coefficients (2 subframes) | `&mut [i16; 2 * (M + 1)]` local scratch |
| 8 | `*seed` | `Word16 *` | i/o | integer | Random generator seed for CNG excitation | `&mut DecoderState.seed` |
| 9 | `freq_prev[MA_NP][M]` | `Word16 [4][10]` | i/o | Q13 | Previous LSF vectors for MA prediction (updated by `sid_lsfq_decode`) | `&mut DecoderState.freq_prev` |

- `sharp = SHARPMIN` reset during non-active frames
- Static helper functions in `CALCEXC.C` for C-to-Rust mapping: `Gauss(seed)` generates Gaussian random values via sum of 12 `Random(&seed)` calls, right-shifted by 7; `Sqrt(x)` computes integer square root returning `sqrt(Num/2)`, NOT `sqrt(Num)` (`calcexc.c:296` comment: "returns sqrt(Num/2)"). This factor-of-2 is intentional and compensated by `FRAC1=19043` in the Gaussian energy normalization formula (`calcexc.c:125-141`): `fact = mult_r(cur_gain, FRAC1)` where `FRAC1` encodes `(sqrt(L_SUBFR)*alpha/2 - 1)*32768`. `Sqrt` is distinct from `Inv_sqrt()` in Phase 1's `dsp/div.rs` — it is a CNG-specific forward square root, not the inverse square root used by LP analysis. Both map to private helpers in `annex_b/cng/excitation.rs`, NOT to the DSP math layer. An implementer using a standard integer sqrt (without the /2) would produce incorrect CNG excitation energy normalization. **Implementation note:** `Sqrt()` computes `sqrt(Num/2)` intrinsically via its initial approximation `app = 16384` (= 2^14 = sqrt(2^28) = sqrt(MAX_32/2)), NOT by computing `sqrt(Num)` then dividing by 2. The algorithm is a bit-by-bit binary search from this starting point. An implementation using a standard integer sqrt followed by division could produce different rounding in the binary search iterations. Replicate the reference algorithm exactly (`calcexc.c:271-302`)
- **`Sqrt()` bit-exact convergence proof (mandatory for SPEC_PHASE_09):** The `Sqrt()` function (`calcexc.c:297-312`) uses `L_mult(add(Rez, Exp), add(Rez, Exp))` as the comparison accumulator. Since `L_mult(a, a) = 2*a*a` (by ITU basic_op definition), the convergence condition at each iteration is `Num >= 2*(Rez+Exp)^2`, i.e., `Rez+Exp <= sqrt(Num/2)`. After 14 binary-search iterations (Exp halving from 2^14 to 2^0), `Rez = floor(sqrt(Num/2))`. The initial approximation `Exp = 0x4000 = 16384 = 2^14` equals `sqrt(2^28) = sqrt(MAX_32/2)`, establishing the scale intrinsically. **Verification test case:** For `Num = 2*K*K` where K is any positive Word16, `Sqrt(2*K*K)` must equal `K`. SPEC_PHASE_09 must include: (a) this proof trace in the `Sqrt()` function specification, (b) a TDD test asserting `Sqrt(L_mult(100, 100)) == 100` (since `L_mult(100,100) = 20000`, and `sqrt(20000/2) = 100`), and (c) a test asserting `Sqrt(L_k)` matches the C reference output for the discriminant values from the first CNG subframe of tstseq1a
- **CNG fixed codebook gain root selection** (Annex B spec B.4.4): When solving the quadratic `4X^2 + 2bX + c = 0` for Gf, select the root with the **lowest absolute value**. Reference: `calcexc.c:234-237` — the two roots are computed as `x1 = Sqrt(disc) - inter_exc` and `x2 = -(inter_exc + Sqrt(disc))`, then `if(abs_s(x2) < abs_s(x1)) x1 = x2;` selects the smaller-magnitude root. The same selection logic applies to both the Stage 1 Gf quadratic and the Stage 2 beta quadratic (line 237 is shared by both code paths)
- **`Calc_exc_rand` internal subframe loop:** `Calc_exc_rand` contains an internal loop of 2 iterations (subframes of `L_SUBFR=40` samples each), generating independent random pitch delays, gains, and pulse positions per subframe. The `update_exc_err(Gp, t0)` call (gated by `flag_cod`) also occurs inside each subframe iteration (`calcexc.c:59,258`), not once per frame. SPEC_PHASE_09 pseudocode must reflect this per-subframe structure
- **`Calc_exc_rand` FLAG_COD/FLAG_DEC parameter** (`calcexc.c:35`): The function takes `Flag flag_cod` distinguishing encoder (`FLAG_COD=1`) from decoder (`FLAG_DEC=0`) operation. When `flag_cod == FLAG_COD`, `update_exc_err(Gp, t0)` is called inside each CNG subframe loop iteration (`calcexc.c:59,258`) to keep the taming state (`L_exc_err[4]`) current during DTX frames. When `flag_cod == FLAG_DEC`, the taming update is skipped. In Rust, map to a `bool` parameter (e.g., `update_taming: bool`). The decoder's `Dec_cng` calls with `false` (`dec_sid.c:113`); the encoder's `Cod_cng` calls with `true` (`dtx.c:216`). Omitting this distinction would leave the encoder's taming state stale during silence periods, causing incorrect taming behavior when speech resumes
- CNG excitation generation is a **two-stage** process (`calcexc.c:171-239`, Annex B B.4.4):
  - **Stage 1 (ACELP-like):** Generate adaptive excitation `ea(n)` using a random pitch delay in [40,103], plus fixed ACELP excitation `ef(n)` from random 4-pulse positions/signs. Solve the quadratic `4X^2 + 2bX + c = 0` for Gf (fixed codebook gain) where `b` = interaction term (sum of excitation at pulse positions), `c` = target energy minus adaptive energy. Compose `ex1(n) = Ga*ea(n) + Gf*ef(n)`
  - **Stage 2 (Mixture):** Generate Gaussian white noise `ex2(n)`. When the Stage 1 discriminant (`b^2 - 4c`) is negative, the adaptive excitation is zeroed and a second quadratic is solved using `K0 = (1-alpha^2)` to find `beta`. Final excitation: `ex(n) = alpha*ex1(n) + beta*ex2(n)` with alpha=0.5 (code, NOT 0.6 per spec text)
- CNG excitation mixture alpha: Annex B spec B.4.4 text says alpha=0.6, but the ITU reference code uses alpha=0.5 (`K0=24576 = (1-0.5^2)*32768`, `dtx.h:96`; `calcexc.c:122` comment explicitly states "alpha = 0.5"). Code is authoritative. PRD §8.3 already correctly documents `K0=24576 (= 1 - alpha^2 with alpha=0.5, in Q15)` — no PRD erratum needed. The discrepancy is between the **Annex B specification text** and the **ITU reference code**, not between the PRD and the code
- **bcg729 RFC 3389 CN interworking:** bcg729 implements RFC 3389 comfort noise payload support (`bcg729GetRFC3389Payload()`, decoder RFC 3389 flag, `cng.c:247-313`) as an extension beyond the ITU specification's native Annex B SID frames. This does not affect bit-exact conformance against ITU reference test vectors, but interop testing should be aware of this alternative CN pathway. Our implementation targets only native Annex B SID frame CNG per the ITU specification

**Module file mapping:** `annex_b/cng/state.rs`, `annex_b/cng/decode.rs`, `annex_b/cng/excitation.rs`. Also depends on: `annex_b/cng/sid.rs` (created in Phase 8)

**TDD requirements:**

- Integration tests in `g729/tests/integration/annex_b_conformance.rs`: 6 decoder test functions (tstseq1a-4a + tstseq5-6)
- CNG seed reset verification, SID frame decoding, gain smoothing unit tests
- `Sqrt()` convergence verification: `Sqrt(L_mult(K, K)) == K` for K in {1, 100, 1000, 16383}; `Sqrt(0) == 0`; `Sqrt(MAX_32) == 23170` (= floor(sqrt(MAX_32/2)))

**Annex B test vector naming convention:** The `a`-suffix variants (e.g., `tstseq1a.bit`, `tstseq1a.out`) are for G.729A+B (this implementation). Non-`a` variants (e.g., `tstseq1.bit`, `tstseq1.out`) are for base G.729+B and are NOT used. Spec documents should always reference the `a`-suffix files for conformance testing. **Naming asymmetry for tstseq5/6:** Unlike tstseq1-4 which have both `a` and non-`a` `.bit` files, tstseq5 and tstseq6 have NO `a`-suffix `.bit` input files -- the input bitstreams are shared between base G.729+B and G.729A+B. The correct file pairings for this implementation are: `tstseq5.bit` (input, no `a`) → `tstseq5a.out` (reference output, with `a`), and `tstseq6.bit` (input, no `a`) → `tstseq6a.out` (reference output, with `a`). Test infrastructure must not look for `tstseq5a.bit` or `tstseq6a.bit` -- they do not exist.

**Annex B test vector directory duplication:** The ITU reference code ships Annex B test vectors in two locations: `G729_Release3/g729AnnexB/test_vectors/` and `g729_annex_b_test_vectors/`. These directories contain identical files. Prefer the `g729AnnexB/test_vectors/` path (inside the Release 3 bundle) as the canonical source, since it is co-located with the reference C code. If both are present, a checksum verification step in the test infrastructure should confirm they are byte-identical to prevent silent divergence.

**Conformance gate for B5:** All 10 Annex B test vectors (4 encoder + 6 decoder) must pass bit-exactly. The 4 encoder vectors require VAD (Phase 7) + DTX (Phase 8), while the 6 decoder vectors require CNG (Phase 9).
