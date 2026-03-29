> Part of [Specification Plan](README.md)

### Batch 6: Phase 10 (API + CLI)

#### SPEC_PHASE_10_api_cli.md

**Sources:**

| Source | What it provides |
|--------|-----------------|
| [Implementation Plan Phase 10](../implementation/phase_10_api_cli.md) | API surface, CLI subcommands, performance targets, TDD workflow |
| `PRD.md` §11 | API requirements, performance, `no_std`, Send bounds |
| `tests/scripts/performance_checks.py` | Automated Tier P checks |

**Key decisions:**

- G729Encoder/G729Decoder public API surface wrapping internal codec state
- **Frame type inference design decision:** The primary `decode()` method infers frame type from bitstream length (10 bytes = Speech, 2 bytes = SID, 0 bytes = NoData). An alternative `decode_with_type()` method accepts an explicit `FrameType` parameter for transport layers that provide frame type metadata alongside the data. The ITU reference code's `read_frame()` produces both bitstream data AND a frame type field; `decode_with_type()` accommodates this pattern. Both methods produce identical results when inferred and explicit types agree
- **BFI-to-API mapping:** In the ITU reference, BFI (bad frame indicator) is detected by `read_frame()` from zero-valued bit words (PRD §4.5) and is a separate signal from frame type. The Rust API maps BFI as follows: BFI=1 with data present -> caller should use `decode_erasure()` (not `decode()` with the corrupted data, which would produce incorrect output); BFI=1 with no data -> `decode_erasure()` or `decode(&[])` (both equivalent — 0-length input triggers NoData/erasure path). `decode_with_type(data, FrameType::NoData)` is equivalent to `decode_erasure()`. The `decode_erasure()` method is the canonical way to signal frame erasure from any transport layer (RTP, ITU serial, raw). Reference: PRD §4.5 (BFI detection), PRD §6 (frame erasure concealment)
- Error types: CodecError enum (~4 variants)
- CLI subcommands: `encode`, `decode`, `test-vectors` (unified binary, documented deviation from PRD §11.4's separate binaries)
- Configurable max consecutive erasures before muting: `DecoderConfig::max_consecutive_erasures: Option<usize>`
- `DecoderConfig::post_filter: bool` (default: true) -- bypasses post-filter chain when disabled; must be propagated to `Post_Filter()` call in decoder main loop
- Benchmark thresholds: encode < 100us/frame, decode < 50us/frame (criterion). Note: these are aspirational implementation targets that significantly exceed the PRD §11.5 requirements (< 2ms encode / < 1ms decode). The PRD requirements are the conformance floor; the implementation targets are stretch goals
- Memory assertions: `size_of::<EncoderState>() < 8192`, `size_of::<DecoderState>() < 4096`
- `#![no_std]` verification: `cargo build --no-default-features` must succeed
- `Send` trait bounds: static assertions on G729Encoder and G729Decoder
- Fuzz targets: decoder (random bitstreams), encoder (random PCM) -- no panics, no UB
- bcg729 interoperability: (a) decode bcg729-encoded bitstreams, (b) decode our output with bcg729 decoder. Encoder bitstream comparison NOT expected to match (different ACELP search)
- Long-duration session test: 10+ minutes continuous encode/decode (~60K frames)

**Module file mapping:** `api/encoder.rs`, `api/decoder.rs`, `api/config.rs`, `api/frame.rs`, `lib.rs`, `error.rs`, `bin/cli.rs`

**TDD requirements:**

- Size/Send assertions, `no_std` build check, feature matrix tests
- Benchmark harness stubs (criterion)
- Fuzz target stubs (cargo-fuzz)
- Full conformance regression: all 27 test vectors (9+1 Annex A decoder including undocumented TEST, 6+1 Annex A encoder including TEST, 10 Annex B)
- Tandem encoding: encode -> decode -> encode -> decode SPEECH.IN (no crashes, graceful degradation per PRD §13.5)
- Tier mapping: All tiers (Tier 0 through Tier 4 + Tier P per Section 11 orchestrator coverage)
