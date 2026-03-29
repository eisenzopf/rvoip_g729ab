# Specification Plan — G.729AB Rust Implementation

> Historical note: the detailed batch docs below were written against the
> original `g729/` sub-crate workspace. In this extracted public repo the crate
> now lives at repository root as `rvoip_g729ab`.

> This document has been split from the original monolithic `SPECIFICATION_PLAN.md`. Per-batch content outlines are in individual files linked below. Errata registries are in [errata.md](errata.md).

## Batch Index

| Batch | Phases | File |
|-------|--------|------|
| B1 | Phase 1 (DSP Math) + Phase 2 (Tables) | [batch_01_dsp_math_tables.md](batch_01_dsp_math_tables.md) |
| B2 | Phase 3 (Common DSP) + Phase 4 (Bitstream) | [batch_02_common_dsp_bitstream.md](batch_02_common_dsp_bitstream.md) |
| B3 | Phase 5 (Decoder) | [batch_03_decoder.md](batch_03_decoder.md) |
| B4 | Phase 6 (Encoder) | [batch_04_encoder.md](batch_04_encoder.md) |
| B5 | Phase 7 (VAD) + Phase 8 (DTX) + Phase 9 (CNG) | [batch_05_vad_dtx_cng.md](batch_05_vad_dtx_cng.md) |
| B6 | Phase 10 (API + CLI) | [batch_06_api_cli.md](batch_06_api_cli.md) |

See also: [Errata](errata.md) (PRD errata registry E1-E29 + ITU specification errata SE1-SE7)

---

## 1. Purpose

The implementation plan ([Implementation Plan](../implementation/README.md)) covers 10 phases, ~99 files, ~158 functions, and ~11,870 lines of Rust. Together with `PRD.md`, it specifies **what** to build, **how** to structure it, and **how** to test it. The implementation plan also resolves key design decisions (newtypes, DspContext, state struct layouts) and documents several PRD errata discovered during analysis.

The specification documents serve a **dual role** in the project's Reference-Driven Development (RDD) workflow:

1. **Bridge the remaining detail gap** — function-level algorithm pseudocode, exact Q-format annotations, field-by-field state struct definitions, and C-to-Rust mappings that make the translation deterministic and auditable.
2. **Drive Test-Driven Development** — each spec's test plan produces the Rust test harnesses that are written *before* implementation code, with tests mapping to the multi-tier test infrastructure already built in `tests/`.

**What the specs fill in:**

| Gap | Example |
|-----|---------|
| Algorithm pseudocode (step-by-step, Rust-idiomatic) | 3-20 lines per function, annotated with DSP calls |
| Complete state struct field definitions | Every field with type, Q-format, dimensions, init value |
| C-to-Rust mapping | Which C function/variable maps to which Rust item |
| Q-format annotations | Fixed-point format metadata for every I/O |
| TDD test plan per function | Exact inputs, expected outputs, tier classification, Rust test file location |
| C reference fixture requirements | Which intermediate dumps are needed for component-level cross-validation |
| PRD errata | Corrections to PRD discovered during implementation plan analysis |

**What is already resolved** (by [Implementation Plan](../implementation/README.md) and `PRD.md`):

- Exact Rust signatures for all functions (DspContext threading model decided)
- Module structure (~99 files with hierarchical layout)
- Word16/Word32 newtypes and DspContext design
- State struct memory budgets (~2.4 KB encoder, ~1.1 KB decoder)
- Test infrastructure (multi-tier Python suite, Rust-native test layout, C dump generation pipeline)
- Conformance gate definitions and TDD workflows per phase

Without these specs, each function would require independent analysis of the C reference source — error-prone and unreviewable. The specs make the translation deterministic and auditable, while also producing the test harnesses that enforce correctness from the first line of implementation code.

---

## 2. Three-Pass RDD/TDD Approach

The project follows a **three-pass, test-first incremental delivery** model:

1. **Pass 1 — Spec documents** (markdown): One `SPEC_PHASE_NN_<name>.md` per implementation phase. Written for human review before any code is generated. Each spec includes a TDD test plan that maps every function to specific test cases, tier classifications, and Rust test file locations.

2. **Pass 2 — Test harnesses + skeleton code** (`.rs` files): After spec approval, generate **both**:
   - **Rust test stubs** (written first): Unit tests in `#[cfg(test)] mod tests` within source files, component tests in `g729/tests/component/`, integration/conformance tests in `g729/tests/integration/`. All marked `#[test] #[ignore]`.
   - **Skeleton source files**: Function signatures with `todo!()` bodies, complete type definitions, exact initial values.
   - Verification: `cargo check` and `cargo test --no-run` must pass; `cargo test` reports 0 passed, N ignored.

3. **Pass 3 — TDD implementation**: Fill in `todo!()` bodies incrementally. Un-ignore tests one function at a time. Each function is considered done when its tests go green. Run multi-tier verification at phase completion.

4. **Repeat**: Advance to the next batch following the dependency graph.

**Parallel pipelines:**

- Spec writing for batch N+1 can proceed **in parallel** with implementation of batch N.
- The Python external test suite (`tests/conformance/`, `tests/scripts/`) already exists and does not need to be generated per-batch — it activates automatically as the Rust crate produces working binaries.
- C reference fixture generation (`bash tests/scripts/generate_c_dumps.sh`) should run before Pass 2 for batches that require cross-validation against intermediate C values (Batches 1-3).

---

## 3. Batch Schedule and Dependency Graph

| Batch | Spec Documents | Phases | Depends On |
|-------|---------------|--------|------------|
| **B1** | `SPEC_PHASE_01_dsp_math.md`, `SPEC_PHASE_02_tables.md` | 1, 2 | -- (root) |
| **B2** | `SPEC_PHASE_03_common_dsp.md`, `SPEC_PHASE_04_bitstream.md` | 3, 4 | B1 approved |
| **B3** | `SPEC_PHASE_05_decoder.md` | 5 | B2 approved |
| **B4** | `SPEC_PHASE_06_encoder.md` | 6 | B3 approved |
| **B5** | `SPEC_PHASE_07_vad.md`, `SPEC_PHASE_08_dtx.md`, `SPEC_PHASE_09_cng.md` | 7, 8, 9 | B4 approved |
| **B6** | `SPEC_PHASE_10_api_cli.md` | 10 | B5 approved |

**Rationale:**

- **B1** is the critical-path root -- DSP math primitives and lookup tables have no dependencies; everything else depends on them. Phase 1 and Phase 2 are partially parallelizable (tables need Word16/Word32 types from Phase 1).
- **B2** groups common DSP routines (LP analysis, filters) and bitstream handling, both of which depend on B1's math and tables. Phase 3 and Phase 4 are independent once B1 is done.
- **B3** (decoder) depends on B2's common DSP functions. Decoder is implemented before encoder because it is simpler and independently testable with all 10 ITU decoder test vectors (including the undocumented TEST vector).
- **B4** (encoder) depends on B3 -- encoder reuses decoder components (LSP decode for verification, gain prediction, etc.). All 7 encoder test vectors (including TEST) plus an encode-decode round-trip test.
- **B5** groups three related Annex B features (VAD, DTX, CNG). Phase 7 (VAD, encoder-side) and Phase 9 (CNG, decoder-side) are partially parallelizable. All 10 Annex B test vectors (4 encoder + 6 decoder) form the combined gate.
- **B6** is the final API/CLI integration layer wrapping everything, plus production hardening (fuzz, benchmarks, `no_std`).

**Dependency graph (with parallelizable pairs):**

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

All spec documents are written to: `specs/` (project root).

---

## 4. Spec Document Template

Every `SPEC_PHASE_NN_<name>.md` must contain these sections:

### Section 1: Overview

- Purpose, scope, estimated size (functions, lines, files)
- Dependencies on prior phases
- C reference files used (with brief descriptions)
- Which conformance gate(s) this phase contributes to
- **TDD tier mapping**: Which test tiers apply (Tier 0 = cargo test, Tier 1 = ITU vectors, Tier 2 = C cross-validation, etc.) and which Rust test modules this phase maps to (e.g., `g729/tests/integration/decoder_conformance.rs`)

### Section 2: C-to-Rust Mapping Table

| C File | C Function / Variable | Rust Module | Rust Function / Field | Notes |
|--------|----------------------|-------------|----------------------|-------|
| `basic_op.c` | `add()` | `dsp::arith` | `add()` | Saturating 16-bit |
| ... | ... | ... | ... | ... |

**Cross-cutting pattern — `Init_*` functions**: All C `Init_*` functions (`Init_Coder_ld8a`, `Init_Decod_ld8a`, `Init_Pre_Process`, `Init_Post_Process`, `Init_Post_Filter`, `Init_Cod_cng`, `Init_Dec_cng`, `Init_exc_err`, `vad_init`, `Lsp_encw_reset`, `Lsp_decw_reset`) map to `new()` constructors or `Default` trait implementations on the corresponding Rust state structs. No standalone `init_*` functions are needed in Rust. Each spec document should list the `Init_*` function as "eliminated — initial values in `StateStruct::new()`" in the C-to-Rust mapping table.

### Section 3: Type Definitions and State Structs

- Complete struct definitions with every field
- Each field documented with: **name, type, Q-format, array dimensions, C source, initial value**
- `new()` / `Default` implementations with exact values from PRD §2.2.1

### Section 4: Function Specifications

For each function, a subsection containing:

- **Signature**: `fn name(dsp: &mut DspContext, ...) -> ReturnType`
- **C reference**: `file.c:startline-endline`
- **Purpose**: One-line description
- **Q-formats**: Input/output fixed-point formats
- **Algorithm**: Step-by-step pseudocode (3-20 lines), Rust-idiomatic, key operations annotated with the DSP function calls they use
- **DspContext usage**: Does it read/write/clear the overflow flag?
- **Edge cases**: Saturation boundaries, special inputs (zero, MIN_16, etc.)
- **Caller context**: Which higher-level functions call this?
- **PRD errata**: Any corrections to PRD found during analysis (reference Section 11 of this document)

### Section 5: Test Specifications

For each test case:

- **Test name and description**
- **Tier classification**: Tier 0 (unit/cargo test), Tier 1 (ITU vector), Tier 2 (C cross-validation), etc.
- **Rust test location**: Exact file path (e.g., `g729/src/dsp/arith.rs` `#[cfg(test)]` module, or `g729/tests/integration/decoder_conformance.rs`)
- **Inputs**: Exact values or fixture file references
- **Expected outputs**: Exact values or reference file
- **Fixture requirements**: C reference dumps needed, ITU test vector files, how to load (`include_bytes!` or `G729_FIXTURE_DIR`)

Test categories per phase:

- **Boundary tests**: Exact inputs and expected outputs for edge cases (saturation, zero, MIN_16, etc.)
- **Component tests**: Cross-validation against C reference intermediate dumps from `tests/fixtures/c_dumps/`
- **Conformance tests**: Bit-exact comparison against ITU test vectors (Annex A and/or Annex B)
- **Property-based tests**: Proptest strategies for algebraic properties (commutativity, identity, range preservation)
- **TDD ordering**: Which tests to write first to enable incremental implementation

### Section 6: Constants and Tables Used

- Which tables from Phase 2 are referenced
- Any phase-local constants with values and Q-formats

### Section 7: TDD Workflow

Phase-specific TDD steps:

- **Prerequisite steps**: C fixture generation, test vector file verification
- **Test-first ordering**: Which test files and test functions to create before implementation, in what dependency order
- **Incremental implementation order**: Which functions to implement first (e.g., types.rs -> arith.rs -> arith32.rs -> ...)
- **Verification commands**: Exact `cargo test` and `python tests/scripts/run_all_tiers.py --phase N` invocations
- **Gate criteria**: What must pass before the phase is considered complete

### Section 8: C Reference Fixtures

- What C intermediate dumps are needed for this phase (e.g., `autocorr_frame{0..9}.bin`)
- How to generate them: `bash tests/scripts/generate_c_dumps.sh`
- Fixture file format (binary layout, endianness, field order)
- How Rust tests load them: `include_bytes!` or `G729_FIXTURE_DIR` environment variable
- Fallback behavior: component tests that require missing fixtures should be `#[ignore]`d with a message pointing to the generation script

### Section 9: PRD Errata

Any corrections to `PRD.md` discovered during analysis for this phase. Each erratum references:

- The PRD section and claim being corrected
- The authoritative source (C reference code file and line numbers)
- The correct behavior
- Cross-reference to the relevant entry in Section 11 of this document (PRD Errata Registry)

---

## 6. Skeleton Code and Test Harness Generation Rules

After a batch's spec documents are approved, **both** skeleton `.rs` files and test harnesses are generated into the project source tree. Test harnesses are written first -- they define what correctness means before any implementation exists.

### What test harnesses contain

- **Unit test stubs** in `#[cfg(test)] mod tests` within each source file, with `#[test] #[ignore]` on every test. Each test corresponds to an entry in the spec's Test Specifications section.
- **Component test files** in `g729/tests/component/` for cross-validation against C reference dumps. Tests load fixtures via `include_bytes!` or `G729_FIXTURE_DIR`.
- **Integration test files** in `g729/tests/integration/` for end-to-end conformance (ITU test vectors, round-trip).
- **Proptest strategies** in unit test modules where property-based testing is specified.
- **Fixture references**: tests that need C reference dumps are `#[ignore]`d with a message pointing to `tests/scripts/generate_c_dumps.sh` if fixtures are absent.

### Rust edition

Skeleton code targets `edition = "2024"` (Rust 1.85+, stable since February 2025). CI must pin a minimum Rust version (MSRV) per [Implementation Plan](../implementation/README.md) and PRD §11.6 (stable toolchain only). Edition-2024 idioms (e.g., `gen` keyword reservation, unsafe-in-extern defaults) apply to all generated code.

### What skeleton source files contain

- **Module declarations** (`mod.rs` files with `pub mod` items)
- **Type definitions** (structs, enums) with all fields
- **`new()` / `Default` implementations** with exact initial values
- **Function signatures** matching the spec exactly
- **Function bodies**: `todo!()` marker (compiles but panics at runtime)
- **`use` imports** for dependencies from prior phases
- **`#[inline(always)]`** annotations where specified in the spec
- **`#[cfg(feature = "annex_b")]`** gates on Annex B items
- **`#[cfg(feature = "itu_serial")]`** gates on ITU serial format items (Phase 4 `bitstream/itu_serial.rs`)

### What is NOT generated

- Algorithm implementations (those are `todo!()`)
- Comments explaining the algorithm (those are in the spec doc)
- Benchmark harnesses (added in Phase 10)
- Generated table data (table values are filled during Phase 2 implementation)
- Python external tests (already exist in `tests/` at project root)

### Dual test system note

Skeleton generation produces **Rust-native tests only** (inside the `g729/` crate). The Python external test suite (`tests/conformance/`, `tests/scripts/`) already exists at the project root and activates automatically when the Rust crate produces working binaries. No Python test code is generated per-batch.

### Verification criteria per skeleton batch

| Criterion | How verified |
|-----------|-------------|
| Compiles | `cargo check` passes |
| Module structure matches plan | Diff file list against [Implementation Plan](../implementation/README.md) ~99-file module tree |
| Tests compile | `cargo test --no-run` succeeds |
| All tests ignored | `cargo test` reports 0 passed, N ignored |
| Test count matches spec | Number of `#[test]` functions matches spec's test plan |
| Feature gates correct | `cargo check --no-default-features`, `cargo check` (default: annex_b), `cargo check --features itu_serial`, `cargo check --features annex_b,itu_serial`, and `cargo check --all-features` all pass |
| Fixture references correct | Component tests reference correct fixture paths |

---

## 7. Workflow

```
┌──────────────────────────────────────────────────────────────────┐
│  Batch N                                                         │
│                                                                  │
│  0. Generate C reference fixtures (if needed for this batch)     │
│     bash tests/scripts/generate_c_dumps.sh                      │
│                                                                  │
│  1. Write spec docs for batch N phases                           │
│  2. User reviews spec docs                                       │
│  3. User approves (or requests revisions)                        │
│                                                                  │
│  4a. Generate test harnesses from approved specs (tests first)   │
│  4b. Generate skeleton .rs files from approved specs             │
│                                                                  │
│  5a. Verify: cargo check, cargo test --no-run                   │
│  5b. Verify: cargo test reports 0 passed, N ignored              │
│  5c. Verify: test count matches spec, module structure matches   │
│                                                                  │
│  6. TDD implementation:                                          │
│     - Un-ignore tests one function at a time                     │
│     - Implement until test goes green                            │
│     - Repeat for all functions in dependency order               │
│                                                                  │
│  7. Run multi-tier verification:                                 │
│     cargo test --all-features                                    │
│     python tests/scripts/run_all_tiers.py --phase N              │
│                                                                  │
│  8. Conformance gate passes AND no regressions on prior gates    │
│     → batch N complete                                           │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │ In parallel: write spec docs for batch N+1                 │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

**Rules:**

- Spec writing for batch N+1 may begin while batch N is being implemented.
- Implementation of batch N may **not** begin until batch N's specs are approved.
- Implementation of batch N may **not** begin until batch N-1 is complete. For batches with a conformance gate, "complete" means the gate passes. For batches without a formal conformance gate number (B2: Phases 3-4), "complete" means all Tier 0 `cargo test` passes and `run_all_tiers.py --phase N` exits 0 for each phase in the batch (Phase 3 additionally requires bit-exact match against C reference dumps for 10 frames).
- Each spec revision cycle should be at most one round of feedback (specs are detailed enough to minimize ambiguity).
- C reference fixtures must be generated before Pass 2 for batches that require cross-validation (Batches 1-3).
- The Python test orchestrator (`run_all_tiers.py`) is the authoritative gate runner -- it invokes both Rust-native tests (Tier 0) and external validation (Tiers 1-4, Tier P).
- Regression checking is mandatory: when gate N passes, all gates 1 through N-1 must still pass.

---

## 8. Verification Checklists

### Spec Document Checklist

Before a spec is considered ready for review:

- [ ] Every function listed in [Implementation Plan](../implementation/README.md) for this phase is covered
- [ ] Every function has a complete Rust signature with types decided
- [ ] Every state struct field has: type, Q-format, array dimensions, C source mapping, initial value
- [ ] Algorithm pseudocode is traceable to specific C `file.c:line` ranges
- [ ] DspContext usage (read/write/clear overflow flag) is explicitly documented per function
- [ ] Test cases cover all boundary conditions from the implementation plan's test table
- [ ] C-to-Rust mapping table is complete (no C functions/variables left unmapped)
- [ ] Constants and tables used are cross-referenced against Phase 2 spec
- [ ] Conformance gate contributions are identified
- [ ] TDD workflow documented with test-first ordering and incremental implementation order
- [ ] C reference fixture requirements specified (what dumps are needed, how to generate)
- [ ] PRD errata documented where applicable (with cross-reference to Section 11)
- [ ] Test tier mapping provided (Tier 0-4 classification for each test)
- [ ] Rust test file locations specified (source module `#[cfg(test)]`, `g729/tests/component/`, or `g729/tests/integration/`)

### Test Harness Checklist

Before test harnesses are considered ready (generated alongside skeletons):

- [ ] All tests from spec's Test Specifications section exist as `#[test]` functions
- [ ] Unit tests in `#[cfg(test)]` modules within source files
- [ ] Component tests in `g729/tests/component/` for C reference dump cross-validation
- [ ] Integration tests in `g729/tests/integration/` for ITU vector conformance
- [ ] All tests marked `#[ignore]` initially
- [ ] Fixture files present or tests `#[ignore]`d with message pointing to `tests/scripts/generate_c_dumps.sh`
- [ ] Proptest strategies defined where specified in the spec
- [ ] Test count matches spec's test plan

### Skeleton Code Checklist

Before skeletons are considered ready for implementation:

- [ ] `cargo check` passes with default features
- [ ] `cargo check --no-default-features` passes
- [ ] `cargo check --all-features` passes
- [ ] Module file tree matches [Implementation Plan](../implementation/README.md) ~97-file module structure
- [ ] `cargo test --no-run` succeeds (tests compile)
- [ ] `cargo test` reports 0 passed, N ignored (all tests are `#[ignore]`)
- [ ] Every `todo!()` body corresponds to a function specification in the approved spec
- [ ] Struct field counts and types match the spec exactly
- [ ] Initial values in `new()` / `Default` match PRD §2.2.1

### Implementation Checklist (per phase)

Before a phase is considered complete:

- [ ] All `todo!()` markers removed
- [ ] All `#[ignore]` attributes removed from tests
- [ ] `cargo test` -- all unit and component tests pass
- [ ] Conformance gate tests pass (bit-exact match against ITU test vectors where applicable)
- [ ] `cargo clippy -- -D warnings` clean
- [ ] No file exceeds 200 LOC target (220 hard limit; 180-220 triggers CI warning) -- excluding generated table data
- [ ] No `unsafe` blocks
- [ ] `python tests/scripts/run_all_tiers.py --phase N` exits 0
- [ ] No regressions on previous phases' gates (all prior gates still pass)
- [ ] C cross-validation passes (Tier 2) where applicable
- [ ] Performance smoke check passes (Tier P) for Phase 10

---

## 9. Phase Summary Reference

Quick reference mapping phases to scope, sourced from [Implementation Plan](../implementation/README.md):

| Phase | Name | Functions | Lines | Files | Gate |
|-------|------|-----------|-------|-------|------|
| 1 | DSP Math Kernel | 38 | ~720 | 8 | Gate 1: all 38 ops pass boundary + reference tests |
| 2 | Codec Constants & Tables | 0 (data) | ~1,800 | 10 | Gate 2: all tables verified via checksums |
| 3 | Common DSP Functions | 18 | ~1,500 | 13 | -- (verified by Phase 5 gate) |
| 4 | Bitstream Pack/Unpack | 8 | ~350 | 3 | -- (verified by Phase 5 gate) |
| 5 | Decoder (Core G.729A) | 30 | ~1,800 | 18 | Gate 3: all 10 decoder test vectors bit-exact |
| 6 | Encoder (Core G.729A) | 40 | ~3,200 | 13 | Gate 4: all 7 encoder test vectors bit-exact + round-trip |
| 7 | Annex B -- VAD | 4 | ~450 | 4 | -- (verified by Phase 9 gate) |
| 8 | Annex B -- DTX | 15 | ~900 | 4 | -- (verified by Phase 9 gate) |
| 9 | Annex B — CNG | 5 | ~450 | 3 new + 1 shared (P8's `sid.rs`) | Gate 5: all 10 Annex B vectors bit-exact |
| 10 | Public API, CLI, Hardening | -- | ~700 | 7 | Gate 6: perf, fuzz, no_std, Send |
| **Total** | | **~158** | **~11,870** | **~84** | |

Note: File count is ~84 algorithm/data files. Including `mod.rs` re-export files and shared infrastructure (`lib.rs`, `error.rs`), the full crate is ~99 files.

---

## 10. Conformance Gates (from [Implementation Plan](../implementation/README.md))

| Gate | After Phase | Requirement | Orchestrator Command |
|------|-------------|-------------|---------------------|
| **Gate 1** | Phase 1 | All 38 DSP functions pass boundary + reference tests | `run_all_tiers.py --phase 1` |
| **Gate 2** | Phase 2 | All tables verified against C source (automated checksums) | `run_all_tiers.py --phase 2` |
| **Gate 3** | Phase 5 | All 10 Annex A decoder test vectors bit-exact | `run_all_tiers.py --phase 5` |
| **Gate 4** | Phase 6 | All 7 Annex A encoder test vectors bit-exact + round-trip | `run_all_tiers.py --phase 6` |
| **Gate 5** | Phase 9 | All 10 Annex B test vectors bit-exact | `run_all_tiers.py --phase 9` |
| **Gate 6** | Phase 10 | Performance targets met, fuzz clean, `no_std`, `Send`, memory budget | `run_all_tiers.py --phase 10` |

Total test vectors: 27 (10 Annex A decoder + 7 Annex A encoder + 10 Annex B). Homing frame tests are deferred (see below).

**Homing frame deferral:** Homing frame detection (PRD §6.6) is deferred from the initial implementation. Neither the Annex A reference code, the Annex B reference code, nor bcg729 implement it, and no test vectors exist in Release 3. Implementation cost is low (~50 lines) when needed: encode 80 zeros from initial state to produce the DHF pattern, store as `const`, detect in decoder to trigger state reset. Implementation trigger: downstream consumer requirement or ITU compliance certification.

---

## 11. Test Infrastructure Integration

The project uses a **dual test system** that bridges Rust-native testing and external Python validation. Spec documents drive the Rust-native side; the Python side already exists.

### Dual Test System Architecture

1. **Rust-native tests** (`cargo test`) -- Unit, component, and integration tests inside the `g729/` Rust crate. Generated from spec documents during Pass 2 (test harnesses). These are the primary gate for Phases 1-4 and contribute to all later phases.

2. **Python external tests** (`tests/` at project root) -- Black-box testing of compiled binaries. These already exist and do not need to be generated per-batch. They activate as the Rust crate produces working binaries.

### Tier System

| Tier | Name | What it validates | Tool |
|------|------|-------------------|------|
| **Tier 0** | Rust-native | Unit, component, integration tests | `cargo test --all-features` |
| **Tier 1** | ITU conformance | Bit-exact match against all 27 ITU test vectors | `tests/conformance/test_vectors.py` |
| **Tier 2** | C cross-validation | Rust vs C reference encoder/decoder on real audio | `tests/scripts/cross_validate.py` |
| **Tier 3** | Spectral analysis | Log-spectral distance, segmental SNR, formant tracking | `tests/scripts/spectral_analysis.py` |
| **Tier 4** | Transcription | Whisper-based intelligibility (WER/CER) | `tests/scripts/transcription_roundtrip.py` |
| **Tier P** | Performance | Latency, memory, no_std, Send, fuzz, unsafe check | `tests/scripts/performance_checks.py` |

### Orchestrator

`python tests/scripts/run_all_tiers.py --phase N` is the single command that runs all applicable tiers for a given phase:

- Phases 1-2: Tier 0 only (no binaries to test externally)
- Phases 3-4: Tier 0 only (unit tests + Rust-native component tests loading C reference dump fixtures)
- Phase 5: Tier 0 + Tier 1 (decoder vectors) + Tier 2 (C cross-validation)
- Phase 6: Tier 0 + Tier 1 (encoder + decoder vectors) + Tier 2 + Tier 3
- Phases 7-9: Tier 0 + Tier 1 (all vectors including Annex B)
- Phase 10: All tiers including Tier P

The orchestrator produces JSON results in `test_results/` and an HTML dashboard via `tests/scripts/generate_report.py`.

### C Reference Fixture Pipeline

For component-level cross-validation (Phases 1-3), C reference intermediate dumps are needed:

1. Build C reference binaries: `bash tests/scripts/build_c_reference.sh`
2. Generate intermediate dumps: `bash tests/scripts/generate_c_dumps.sh`
3. Fixtures are stored in `tests/fixtures/c_dumps/`
4. Rust tests load fixtures via `include_bytes!` or the `G729_FIXTURE_DIR` environment variable
5. If fixtures are absent, dependent tests are `#[ignore]`d with a message pointing to the generation script

### How Specs Connect to Test Infrastructure

- **Spec Test Specifications (Section 5)** -> Rust test stubs generated in Pass 2, mapped to specific `g729/src/` and `g729/tests/` files
- **Spec Conformance Gates** -> Python Tier 1 tests in `tests/conformance/test_vectors.py` (already exist)
- **Spec C-to-Rust Mapping** -> Python Tier 2 cross-validation in `tests/scripts/cross_validate.py` (already exists)
- **Spec TDD Workflow (Section 7)** -> Implementation order and verification commands per phase

### Directory Structure Reference

```
g729_reference/                     -- Project root
  g729/                             -- Rust crate root
    Cargo.toml
    src/                            -- Rust source (~99 files)
    tests/                          -- Rust integration tests (cargo test --test)
      common/                       -- Shared test utilities
        mod.rs                      -- Utility re-exports
        itu_format.rs               -- ITU serial format parser (needed from Phase 4+)
        pcm_format.rs               -- Raw PCM reader (needed from Phase 5+)
      component/                    -- Phase 3-4: C dump cross-validation
        autocorr.rs                 -- Phase 3: autocorrelation vs C dumps
        levinson.rs                 -- Phase 3: Levinson-Durbin vs C dumps
        lsp.rs                      -- Phase 3: LSP conversion vs C dumps
        bitstream.rs                -- Phase 4: pack/unpack vs C dumps
      integration/                  -- Phase 5+: ITU vector conformance
    benches/                        -- Criterion benchmarks
  tests/                            -- Python external test suite
    conformance/                    -- Tier 1: ITU test vector conformance
    scripts/                        -- Tiers 2-5 + orchestrator + report generation
    fixtures/                       -- C reference binaries, C dumps
  reference/                        -- ITU reference code and specs
  specs/                            -- Spec documents (SPEC_PHASE_*.md)
  test_results/                     -- JSON results and HTML dashboard
```
