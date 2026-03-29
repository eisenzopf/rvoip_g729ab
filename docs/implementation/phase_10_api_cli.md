> Part of [G.729AB Implementation Plan](README.md)

### Phase 10: Public API, CLI, and Production Hardening

**Goal**: Create user-facing API, CLI tool, and production polish.

**Files to create**:
- `src/api/encoder.rs` — G729Encoder wrapping EncoderState
- `src/api/decoder.rs` — G729Decoder wrapping DecoderState
- `src/api/config.rs` — EncoderConfig, DecoderConfig
- `src/api/frame.rs` — FrameType enum, EncodeResult
- `src/lib.rs` — Crate root with public re-exports
- `src/error.rs` — CodecError enum
- `src/bin/cli.rs` — CLI binary for batch encode/decode and test vector runner

**Total**: ~700 lines

**Tasks**:
1. Implement G729Encoder::new(config), encode(), encode_frame(), reset() wrapping codec/encode
2. Implement G729Decoder::new(config), decode(), decode_frame(), decode_erasure(), reset() wrapping codec/decode
3. Implement configurable maximum consecutive erasures before muting (PRD §11.7) — `DecoderConfig::max_consecutive_erasures: Option<usize>` with a sensible default (e.g., 10 frames)
4. Implement CLI with subcommands: `encode`, `decode`, `test-vectors` (Note: PRD §11.4 specifies separate `g729-enc`/`g729-dec` binaries; unified subcommand design preferred for single-binary distribution but should be documented as a deliberate deviation)
5. Add ITU test vector runner that parses serial format and reports pass/fail
6. Add benchmark suite (criterion): per-frame encode/decode, throughput for 1000 frames
7. Add fuzz targets for decoder (random bitstreams) and encoder (random PCM)
8. **Verify `#![no_std]` compatibility**: compile without `std` feature and confirm no `std`-dependent code leaks into core path. Add CI check that `cargo build --no-default-features` succeeds.
9. Verify `Send` trait bound: `assert_impl!(G729Encoder: Send)`, `assert_impl!(G729Decoder: Send)`
10. Document public API with examples
11. Run full conformance suite: all Annex A + Annex B test vectors

**Test Plan**:

| Test | Method | Target |
|------|--------|--------|
| Encode latency | criterion benchmark | < 100 us per frame (> 100x real-time) |
| Decode latency | criterion benchmark | < 50 us per frame (> 200x real-time) |
| Memory per instance | `std::mem::size_of::<EncoderState>()` | < 8 KB encoder, < 4 KB decoder |
| No heap allocation | Run encode/decode under custom allocator that panics | Zero allocations |
| Determinism | Encode SPEECH.IN 10 times | All outputs byte-identical |
| Fuzz decoder | 1M random bitstreams | No panics, no UB |
| Fuzz encoder | 1M random PCM frames | No panics, no UB |
| All Annex A encoder vectors | 7 test pairs | Bit-exact |
| All Annex A decoder vectors | 10 test pairs | Bit-exact |
| All Annex B vectors | 10 test pairs | Bit-exact |
| `no_std` build | `cargo build --no-default-features` | Compiles without errors |
| `Send` bounds | Static assertions | G729Encoder: Send, G729Decoder: Send |
| Tandem encoding | Encode -> decode -> encode -> decode SPEECH.IN | No crashes, output degrades gracefully (PRD §13.5) |
| bcg729 interoperability | (a) Decode bcg729-encoded bitstreams with our decoder, (b) decode our encoder output with bcg729 decoder | Decoder output matches for identical bitstream input (except overflow-triggering frames where bcg729 uses saturation-only). Encoder bitstream comparison will NOT match due to different fixed codebook search algorithms (PRD §13.5) |
| Long-duration session | 10+ minutes continuous encode/decode (~60K frames) | No state accumulation drift, no panics (PRD §13.5) |

**Exit Criteria**:
1. All 27 test vectors (9+1 Annex A decoder including undocumented TEST, 6+1 Annex A encoder including TEST, 10 Annex B) pass bit-exactly
2. Performance targets met (< 100us encode, < 50us decode per frame)
3. Memory per instance verified under budget
4. Fuzz campaign clean (no panics, no UB)
5. `no_std` build succeeds
6. `Send` bounds verified

**TDD Workflow**:
1. **Write tests first**: Create `size_assertions` tests (`size_of::<EncoderState>() < 8192`, etc.), `send_bounds` tests (static assertions), CLI integration tests, and benchmark harness stubs. Set up fuzz targets for decoder and encoder.
   ```
   cargo test size_assertions send_bounds             # structural tests
   cargo build --no-default-features                  # no_std check
   cargo clippy -- -F unsafe_code                     # no unsafe
   ```
2. **Implement**: API wrappers -> config types -> error enum -> CLI binary -> benchmarks -> fuzz targets. Wire the public API to use the internal codec functions.
3. **Verify**:
   ```
   cargo test --all-features                          # all tests pass
   cargo bench --bench codec                          # latency under PRD thresholds
   python tests/scripts/performance_checks.py --crate-dir . --json  # all checks pass
   python tests/scripts/run_all_tiers.py --phase 10   # exits 0 (Gate 6)
   ```
4. **Gate 6**: `run_all_tiers.py --phase 10` reports all 27 vectors bit-exact, Tier P checks pass (no_std, Send, memory, benchmarks, fuzz smoke).

---

### Deferred: Homing Frame Detection

Homing frame detection (PRD §6.6) is **deferred** from the initial implementation. Per PRD §6.6 and the ITU Implementers' Guide, homing frames are a testing facility — neither the Annex A reference code, the Annex B reference code, nor bcg729 implement them, and no homing frame test vectors exist in Release 3. If needed later, the approach is: encode 80 zero samples from initial state to produce the DHF pattern, then detect it in the decoder to trigger state reset.

> **Note:** PRD §13.2 lists "Homing frame tests" as a formal test vector category. When homing frame detection is implemented, corresponding conformance tests should be added to the test suite.

**Implementation trigger**: Implement homing frame support when (a) a downstream consumer explicitly requires it, or (b) ITU compliance certification is sought. The implementation cost is low (~50 lines): encode 80 zeros from initial state to produce the DHF pattern, store as a `const` array, compare incoming frames against it.

> **Conformance note:** The 27 test vector count in the Conformance Checkpoints section (Gate 6) excludes homing frame tests. PRD §13.2 lists "Homing frame tests" as a formal test vector category, but no such vectors exist in Release 3.
