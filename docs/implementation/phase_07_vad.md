> Part of [G.729AB Implementation Plan](README.md)

### Phase 7: Annex B — Voice Activity Detection (VAD)

**Goal**: Implement encoder-side VAD that classifies frames as voice or noise.

**Files to create**:
- `src/annex_b/vad/state.rs` — VadState struct
- `src/annex_b/vad/features.rs` — Extract: full-band energy, low-band energy, spectral distortion, zero-crossing rate
- `src/annex_b/vad/decision.rs` — `MakeDec()`: 14 linear discriminant conditions
- `src/annex_b/vad/detect.rs` — `vad()`: main decision with initialization (frames 0-32, 33 frames total, per INIT_FRAME=32 and `sub(frm_count, INIT_FRAME) <= 0` check), smoothing (4 stages), background noise update

**Reference**: `vad.c`

**Total**: ~4 functions, ~450 lines

**Tasks**:
1. Define VadState with running means, counters, min energy buffer
2. Implement feature extraction: Ef (full-band log energy), El (low-band via lbf_corr), SD (LSF distance), ZC (zero-crossing count) — PRD §8.1.1. Note: when Annex B is active, the encoder's main `Autocorr()` call uses order NP=12 (not M=10), producing 13 autocorrelation lags (0-12). `Lag_window()` and `Levinson()` also receive these 13-lag arrays — Levinson uses only the first M+1=11 values for LP coefficient computation, while the VAD's energy features (Ef, El via `lbf_corr[13]`) use all 13 values. The VAD does NOT compute its own autocorrelation; it shares the encoder's NP=12 result. Reference: `COD_LD8A.C:231,242` (`r_h[NP+1]`, `Autocorr(p_window, NP, ...)`). When the `annex_b` feature is disabled, `Autocorr` can use order M=10 instead
3. Implement MakeDec: 14 linear discriminant functions (any match -> VOICE) — PRD §8.1.2
4. Implement initialization (frames 0-32, i.e., 33 frames total where `sub(frm_count, INIT_FRAME) <= 0` with INIT_FRAME=32): hard threshold Ef < 3072 -> NOISE — PRD §8.1.3
5. Implement 4-stage smoothing: inertia (6 frames), energy hangover (2dB), extension (4 frames), forced noise — PRD §8.1.3
6. Implement background noise update with rate-adaptive coefficients and 16-entry sliding minimum buffer — PRD §8.1.4

**Test Plan**:

| Test | Input | Expected | Validates |
|------|-------|----------|-----------|
| Silence detection | 1 second of silence | VAD=NOISE after frame 32 | Noise detection |
| Speech detection | Voiced speech | VAD=VOICE | Voice detection |
| Initialization period | First 33 frames (0-32) | VOICE if Ef >= 3072, NOISE if Ef < 3072 | Init threshold |
| Transition speech->silence | Speech then silence | VOICE -> SID -> no-tx | State machine |
| tstseq1.bin | Annex B test sequence | VAD decisions match reference | Bit-exact VAD |
| Smoothing inertia | Brief noise during speech | Stays VOICE for 6 frames | Inertia smoothing |

**Exit Criteria**:
1. VAD decisions match reference for all Annex B test sequences
2. All 4 smoothing stages verified with targeted test cases
3. Background noise update coefficients match PRD §8.1.4 tables

**TDD Workflow**:
1. **Write tests first**: Create unit tests for VAD feature extraction (silence -> low energy, speech -> high energy), MakeDec discriminants, initialization period (first 33 frames: VOICE if Ef >= 3072, NOISE if Ef < 3072), and smoothing stages. These are `#[cfg(feature = "annex_b")]` tests.
   ```
   cargo test --lib annex_b::vad --features annex_b  # expect: 0 passed
   ```
2. **Implement**: VadState -> feature extraction -> MakeDec -> smoothing -> background noise update.
3. **Verify**: VAD alone cannot be tested end-to-end against ITU vectors (requires DTX in Phase 8 to produce Annex B bitstreams). Unit-level verification only at this stage.
   ```
   cargo test --lib annex_b::vad --features annex_b  # unit tests pass
   python tests/scripts/run_all_tiers.py --phase 7   # Tier 1 runs Annex A + Annex B decoder
   ```
4. **Note**: Annex B decoder vectors (CNG) can be tested at Phase 7 if CNG is implemented in parallel. Annex B encoder vectors require Phase 8 (DTX).
