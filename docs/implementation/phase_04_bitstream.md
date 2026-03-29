> Part of [G.729AB Implementation Plan](README.md)

### Phase 4: Bitstream Pack/Unpack

**Goal**: Implement bitstream serialization for speech frames, SID frames, and ITU test format.

**Files to create**:
- `src/bitstream/pack.rs` — `prm2bits_ld8k()`: 11 parameters -> 80-bit frame
- `src/bitstream/unpack.rs` — `bits2prm_ld8k()`: 80-bit frame -> 11 parameters; `int2bin`/`bin2int`
- `src/bitstream/itu_serial.rs` — ITU serial format reader/writer [cfg(itu_serial)]

**Reference**: `bits.c`

**Total**: ~8 functions, ~350 lines

**Tasks**:
1. Implement `prm2bits_ld8k` — pack parameters per `bitsno[11] = {8,10,8,1,13,4,7,5,13,4,7}`
2. Implement `bits2prm_ld8k` — unpack 80 bits to 11 parameters
3. Implement ITU serial format parser: SYNC_WORD (0x6B21) + SIZE_WORD + bit array (BIT_0=0x007F, BIT_1=0x0081)
4. Implement BFI detection: any zero-valued bit word (0x0000) -> frame erasure (PRD §4.5)
5. Implement SID frame pack/unpack: `bitsno2[4] = {1,5,4,5}` = 15 bits
6. **Implement OCTET_TX_MODE handling**: SID frames are 16 bits (15 data + 1 padding zero bit) when OCTET_TX_MODE is active (default for RTP/SIP and test vector conformance). RATE_SID_OCTET=16 vs RATE_SID=15. The Annex B test vectors use OCTET_TX_MODE — without this, SID frame tests will fail (PRD §4.4)
7. Implement frame type detection from SIZE_WORD: 80→speech(ftyp=1), 15/16→SID(ftyp=2), 0→no-tx(ftyp=0)

**Test Plan**:

| Test | Input | Expected | Validates |
|------|-------|----------|-----------|
| Pack/unpack round-trip | Known parameter vector | `bits2prm(prm2bits(prm)) == prm` | Lossless round-trip |
| Bit allocation | Sum of bitsno | 80 bits total | Correct allocation |
| SPEECH.BIT frame 0 parse | First frame from ITU test vector | Parameters match reference | ITU format parsing |
| BFI detection | Frame with one zero-valued bit word | `bfi == true` | Erasure detection |
| SID frame parse (OCTET_TX_MODE) | 16-bit SID frame | Correct 4 parameters | SID format with padding |
| SID frame parse (non-OCTET) | 15-bit SID frame | Correct 4 parameters | SID format without padding |
| Untransmitted frame | SIZE_WORD=0 | `frame_type == Untransmitted` | No-data detection |
| Frame type routing | Various SIZE_WORD values | Correct ftyp assignment | Frame type discriminator |

**Exit Criteria**:
1. Binary compatibility against ITU known frame samples
2. SID frame round-trip works in both OCTET_TX_MODE and non-OCTET mode
3. All frame types (speech, SID, no-tx) correctly detected and parsed

**TDD Workflow**:
1. **Write tests first**: Create round-trip tests (`prm2bits(bits2prm(x)) == x`), ITU serial format parsing tests using the first frame of SPEECH.BIT, BFI detection tests, SID frame tests, and frame type routing tests.
   ```
   cargo test --lib bitstream::                     # expect: 0 passed
   cargo test --test component bitstream            # expect: 0 passed
   ```
2. **Implement**: pack.rs -> unpack.rs -> itu_serial.rs. Tests become green as each is completed.
3. **Verify**:
   ```
   cargo test --lib bitstream::                     # all unit tests pass
   cargo test --features itu_serial                 # ITU format tests pass
   python tests/scripts/run_all_tiers.py --phase 4  # exits 0
   ```
4. **Gate**: All Tier 0 cargo tests pass including bitstream round-trip and ITU format parsing.
