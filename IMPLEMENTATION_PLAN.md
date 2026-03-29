# G.729AB Codec Implementation Plan (Pure Rust) — Final

> Preserved planning artifact from the original workspace. In this extracted
> repo the crate lives at repository root as `rvoip_g729ab`; references below
> to `g729/` describe the original build layout used during development.

> **This document has been reorganized.** The content has been split into focused files under [`docs/implementation/`](docs/implementation/README.md).

## Document Index

| Document | Contents |
|----------|----------|
| [Overview](docs/implementation/README.md) | Context, architecture, module structure, design decisions |
| [Phase 1: DSP Math Kernel](docs/implementation/phase_01_dsp_math.md) | Saturating arithmetic library (~38 functions, ~720 lines) |
| [Phase 2: Codec Constants and Tables](docs/implementation/phase_02_tables.md) | ROM data transcription (~40 const arrays, ~1,800 lines) |
| [Phase 3: Common DSP Functions](docs/implementation/phase_03_common_dsp.md) | Shared signal processing (~18 functions, ~1,500 lines) |
| [Phase 4: Bitstream Pack/Unpack](docs/implementation/phase_04_bitstream.md) | Bitstream serialization (~8 functions, ~350 lines) |
| [Phase 5: Decoder (Core G.729A)](docs/implementation/phase_05_decoder.md) | Complete decoder with post-processing (~30 functions, ~1,800 lines) |
| [Phase 6: Encoder (Core G.729A)](docs/implementation/phase_06_encoder.md) | Complete encoder with ACELP search (~40 functions, ~3,200 lines) |
| [Phase 7: Annex B -- VAD](docs/implementation/phase_07_vad.md) | Voice Activity Detection (~4 functions, ~450 lines) |
| [Phase 8: Annex B -- DTX](docs/implementation/phase_08_dtx.md) | Discontinuous Transmission (~15 functions, ~900 lines) |
| [Phase 9: Annex B -- CNG](docs/implementation/phase_09_cng.md) | Comfort Noise Generation (~5 functions, ~450 lines) |
| [Phase 10: Public API, CLI, Hardening](docs/implementation/phase_10_api_cli.md) | Production API and tooling (~700 lines) |
| [Appendices](docs/implementation/appendices.md) | Dependency graph, conformance gates, test infrastructure, CI, risks, totals |
