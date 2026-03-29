> Part of [G.729AB Implementation Plan](README.md)

### Phase 1: DSP Math Kernel

**Goal**: Implement the foundational saturating arithmetic library (`basic_op.c` + `oper_32b.c` equivalent).

**Files to create**:
- `src/dsp/types.rs` — Word16, Word32, DspContext, constants (MAX_16, MIN_16, MAX_32, MIN_32)
- `src/dsp/arith.rs` — `add`, `sub`, `mult`, `mult_r`, `negate`, `abs_s`, `saturate`
- `src/dsp/arith32.rs` — `L_add`, `L_sub`, `L_mult`, `L_mac`, `L_msu`, `L_negate`, `L_abs`, `L_sat` (NOTE: `L_sat` has no callers in the reference code — included for completeness), `L_deposit_h`, `L_deposit_l`, `extract_h`, `extract_l`, `round` (Note: `L_macNs`, `L_msuNs`, `L_add_c`, `L_sub_c`, `mac_r`, `msu_r` are defined in `basic_op.c` but **never called** anywhere in g729ab_v14 — omitted to avoid unused complexity. `L_macNs`, `L_msuNs`, `L_add_c`, `L_sub_c` are the only operations that use the `Carry` flag, so `DspContext` does not include a `carry` field. `mac_r` and `msu_r` are round-after-MAC variants with no callers.)
- `src/dsp/shift.rs` — `shl`, `shr`, `L_shl`, `L_shr`, `shr_r`, `L_shr_r`, `norm_s`, `norm_l`
- `src/dsp/div.rs` — `div_s`, `inv_sqrt`, `Log2`, `Pow2`
- `src/dsp/random.rs` — `Random()`: LCG pseudo-random generator (`seed = extract_l(L_add(L_shr(L_mult(seed, 31821), 1), 13849))`). Used by frame erasure concealment (seed_fer=21845) and CNG excitation (seed=11111). Both seeds use the same LCG formula but maintain separate state.
- `src/dsp/oper32.rs` — `L_Extract`, `L_Comp`, `Mpy_32`, `Mpy_32_16`, `Div_32`
- `src/dsp/mod.rs` — Re-exports

**Reference**: `reference/itu_reference_code/g729ab_v14/basic_op.c` (104 KB), `oper_32b.c`, `dspfunc.c` (Log2, Pow2, Inv_sqrt extended math operations)

**Total**: ~38 functions, ~720 lines

**Tasks**:
1. Define Word16/Word32 newtypes with `#[repr(transparent)]`, Copy, Clone, Debug, Eq, Ord
2. Implement DspContext with overflow/carry flag, clear/set/check methods
3. Implement 16-bit ops: add, sub, mult, negate, abs_s with saturation
4. Implement 32-bit ops: L_add, L_sub, L_mult, L_mac, L_msu with saturation
5. Implement shift ops: shl/shr with bidirectional logic, L_shl/L_shr, norm_s/norm_l
6. Implement div_s (15-iteration fractional division), inv_sqrt, Log2, Pow2
7. Implement DPF ops: L_Extract, L_Comp, Mpy_32, Mpy_32_16, Div_32
8. Implement `Random()` LCG: `seed = extract_l(L_add(L_shr(L_mult(seed, 31821), 1), 13849))` — returns new seed as Word16. Used by decoder frame erasure (seed_fer=21845) and CNG excitation (seed=11111). Reference: `UTIL.C:55-61`
9. All functions `#[inline(always)]` — these are called millions of times per second

**Test Plan**:

| Test | Input | Expected Output | Validates |
|------|-------|-----------------|-----------|
| `add` saturation positive | `add(MAX_16, 1)` | `MAX_16` (32767) | Positive overflow clamps |
| `add` saturation negative | `add(MIN_16, -1)` | `MIN_16` (-32768) | Negative overflow clamps |
| `add` cancellation | `add(MAX_16, MIN_16)` | `-1` | Normal arithmetic |
| `sub` edge | `sub(0, MIN_16)` | `MAX_16` | 0 - (-32768) saturates |
| `mult` critical | `mult(MIN_16, MIN_16)` | `MAX_16` | -32768*-32768>>15 saturates |
| `mult` fractional | `mult(16384, 16384)` | `8192` | 0.5 * 0.5 = 0.25 in Q15 |
| `negate` edge | `negate(MIN_16)` | `MAX_16` | -(-32768) saturates |
| `abs_s` edge | `abs_s(MIN_16)` | `MAX_16` | |-32768| saturates |
| `L_mult` critical | `L_mult(MIN_16, MIN_16)` | `MAX_32` | Double-width saturates |
| `L_mac` overflow | `L_mac(MAX_32, 1, 1)` | `MAX_32` | Accumulate saturates |
| `shl` bidirectional | `shl(1, -3)` | `shr(1, 3)` = 0 | Negative shift reverses |
| `shl` saturation | `shl(1, 15)` | `MAX_16` | Would exceed 16-bit |
| `shr` sign extend | `shr(MIN_16, 1)` | `-16384` | Arithmetic right shift |
| `norm_s` zero | `norm_s(0)` | `0` | Special case per ITU |
| `norm_s` positive | `norm_s(1)` | `14` | 1 << 14 = 16384 |
| `norm_l` positive | `norm_l(1)` | `30` | 1 << 30 normalized |
| `div_s` equal | `div_s(1, 1)` | `MAX_16` | Equal inputs -> max |
| `div_s` half | `div_s(1, 2)` | `16384` | 0.5 in Q15 |
| `round` midpoint | `round(0x00008000)` | `1` | 0.5 rounds up |
| `round` below | `round(0x00007FFF)` | `0` | Below 0.5 rounds down |
| `L_Extract`/`L_Comp` round-trip | `L_Comp(L_Extract(x))` | `x` (approx) | DPF precision |
| Overflow flag set | `shl(MAX_16, 1)` then check | `overflow == true` | Flag is set on saturation |
| Overflow flag clear | `clear_overflow()` then `add(1,1)` | `overflow == false` | Non-saturating op doesn't set |
| `Random` FER seed | `Random(21845)` 3 times | Matches C reference output sequence | LCG with frame erasure seed |
| `Random` CNG seed | `Random(11111)` 3 times | Matches C reference output sequence | LCG with CNG seed |
| `L_deposit_h` MAX_16 | `L_deposit_h(MAX_16)` | `0x7FFF0000` (2147418112) | Upper deposit of max positive |
| `L_deposit_h` MIN_16 | `L_deposit_h(MIN_16)` | `0x80000000` (-2147483648) | Upper deposit of min negative |
| `L_deposit_l` MAX_16 | `L_deposit_l(MAX_16)` | `32767` | Lower deposit sign-extends positive |
| `L_deposit_l` MIN_16 | `L_deposit_l(MIN_16)` | `-32768` | Lower deposit sign-extends negative |

**Additional**: Property-based tests with `proptest` for commutativity (`add(a,b) == add(b,a)`), identity (`add(a,0) == a`), range preservation (result always in `[MIN_16, MAX_16]`).

**Cross-validation**: Build a C test harness that calls each `basic_op.c` function with 10,000 random input pairs and records outputs. Compare Rust results sample-by-sample.

**Exit Criteria**:
1. 100% pass on primitive boundary suite (all rows in test table above)
2. No panics for randomized fuzz input to arithmetic API
3. Property tests pass for commutativity, identity, and range preservation
4. Cross-validation against C `basic_op.c` outputs matches for 10,000 random pairs per function

**TDD Workflow**:
1. **Write tests first**: Create `src/dsp/arith.rs` with `#[cfg(test)] mod tests` containing all 29 boundary tests from the test plan table above, plus proptest strategies for commutativity/identity/range. All tests fail initially (functions not yet implemented).
   ```
   cargo test --lib dsp::arith -- --color=never    # expect: 0 passed
   ```
2. **Implement incrementally**: Implement one function at a time (e.g., `add` first), run its tests, then proceed to the next. Order: types.rs -> arith.rs -> arith32.rs -> shift.rs -> div.rs -> oper32.rs -> random.rs.
3. **Verify**:
   ```
   cargo test --lib dsp::                           # all 38+ boundary tests pass
   cargo test --lib dsp:: -- proptest               # property tests pass
   python tests/scripts/run_all_tiers.py --phase 1  # exits 0 (Gate 1)
   ```
4. **Gate 1**: `run_all_tiers.py --phase 1` reports `PASS` for Gate 1.
