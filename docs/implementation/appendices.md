> Part of [G.729AB Implementation Plan](README.md)

# Appendices

## Dependency Graph and Critical Path

```
Phase 1 (DSP Math) ──┬──> Phase 3 (Common DSP) ──┬──> Phase 5 (Decoder) ──> Phase 6 (Encoder)
                      │                            │         │                      │
Phase 2 (Tables) ─────┘    Phase 4 (Bitstream) ───┘         │                      │
                                                              │                      │
                                                    Phase 9 (CNG) ───┐    Phase 7 (VAD)
                                                                     │         │
                                                                     │    Phase 8 (DTX)
                                                                     │         │
                                                                     └────┬────┘
                                                                          │
                                                                    Phase 10 (API/CLI)
```

**Critical path**: Phase 1 -> Phase 3 -> Phase 5 -> Phase 6 -> Phase 7 -> Phase 8 -> Phase 10

**Parallelizable pairs**:
- Phase 1 + Phase 2 (partial — tables need Word16/Word32 types)
- Phase 3 + Phase 4 (independent once Phase 1/2 done)
- Phase 7 (VAD) + Phase 9 (CNG) (encoder-side vs decoder-side)

---

## Conformance Checkpoints

| Gate | After Phase | Requirement |
|------|-------------|-------------|
| **Gate 1** | Phase 1 | All 38 DSP functions pass boundary + reference tests |
| **Gate 2** | Phase 2 | All tables verified against C source (automated checksums) |
| **Gate 3** | Phase 5 | **All 10 Annex A decoder test vectors bit-exact** |
| **Gate 4** | Phase 6 | **All 7 Annex A encoder test vectors bit-exact** + round-trip |
| **Gate 5** | Phase 9 | **All 10 Annex B test vectors bit-exact** |
| **Gate 6** | Phase 10 | Performance targets met, fuzz clean, `no_std` verified |

---

## Functional Requirement Validation Matrix

Each PRD section has explicit input/output validation mapped to implementation phases:

| # | PRD Section | Functional Block | Validation Method | Phase |
|---|------------|------------------|-------------------|-------|
| 1 | §3.1 | Pre-processing (HP filter) | Impulse and sine fixtures; verify fixed-point filter output exactly | 6 |
| 2 | §3.2 | LP analysis | Compare autocorrelation, overflow-retry scaling, Levinson coefficients vs C dumps | 3 |
| 3 | §3.3 | LP→LSP conversion | Validate root count, fallback behavior, Chebps_11/Chebps_10 paths | 3 |
| 4 | §3.4 | LSP quantization | Index bounds, MA mode selection, stability enforcement, reconstructed vectors | 6 |
| 5 | §3.5 | LSP interpolation | Interpolation outputs for both subframes match reference | 3 |
| 6 | §3.6 | Weighting filter | Confirm Annex A fixed-gamma=0.75 behavior and generated `Ap` | 6 |
| 7 | §3.7 | Open-loop pitch | Verify decimation search ranges, submultiple preference, overflow rescaling | 6 |
| 8 | §3.8–3.9 | Target/impulse computation | Compare vectors against reference snapshots | 6 |
| 9 | §3.10 | Adaptive codebook search | Delay/index mapping and interpolation exactness | 6 |
| 10 | §3.11 | Fixed codebook search | Pulse positions/sign reconstruction, depth-first tree | 6 |
| 11 | §3.12 | Gain quantization + taming | Predictor update, clipping, NCAN candidate logic, taming constraint | 6 |
| 12 | §3.13 | Memory update | Verify buffer shifts and mem_w0 updates frame-to-frame | 6 |
| 13 | §4.1–4.5 | Bitstream speech/SID formats | Roundtrip and bit order tests, OCTET_TX_MODE handling | 4 |
| 14 | §5.2 | LSP decoding | Index decoding, stability enforcement, erasure fallback | 5 |
| 15 | §5.3–5.5 | Pitch/fixed codebook decode | Delay/index mapping, pitch sharpening | 5 |
| 16 | §5.6 | Gain decoding | MA prediction, erasure handling (Gain_update_erasure path) | 5 |
| 17 | §5.8 | Decoder synthesis + overflow | Verify retry with excitation >>2 path | 5 |
| 18 | §5.9 | Post-filters/post-process | Deterministic block vectors and AGC energy tracking | 5 |
| 19 | §6 | Erasure concealment/recovery | Parity/BFI interactions, gain attenuation, voiced/unvoiced paths | 5 |
| 20 | §8.1 | VAD | Feature extraction, MakeDec boundaries, smoothing stages | 7 |
| 21 | §8.2 | DTX | State machine transitions, SID cadence, stationarity checks | 8 |
| 22 | §8.3 | CNG | Seed handling, gain smoothing, excitation generation | 9 |
| 23 | §8.4 | Annex B integration | Frame type routing, noise_fg derivation, SID/no-tx behavior | 8–9 |
| 24 | §10.5 | Overflow flag | All 3 overflow check sites produce correct behavior | 1, 3, 5, 6 |
| 25 | §11.3 | API requirements | Type-safe interfaces, error cases, reset semantics | 10 |
| 26 | §11.5–11.6 | Performance/non-functional | Latency, memory, no allocation, `no_std` builds | 10 |

---

## Test Infrastructure

### Dual Test System: Rust-Native and Python External

The test suite operates at two levels that complement each other:

1. **Rust-native tests** (`cargo test`) — Unit, component, and integration tests inside the Rust crate. These test individual functions, module boundaries, and full encode/decode pipelines. They are the primary gate for Phases 1-4 and contribute to all later phases.

2. **Python external tests** (`tests/` at project root) — Black-box testing of compiled binaries. These run ITU test vectors (Tier 1), cross-validate against the C reference (Tier 2), perform spectral analysis (Tier 3), and measure intelligibility (Tier 4). They are the primary gate for Phases 5+.

The orchestrator (`tests/scripts/run_all_tiers.py`) invokes **both** systems via Tier 0 (cargo test) and Tiers 1-4 (Python scripts), providing a single command per phase.

### Crate Location

The Rust crate lives at `g729/` within this repository (a subdirectory, not a separate repo). The orchestrator defaults to looking for the crate at `<project_root>/g729/` and can be overridden with `--crate-dir`. The Python test suite in `tests/` at the project root is separate from the Rust crate's test directory.

```
g729_reference/                     -- Project root (this repo)
  g729/                             -- Rust crate root
    Cargo.toml
    src/                            -- Rust source
    tests/                          -- Rust integration tests (cargo test --test)
    benches/                        -- Criterion benchmarks
  tests/                            -- Python external test suite
    conformance/                    -- Tier 1: ITU test vector conformance
    scripts/                        -- Tiers 2-5 + report generation
    fixtures/                       -- C reference binaries, C dumps
  reference/                        -- ITU reference code and specs
  test_results/                     -- JSON results and HTML dashboard
```

### Reference Data Generation

Build an instrumented version of the ITU C code (`g729ab_v14/`) that dumps intermediate values:
1. Run `bash tests/scripts/generate_c_dumps.sh` (generates all fixtures automatically)
2. Alternatively, compile reference C with `make` in `reference/itu_reference_code/g729ab_v14/` and create custom wrapper programs
3. Record outputs as binary test fixtures in `tests/fixtures/c_dumps/`
4. Rust tests load fixtures via `include_bytes!` or `G729_FIXTURE_DIR` env var

### Rust-Native Test Directory Structure (inside `g729/`)

```
g729/tests/
  common/
    mod.rs                        -- Shared test utilities
    itu_format.rs                 -- ITU serial format parser
    pcm_format.rs                 -- Raw PCM reader
  component/
    autocorr.rs                   -- Phase 3: autocorrelation tests
    levinson.rs                   -- Phase 3: Levinson-Durbin tests
    lsp.rs                        -- Phase 3: LSP conversion tests
    bitstream.rs                  -- Phase 4: pack/unpack tests
  integration/
    decoder_conformance.rs        -- Phase 5: all decoder test vectors
    encoder_conformance.rs        -- Phase 6: all encoder test vectors
    annex_b_conformance.rs        -- Phase 7-9: all Annex B vectors
    round_trip.rs                 -- Encode -> decode verification
g729/benches/
    codec.rs                      -- Frame-level performance benchmarks
    dsp_ops.rs                    -- Micro-benchmarks for DSP arithmetic
```

Unit tests for DSP math, tables, and individual modules live inline via `#[cfg(test)] mod tests` within `g729/src/` source files — not as separate test files. This keeps tests adjacent to the code they verify.

### Python External Test Directory Structure (at project root)

```
tests/
  conformance/
    test_vectors.py               -- Tier 1: ITU test vector conformance
    metrics.py                    -- Shared gradient quality metrics
  scripts/
    run_all_tiers.py              -- Tier 5: phase-aware orchestrator
    cross_validate.py             -- Tier 2: C reference cross-validation
    spectral_analysis.py          -- Tier 3: spectral analysis
    transcription_roundtrip.py    -- Tier 4: Whisper transcription
    performance_checks.py         -- Tier P: performance/structural checks
    generate_c_dumps.sh           -- C reference intermediate dump generator
    generate_report.py            -- HTML dashboard generator
    build_c_reference.sh          -- Build C reference binaries
  fixtures/
    c_reference/                  -- C reference coder/decoder binaries
    c_dumps/                      -- Intermediate value dumps (generated)
```

### ITU Test Vector Files

**Annex A** (in `G729_Release3/g729AnnexA/test_vectors/`, uppercase extensions):
- ALGTHM, ERASURE, FIXED, LSP, OVERFLOW, PARITY, PITCH, SPEECH, TAME — each with `.IN` (PCM), `.BIT` (bitstream), `.PST` (decoded PCM)
- TEST — undocumented additional test vector (`.BIT`, `.IN`, `.pst`) referenced by `TEST.BAT` but not listed in `READMETV.txt`
- Encoder tests (have `.IN`): ALGTHM, FIXED, LSP, PITCH, SPEECH, TAME, TEST (7 pairs)
- Decoder-only tests (no `.IN`): ERASURE, OVERFLOW, PARITY (3 pairs)
- Total: 10 decoder tests, 7 encoder tests

**Annex B** (in `G729_Release3/g729AnnexB/test_vectors/` and `g729_annex_b_test_vectors/`):
- tstseq1-4: `.bin` (PCM input) + `a.bit` (encoded with Annex A) + `a.out` (decoded)
- tstseq5-6: `.bit` (decoder-only) + `a.out` (decoded with Annex A decoder)
- The `a`-suffix variants are for G.729A+B (this implementation); non-`a` variants are for base G.729+B (not used)

**File formats:**
- PCM files (`.in`, `.bin`): 16-bit signed, little-endian, 8 kHz mono
- Bitstream files (`.bit`): ITU serial format (SYNC_WORD + SIZE_WORD + N x 16-bit words per frame)
- Decoder output files (`.pst`, `.out`): Same format as PCM input (16-bit signed LE)

---

## CI and Quality Gates

1. **Fast PR gate (`.github/workflows/ci.yml`)**:
   - `bash tests/scripts/ci/pr_checks.sh fmt`
   - `bash tests/scripts/ci/pr_checks.sh clippy`
   - `bash tests/scripts/ci/pr_checks.sh unsafe-policy`
   - `bash tests/scripts/ci/pr_checks.sh feature-matrix`
   - `bash tests/scripts/ci/pr_checks.sh tests`
   - `bash tests/scripts/ci/pr_checks.sh docs`
   - `bash tests/scripts/ci/pr_checks.sh msrv`
2. **Nightly extended gate (`.github/workflows/nightly_quality.yml`)**:
   - `bash tests/scripts/ci/nightly_phase10.sh`
   - Runs Phase 10 orchestration with Tier 4 skipped:
     `python tests/scripts/run_all_tiers.py --phase 10 --skip-tier4 --no-auto-whisper-setup --limit 2`
3. **Weekly fuzz gate (`.github/workflows/fuzz.yml`)**:
   - `bash tests/scripts/ci/fuzz_weekly.sh decode_speech_frame_bytes`
   - `bash tests/scripts/ci/fuzz_weekly.sh decode_annexb_frame_typed`
   - `bash tests/scripts/ci/fuzz_weekly.sh bitstream_pack_unpack_roundtrip`
   - `bash tests/scripts/ci/fuzz_weekly.sh encode_decode_roundtrip_smoke`
4. **Tier-P policy checks** (executed by `tests/scripts/performance_checks.py`):
   - `no_std` build and feature matrix
   - `Send` and memory-size assertions
   - no-unsafe policy (`clippy -D unsafe_code`)
   - benchmark latency thresholds and fuzz smoke
   - file-length and required-layout enforcement
5. **Local single-command parity**:
   - `bash tests/scripts/ci/pr_checks.sh all` reproduces the PR gate command set
   - Additional runbooks live in `docs/QUALITY_GATES.md` and `docs/RELEASE_CHECKLIST.md`

---

## Key Risks and Mitigations

| # | Risk | Impact | Mitigation |
|---|------|--------|------------|
| 1 | **Bit-exact drift** from subtle overflow/rounding differences | Any sample deviation fails conformance | Lock math kernel first (Phase 1); test every primitive boundary; cross-validate against C with 10K random pairs per function |
| 2 | **Table transcription errors** | Incorrect codebook lookups → wrong output | Auto-generate tables from C source; CRC32 checksums in CI; automated parser verifies every literal |
| 3 | **Annex B behavior drift** (SID, seeds, frame-type transitions) | Fail Annex B test vectors | Dedicated Annex B integration tests; deterministic seed checks; verify `noise_fg` derivation exactly |
| 4 | **Hot-path regressions** from overly abstract Rust code | Miss latency targets | Profile-guided inlining; `#[inline(always)]` for math ops; benchmark CI gates; avoid trait indirection in hot path |
| 5 | **Overflow flag propagation** errors | Wrong autocorrelation scaling, wrong synthesis retry | Explicit test cases for all 3 overflow check sites (PRD §3.2.2, §3.7, §5.8.1); DspContext passed as `&mut` ensures flag visibility |
| 6 | **Maintenance burden** from large files | Hard to review, debug, modify | Strict 200 LOC target (220 hard limit); CI enforcement; split by responsibility |
| 7 | **OCTET_TX_MODE mismatch** | SID frame test failures | Explicitly support both 15-bit and 16-bit SID modes; default to OCTET_TX_MODE for test vectors and production |

---

## Estimated Totals

| Phase | Functions | Lines | Files |
|-------|-----------|-------|-------|
| 1. DSP Math | 38 | ~720 | 8 |
| 2. Tables | 0 (data) | ~1,800 | 10 |
| 3. Common DSP | 18 | ~1,500 | 13 |
| 4. Bitstream | 8 | ~350 | 3 |
| 5. Decoder | 30 | ~1,800 | 18 |
| 6. Encoder | 40 | ~3,200 | 13 |
| 7. VAD | 4 | ~450 | 4 |
| 8. DTX | 15 | ~900 | 4 |
| 9. CNG | 5 | ~450 | 4 |
| 10. API/CLI | — | ~700 | 7 |
| **Total** | **~158** | **~11,870** | **~84** |

Per-phase file count is ~84 (algorithm and data files only). Including `mod.rs` re-export files and shared infrastructure (`lib.rs`, `error.rs`), the full crate is ~99 files. Average file size across all 99 files: ~120 lines; across the 84 algorithm/data files: ~141 lines. Maximum: ~200 lines (codec/encode.rs, fixed_cb/search.rs).

---

## First Milestone Deliverable

**Milestone M1** scope (Phases 1–5):
1. Complete DSP math kernel with full primitive tests
2. All tables transcribed and verified
3. Common DSP functions bit-exact against reference
4. Bitstream pack/unpack with OCTET_TX_MODE
5. **Decoder-only Annex A bit-exact pass** (all 10 test vectors including undocumented TEST)
6. Conformance report for decoder vectors and overflow paths

This de-risks core arithmetic/state behavior before encoder and Annex B complexity are added.
