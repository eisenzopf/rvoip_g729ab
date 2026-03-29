# Proof and Archived Result Summary

This page summarizes the archived March 6, 2026 result that serves as the canonical public proof point for this repo.

## Canonical Run

- Run id: `run_20260306T033253Z_phase10`
- Timestamp: `20260306T033253Z`
- Source artifacts:
  - [full run JSON](assets/results/run_20260306T033253Z_phase10.json)
  - [tier 1 JSON](assets/results/run_20260306T033253Z_phase10_tier1.json)
  - [tier 2 JSON](assets/results/run_20260306T033253Z_phase10_tier2.json)

These artifacts were copied from the original project workspace. They are being cited here, not regenerated here.

## Result Snapshot

| Gate | Result |
|---|---|
| Tier 0 Rust-native tests | `81 passed / 0 failed / 0 ignored` |
| Tier 1 ITU conformance | `27/27 bit-exact` |
| Tier 2 C-reference comparison | `12/12 archived comparisons exactly matched` |
| Tier P structural checks | `all checks passed` |

## Tier 1: ITU Conformance

The archived tier 1 result reports a `pass_rate` of `1.0` and `bit_exact = 27`.

That means:

- Annex A decoder vectors matched exactly
- Annex A encoder vectors matched exactly
- Annex B encoder vectors matched exactly
- Annex B decoder vectors matched exactly

Representative properties from the archived tier 1 output:

- `max_abs_error = 0`
- `rms_error = 0.0`
- `snr_db = inf`

## Tier 2: Real-Audio Comparison

The archived tier 2 JSON contains 12 comparison entries named `CRef_vs_Rust_*`. All 12 reported exact parity with the C reference in the March 6, 2026 run.

Two representative archived entries:

| Comparison | Samples | Exact match | Max abs error | RMS error | SNR |
|---|---:|---:|---:|---:|---:|
| `CRef_vs_Rust_call_000111000_agent_550e8400-e29b-41d4-a716-446655440007` | `7,197,120` | `1.0` | `0` | `0.0` | `inf` |
| `CRef_vs_Rust_call_000111001_agent_550e8400-e29b-41d4-a716-446655440006` | `7,197,120` | `1.0` | `0` | `0.0` | `inf` |

CALLHOME is credited as an evaluation corpus, but the audio itself is not redistributed here.

## Tier P: Structural and Packaging Signals

The archived full-run JSON also reports these passing structural checks:

- `no_std` build
- send bounds
- size assertions
- `unsafe` ban
- feature matrix
- benchmark capture
- fuzz smoke
- required layout
- file-length policy

The archived benchmark capture recorded:

- encode: `17.103 us`
- decode: `7.186 us`

## What This Page Is For

This page is the lightweight public proof layer for the extracted repo. It is intentionally narrower than the original internal dashboard bundle and is designed to answer the GitHub visitor's first question quickly:

Does this library actually work?

The archived March 6, 2026 result says yes.
