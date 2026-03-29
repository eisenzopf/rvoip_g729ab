> Part of [G.729AB Implementation Plan](README.md)

### Phase 3: Common DSP Functions

**Goal**: Implement shared signal processing functions used by both encoder and decoder.

**Files to create**:
- `src/lp/window.rs` — Apply asymmetric analysis window to speech buffer
- `src/lp/autocorr.rs` — `Autocorr()` with overflow-retry loop, `Lag_window()` bandwidth expansion on autocorrelation coefficients (`LPC.C:111-115`), and white noise correction `r'(0) = r(0) * 1.0001` (PRD §3.2.4)
- `src/lp/levinson.rs` — Levinson-Durbin recursion using DPF (double-precision format)
- `src/lp/az_lsp.rs` — `Az_lsp()` with `Chebps_11`/`Chebps_10` (LP -> LSP)
- `src/lp/lsp_az.rs` — `Lsp_Az()`, `Get_lsp_pol()` (LSP -> LP)
- `src/lp/lsf.rs` — `Lsp_lsf2()`, `Lsf_lsp2()` (Q13-domain LSF<->LSP conversion), `Lsp_lsf()` (Q15 LSP->LSF for VAD). Reference: `LPCFUNC.C:118-250`. Used by LSP quantization (encoder), LSP decoding (decoder), SID LSF coding (Annex B), and VAD feature extraction. Note: `Lsf_lsp()` (Q15-domain LSF->LSP, the inverse of `Lsp_lsf`) exists in `LPCFUNC.C` but is **not called** anywhere in the Annex A/B code — omitted to avoid dead code
- `src/lp/weight.rs` — `Weight_Az()` bandwidth expansion
- `src/lp/interp.rs` — `Int_qlpc()` LSP interpolation (Annex A only uses quantized LSP interpolation; `Int_lpc` does not exist in the reference code)
- `src/filter/syn.rs` — `Syn_filt()` LP synthesis filter
- `src/filter/resid.rs` — `Residu()` LP residual
- `src/filter/convolve.rs` — `Convolve()` for codebook search
- `src/filter/preemph.rs` — Pre-emphasis / de-emphasis
- `src/pitch/pred_lt3.rs` — `Pred_lt_3()` sinc interpolation for fractional pitch

**Reference**: `lpc.c`, `lpcfunc.c`, `filter.c`, `dspfunc.c`, `pred_lt3.c`

**Total**: ~18 functions (15 external + 3 static helpers: Chebps_11, Chebps_10, Get_lsp_pol), ~1,500 lines. `Lag_window` and white noise correction (`r'(0) = r(0) * 1.0001`) are counted within this total: `Lag_window` is one of the 15 external functions (from `LPC.C`), while white noise correction is an inline sub-operation of the `Autocorr` module (not a standalone function).

**Tasks**:
1. Implement `Autocorr` — windowing + autocorrelation with overflow retry loop (clear overflow, compute energy, if overflow set then right-shift signal and retry) — PRD §3.2.2. The order parameter is **variable**: M=10 when Annex B is disabled, NP=12 when Annex B is active (producing M+1=11 or NP+1=13 lags respectively). The function signature must accept an `order` parameter, not a hardcoded constant. Reference: `LPC.C:28-62`, `COD_LD8A.C:231,242`
2. Implement `Lag_window` — bandwidth expansion on autocorrelation coefficients. Accepts a variable order parameter `m`: when called with m=M=10, accesses `lag_h[0..9]`/`lag_l[0..9]`; when called with m=NP=12 (Annex B active), accesses `lag_h[0..11]`/`lag_l[0..11]` (all M+2=12 table entries). Reference: `LPC.C:111-115`
3. Apply white noise correction (PRD §3.2.4): `r'(0) = r(0) * 1.0001` — adds -40 dB noise floor to ensure positive-definiteness before Levinson-Durbin
4. Implement `Levinson` — 10th-order recursion with DPF precision (Mpy_32, Div_32). **Output parameters:** `Levinson(r, A, rc, &ener)` produces three outputs: `A[M+1]` (LP coefficients), `rc[2]` (first two reflection coefficients; `rc[0]` consumed by VAD Phase 7 at `COD_LD8A.C:248`), and `ener` (Word32 residual energy; consumed by DTX Phase 8 at `dtx.c:117`). **Fallback state:** maintains `old_A[M+1]` and `old_rc[2]` — on stability failure (|k_i| >= 1.0), stops recursion and restores output from saved state; on success, updates saved state from current output. Reference: `LPC.C`
5. Implement `Az_lsp` — Chebyshev polynomial root finding with 50-point grid + 2 bisection iterations (Annex A uses 2, not base G.729's 4; see spec A.3.2.3 and `lpc.c` Az_lsp: `for (i = 0; i < 2; i++)`); dual evaluators (Chebps_11 default, Chebps_10 fallback on overflow)
6. Implement `Lsp_Az` — reconstruct LP from LSP using `Get_lsp_pol`
7. Implement `Weight_Az` — bandwidth expansion: `Ap[i] = a[i] × gamma^i` for i=0..M. Used to compute `A(z/gamma)` with gamma=0.75 (Annex A). Reference: `LPCFUNC.C`
8. Implement `Int_qlpc` — per-subframe quantized LSP interpolation (SF1: 0.5×prev + 0.5×curr, SF2: curr), then convert interpolated LSPs back to LP via `Lsp_Az`. Annex A only interpolates quantized LSPs (spec A.3.2.5: "only the quantized LP coefficients are interpolated since the weighting filter uses the quantized parameters"); `Int_lpc` does not exist in the g729ab_v14 reference code. Reference: `LPCFUNC.C`
9. Implement `Lsp_lsf2` and `Lsf_lsp2` — Q13-domain conversion between cosine-domain LSP (Q15) and frequency-domain LSF (Q13). `Lsp_lsf2` uses `table2[64]` and `slope_cos[64]` tables; `Lsf_lsp2` uses `table[65]` and `slope[64]` tables. Used by: encoder `Qua_lsp()` (LSP→LSF before quantization, LSF→LSP after), decoder `Lsp_iqua_cs()` (LSF→LSP after dequantization), SID `qsidlsf.c`/`dec_sid.c` (both directions). Reference: `LPCFUNC.C:184-250`
10. Implement `Lsp_lsf` — Q15-domain LSP-to-LSF conversion using `table[65]`/`slope_acos[64]` tables. Distinct from `Lsp_lsf2` (different output Q-format and tables). Called by encoder `cod_ld8a.c:250` to convert LSP to LSF for VAD feature extraction (`Lsp_lsf(lsp_new, lsf_new, M)`). Reference: `LPCFUNC.C:142-180`. **Note:** `Lsf_lsp()` (Q15-domain LSF→LSP, the inverse of `Lsp_lsf`, also in `LPCFUNC.C`) is **omitted** — it is not called anywhere in the Annex A/B reference code. The codec uses `Lsf_lsp2` (Q13 domain) for all LSF-to-LSP conversions instead
11. Implement `Syn_filt` — 10th-order IIR filter with optional memory update + overflow detection
12. Implement `Residu` — 10th-order FIR filter
13. Implement `Convolve` — convolution of impulse response h(n) with codebook vector, length L_SUBFR=40. Used by adaptive and fixed codebook searches. Reference: `FILTER.C`
14. Implement pre-emphasis / de-emphasis — `preemphasis(signal, coeff, length, &mem)`: `signal[i] = signal[i] - coeff * signal[i-1]`. Used for tilt compensation in the post-filter. Note: the actual C function is named `preemphasis` (defined locally in `POSTFILT.C:345-368`, NOT in `FILTER.C`). The Rust module mapping to `filter/preemph.rs` is a logical reorganization
15. Implement `Pred_lt_3` — sinc interpolation using `inter_3l` table

**Test Plan**:

| Test | Input | Expected | Validates |
|------|-------|----------|-----------|
| `Autocorr` all-zero | 240 zeros | r[0]=1 (floor), r[1..10]=0 | Zero-energy floor |
| `Autocorr` DC signal | 240 samples all=16384 | r[0] large, r[k] decreasing | DC autocorrelation |
| `Autocorr` overflow retry | 240 samples all=MAX_16 | Retries with increasing scale until no overflow | Overflow retry loop |
| `Autocorr` SPEECH.IN frame 0 | First 240 samples from reference | r_h[], r_l[], exp_R0 match C reference dump | Bit-exact with reference |
| `Levinson` white noise | r[0]=big, r[1..10]=0 | A[0]=4096, A[1..10]=0 | Identity filter |
| `Levinson` SPEECH.IN frame 0 | Autocorrelation from above | A[0..10] match C reference | LP coefficient accuracy |
| `Levinson` unstable | Crafted r[] with \|K\| > 32750 | Falls back to old_A | Stability check |
| `Az_lsp` flat spectrum | A=[4096,0,...,0] | Evenly-spaced LSPs | Simple case |
| `Az_lsp` SPEECH.IN frame 0 | LP coefficients from above | LSP values match C reference | Root finding accuracy |
| `Az_lsp` overflow fallback | Crafted A[] triggering Chebps_10 | Same roots, Q10 path | Dual evaluator |
| `Lsp_Az` round-trip | `Lsp_Az(Az_lsp(A))` | Recovers original A (within precision) | Conversion fidelity |
| `Syn_filt` impulse | Impulse input, known A[] | First 40 samples match reference | Filter accuracy |
| `Syn_filt` overflow | Crafted excitation causing overflow | Overflow flag set, output saturated | Overflow detection |
| `Pred_lt_3` integer pitch | T0=60, frac=0 | Simple copy from exc[-60] | Integer delay |
| `Pred_lt_3` fractional | T0=60, frac=1 | Interpolated output matches reference | Sinc interpolation |

**Generate reference data**: Instrument C `Autocorr`, `Levinson`, `Az_lsp` to dump intermediate/final values for the first 10 frames of SPEECH.IN. Store as binary test fixtures.

**Prerequisite — Generate C Intermediate Dumps (Task 0)**:

Before writing Rust component tests, run the C reference dump generator to produce binary fixtures:
```
bash tests/scripts/generate_c_dumps.sh
```
This produces fixture files in `tests/fixtures/c_dumps/`:
- `autocorr_frame{0..9}.bin` — r_h[11], r_l[11], exp_R0 per frame
- `levinson_frame{0..9}.bin` — A[11] LP coefficients per frame
- `az_lsp_frame{0..9}.bin` — lsp[10] values per frame
- `syn_filt_impulse.bin` — 40-sample synthesis filter impulse response
- `pred_lt3_integer.bin`, `pred_lt3_frac.bin` — Pred_lt_3 outputs
- `random_fer_seq.bin`, `random_cng_seq.bin` — Random() LCG sequences

Rust tests load these via `include_bytes!` or the `G729_FIXTURE_DIR` environment variable. If fixtures are not present, component tests that require them should be `#[ignore]`d with a message pointing to the generation script.

**Exit Criteria**:
1. All functions pass boundary and reference tests (all rows above)
2. Bit-exact match against C reference dumps for first 10 frames of SPEECH.IN
3. Overflow retry path exercised and verified for high-energy input

**TDD Workflow**:
1. **Generate fixtures**: Run `bash tests/scripts/generate_c_dumps.sh` to produce C reference dumps.
2. **Write tests first**: Create Rust integration tests (`tests/component/autocorr.rs`, etc.) that load the fixture files and assert bit-exact output from each function. Write boundary tests (all-zero input, DC signal, overflow cases) as unit tests. All tests fail initially.
   ```
   cargo test --test component                      # expect: 0 passed
   ```
3. **Implement incrementally**: Implement functions in dependency order (Autocorr -> Lag_window -> Levinson -> Az_lsp -> Lsp_Az -> Lsp_lsf2/Lsf_lsp2/Lsp_lsf -> Weight_Az -> Int_qlpc -> Syn_filt -> Residu -> Convolve -> preemph -> Pred_lt_3). Run tests after each function.
4. **Verify**:
   ```
   cargo test --lib lp:: filter:: pitch::pred_lt3   # unit tests pass
   cargo test --test component                      # component tests vs C dumps pass
   python tests/scripts/run_all_tiers.py --phase 3  # exits 0
   ```
5. **Gate**: All Tier 0 cargo tests pass. C reference dumps match bit-exactly for 10 frames.
