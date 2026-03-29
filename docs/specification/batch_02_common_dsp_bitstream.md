> Part of [Specification Plan](README.md)

### Batch 2: Phase 3 (Common DSP) + Phase 4 (Bitstream)

#### SPEC_PHASE_03_common_dsp.md

**Sources:**

| Source | What it provides |
|--------|-----------------|
| [Implementation Plan Phase 3](../implementation/phase_03_common_dsp.md) | Function list (~18 functions: 15 external + 3 static helpers), test plan, C dump fixtures. `Lag_window` is one of the 15 external functions; white noise correction is an inline sub-operation of `Autocorr`, not a standalone function |
| `reference/.../g729ab_v14/lpc.c` | LP analysis (Autocorr, Lag_window, Levinson), LP-to-LSP (Az_lsp, Chebps_11, Chebps_10) |
| `reference/.../g729ab_v14/lpcfunc.c` | LSP-to-LP (Lsp_Az, Get_lsp_pol), LSF conversion (Lsp_lsf2, Lsf_lsp2, Lsp_lsf), Weight_Az, Int_qlpc |
| `reference/.../g729ab_v14/filter.c` | Syn_filt, Residu, Convolve |
| `reference/.../g729ab_v14/postfilt.c` (not declared in ld8a.h; external linkage, de facto module-private) | `preemphasis` (tilt compensation; mapped to `filter/preemph.rs` in Rust) |
| `reference/.../g729ab_v14/util.c` | Copy, Set_zero utilities (map to Rust std: `.copy_from_slice()`, `.fill()`) |
| `reference/.../g729ab_v14/pred_lt3.c` | Pred_lt_3 sinc interpolation |
| `PRD.md` §3.2-3.5 | LP analysis requirements |

**Key decisions to document:**

- Autocorrelation overflow-retry strategy (PRD §3.2.2): how `DspContext.overflow` drives the loop. The `Autocorr` function accepts a **variable order** parameter: M=10 when Annex B is disabled, NP=12 when Annex B is active. The latter produces 13 lags (0-12) shared between LP analysis and VAD. Reference: `LPC.C:28-62`, `COD_LD8A.C:231,242`
- `Lag_window`: bandwidth expansion on autocorrelation coefficients (PRD §3.2.3), applied to r(1)..r(m) using precomputed DPF high/low coefficient pairs from Phase 2 tables. Accepts variable order `m`: M=10 for base LP analysis, NP=12 when Annex B is active. The lag window tables (`lag_h[M+2]`, `lag_l[M+2]`) have exactly 12 entries to support the NP=12 case — these are NOT guard elements. Reference: `LPC.C:111-115`
- White noise correction (PRD §3.2.4): `r'(0) = r(0) * 1.0001` adds -40 dB noise floor to ensure positive-definiteness before Levinson-Durbin
- Levinson DPF precision: which DPF ops are used at each step. **Output parameters:** `Levinson(r, A, rc, &ener)` produces three outputs beyond the LP coefficients `A[M+1]`: (1) `rc[2]` (Word16 array) — the first two reflection coefficients; `rc[1]` (the second reflection coefficient, k_2) is consumed by VAD Phase 7 via `vad(rc[1], ...)` at `COD_LD8A.C:251-252`, (2) `ener` (Word32) — residual prediction error energy; consumed by DTX Phase 8 via `Cod_cng()`→`Levinson(curAcf, ..., &ener[0])` at `dtx.c:117`. Both must be in the function signature and documented in the per-phase spec. **Fallback state management:** Levinson maintains persistent state `old_A[M+1]` and `old_rc[2]`. On successful completion (all |k_i| < 1.0): copy final `A[]` to `old_A[]`, copy `rc[0..1]` to `old_rc[]`. On stability failure (|k_i| >= 1.0 at any iteration): stop recursion, copy `old_A[]` to output `A[]`, copy `old_rc[]` to output `rc[]`. This fallback mechanism is critical for bit-exactness — the Phase 3 spec must document the update/restore semantics explicitly. Reference: `LPC.C` `Levinson()` function
- `Az_lsp(A, lsp, lsp_old)` three-level fallback chain: **Function signature** takes three parameters: `A[M+1]` (input LP coefficients), `lsp[M]` (output LSP vector), `lsp_old[M]` (input previous frame's LSP, used as fallback). Recovery chain: (1) `Chebps_11` (Q11 precision, default). If `ovf_coef` flag is set (overflow during Chebyshev evaluation), retry with (2) `Chebps_10` (Q10 precision, reduced range). If even after Chebps_10, fewer than 10 roots are found (`nf < M`), fall back to (3) previous frame's LSP values from `lsp_old[]` (copied to output). This three-level recovery chain — Chebps_11 → Chebps_10 → previous LSP — must be documented as a single coherent mechanism. The `lsp_old` parameter must be included in the Rust function signature. Reference: `LPC.C` `Az_lsp()` function
- Az_lsp bisection count: 2 iterations (Annex A), not 4 (base G.729) -- see spec A.3.2.3
- Syn_filt overflow handling path (PRD §5.8.1). **`update` parameter semantics:** `Syn_filt(A, x, y, L, mem, update)` has a critical 6th parameter: `update=0` means the filter memory `mem[]` is read but NOT written back (trial synthesis for overflow detection), `update=1` means filter memory IS written back (committed synthesis). The decoder overflow retry pattern uses `update=0` first, and only calls with `update=1` after confirming no overflow or after scaling the excitation. In Rust, this maps to a `bool` or enum parameter on `syn_filt()`. Reference: `filter.c`, `dec_ld8a.c:169-181,331-344`
- `Lsp_Az` / `Get_lsp_pol`: reconstruct LP coefficients from LSP via polynomial expansion. `Get_lsp_pol` is a distinct subroutine called by `Lsp_Az` to build the 5th-order f1/f2 polynomials
- `Int_qlpc` is Annex-A-only (quantized LSP interpolation); `Int_lpc` does not exist in g729ab_v14 reference code. `Int_qlpc` is called by both encoder (Phase 6, for LP interpolation before subframe processing) and decoder (Phase 5, for interpolating decoded quantized LSPs). Function signature must serve both callers. **Internal dependency:** `Int_qlpc` internally calls `Lsp_Az` (from `lp/lsp_az.rs`) to convert the interpolated LSPs back to LP coefficients — it is NOT a pure interpolation function. The output is `Aq_t[2*(M+1)]`, two sets of LP coefficients (one per subframe). Reference: `LPCFUNC.C` `Int_qlpc()` function
- `Weight_Az` bandwidth expansion: `Ap[i] = a[i] * gamma^i` with gamma=0.75 (Annex A fixed)
- `Convolve`: impulse response convolution for codebook search, length L_SUBFR=40
- Pre-emphasis / de-emphasis: `signal[i] = signal[i] - coeff * signal[i-1]`, used for tilt compensation in post-filter
- `Copy` / `Set_zero` (from `util.c`): eliminated in Rust -- map to `.copy_from_slice()` and `.fill()` respectively. No Rust wrapper functions needed; affects C-to-Rust mapping table
- `Lsp_lsf2` / `Lsf_lsp2` (Q13-domain LSF<->LSP conversion): used by the LSP quantization pipeline (encoder `Qua_lsp`, decoder `Lsp_iqua_cs`), and by SID noise LSF coding (`qsidlsf.c`, `dec_sid.c`). `Lsp_lsf2` converts cosine-domain LSP (Q15) to frequency-domain LSF (Q13) using `table2[64]`/`slope_cos[64]`; `Lsf_lsp2` converts back using `table[65]`/`slope[64]`. Reference: `LPCFUNC.C:184-250`
- `Lsp_lsf` (Q15-domain LSP->LSF conversion): converts LSP to normalized LSF (Q15, range 0.0-0.5) using `table[65]`/`slope_acos[64]`. Called by encoder `cod_ld8a.c:250` to produce LSF values for VAD feature extraction. Distinct from `Lsp_lsf2` in Q-format and table usage. Reference: `LPCFUNC.C:142-180`
- `Lsf_lsp` (Q15-domain LSF->LSP conversion, `LPCFUNC.C`): **omitted** — this is the inverse of `Lsp_lsf` but is not called anywhere in the Annex A/B reference code (g729ab_v14). The codec uses `Lsf_lsp2` (Q13 domain) instead for all LSF-to-LSP conversions. Documented here for C-to-Rust mapping completeness to prevent future confusion

**Phase 3 function signature summary:**

| Function | Signature (Rust-style) | Key parameters | C source |
|----------|----------------------|----------------|----------|
| `Autocorr` | `(signal, r, order, &mut ctx)` | `order`: M=10 or NP=12 (Annex B) | `LPC.C:28-62` |
| `Lag_window` | `(r, m)` | `m`: M=10 or NP=12 (matches Autocorr order) | `LPC.C:111-115` |
| `Levinson` | `(r, A, rc, &ener, &mut state)` | `rc[2]` out, `ener` (Word32) out; `state` = old_A/old_rc | `LPC.C` |
| `Az_lsp` | `(A, lsp, lsp_old)` | `lsp_old` = previous frame fallback | `LPC.C` |
| `Lsp_Az` | `(lsp, A)` | internally calls `Get_lsp_pol` | `LPCFUNC.C:70` |
| `Int_qlpc` | `(lsp_old, lsp_new, Aq_t)` | internally calls `Lsp_Az`; output = 2×(M+1) LP coefficients | `LPCFUNC.C` |
| `Weight_Az` | `(A, gamma, Ap)` | gamma=0.75 (Annex A fixed) | `LPCFUNC.C` |
| `Syn_filt` | `(A, x, y, L, mem, update)` | `update`: bool (write-back mem) | `FILTER.C` |
| `Residu` | `(A, x, y, L)` | 10th-order FIR | `FILTER.C` |
| `Convolve` | `(h, x, y, L)` | L=L_SUBFR=40 | `FILTER.C` |
| `preemphasis` | `(signal, coeff, L, &mem)` | tilt compensation | `POSTFILT.C:345` |
| `Pred_lt_3` | `(exc, T0, frac, L)` | sinc interpolation via `inter_3l` | `PRED_LT3.C` |
| `Lsp_lsf2` / `Lsf_lsp2` | `(lsp/lsf, lsf/lsp, M)` | Q13-domain LSF<->LSP | `LPCFUNC.C:184-250` |
| `Lsp_lsf` | `(lsp, lsf, M)` | Q15-domain LSP->LSF (for VAD) | `LPCFUNC.C:142-180` |

**Module file mapping:** `lp/window.rs`, `lp/autocorr.rs` (`Autocorr`, `Lag_window`, white noise correction `r'(0) = r(0) * 1.0001`), `lp/levinson.rs`, `lp/az_lsp.rs`, `lp/lsp_az.rs` (includes `Get_lsp_pol` as a private helper building the 5th-order f1/f2 polynomials; `LPCFUNC.C:70`), `lp/lsf.rs`, `lp/weight.rs`, `lp/interp.rs`, `filter/syn.rs`, `filter/resid.rs`, `filter/convolve.rs`, `filter/preemph.rs`, `pitch/pred_lt3.rs`

**TDD requirements:**

- Boundary tests: all-zero autocorrelation (energy floor), DC signal, overflow retry, unstable Levinson, flat-spectrum Az_lsp
- Component tests against C reference dumps: `autocorr_frame{0..9}.bin`, `levinson_frame{0..9}.bin`, `az_lsp_frame{0..9}.bin`, `syn_filt_impulse.bin`, `pred_lt3_integer.bin`, `pred_lt3_frac.bin`
- Bit-exact match against C dumps for first 10 frames of SPEECH.IN

**C reference fixtures (prerequisite):** Run `bash tests/scripts/generate_c_dumps.sh` before writing component tests.

#### SPEC_PHASE_04_bitstream.md

**Sources:**

| Source | What it provides |
|--------|-----------------|
| [Implementation Plan Phase 4](../implementation/phase_04_bitstream.md) | Function list (~8 functions), test plan |
| `reference/.../g729ab_v14/bits.c` | Pack/unpack implementations |
| `PRD.md` §4.1-4.5 | Bitstream format, OCTET_TX_MODE |

**Key decisions to document:**

- OCTET_TX_MODE: SID frames are 16 bits (15 data + 1 padding zero) vs 15 bits. Annex B test vectors use OCTET_TX_MODE -- without this, SID frame tests fail
- BFI detection: any zero-valued bit word (0x0000, not BIT_0=0x007F) triggers frame erasure
- Frame type discrimination from SIZE_WORD: 80=speech(ftyp=1), 15/16=SID(ftyp=2), 0=no-tx(ftyp=0)
- SID frame pack/unpack: `bitsno2[4] = {1,5,4,5}` = 15 bits
- **`bits2prm_ld8k` output indexing convention:** `bits2prm_ld8k()` writes frame type to `prm[1]` and coded parameters to `prm[i+2]`, leaving `prm[0]` reserved for BFI. The calling function `read_frame()` fills `parm[0]` with the BFI flag. This +2 offset from the `bitsno[]` array index to the `prm[]` position is critical for correct Rust implementation of the ITU serial format parser. Full layout: `parm[0]`=BFI (from `read_frame()`), `parm[1]`=ftyp (from `bits2prm_ld8k()`), `parm[2..12]`=11 coded speech parameters or `parm[2..5]`=4 SID parameters. Reference: `BITS.C:108-180`
- **`Check_Parity_Pitch` called inside `read_frame()`:** After `bits2prm_ld8k()` returns, `read_frame()` calls `Check_Parity_Pitch(parm[4], parm[5])` for speech frames (`parm[1]==1`), writing the parity check result back into `parm[5]` in place (`bits.c:232-235`). This means the decoder (`Decod_ld8a`) receives a pre-checked parity value (0=parity OK, 1=parity error), not the raw parity bit. In the Rust implementation, this parity check must be performed during ITU serial format parsing in `bitstream/itu_serial.rs`, not inside the decoder main loop. Placing it in the wrong location would either skip the check or apply it with incorrect parm indices

**C-to-Rust function mapping:**
- `prm2bits_ld8k()` (`bits.c`) -> `bitstream/pack.rs`
- `bits2prm_ld8k()` (`bits.c`) -> `bitstream/unpack.rs`
- `read_frame()` (`bits.c`) -> `bitstream/itu_serial.rs` (ITU serial format parsing: SYNC_WORD/SIZE_WORD header, frame type discrimination from SIZE_WORD, BFI detection from zero-valued bit words)
- `static int2bin()` / `static bin2int()` (`bits.c`) -> eliminated/inlined (trivial bit manipulation helpers absorbed into pack/unpack implementations)

**Module file mapping:** `bitstream/pack.rs`, `bitstream/unpack.rs`, `bitstream/itu_serial.rs`

**TDD requirements:**

- Round-trip tests: `bits2prm(prm2bits(prm)) == prm`
- ITU serial format parsing: first frame of SPEECH.BIT
- BFI detection, SID frame parse (both OCTET and non-OCTET modes), frame type routing
