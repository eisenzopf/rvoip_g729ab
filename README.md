# rvoip_g729ab

`rvoip_g729ab` is a working pure-Rust G.729AB library intended for standalone use now and later integration into [`rvoip`](https://github.com/eisenzopf/rvoip).

This repo is also a case study in AI-assisted systems work. The central question was:

Can any of the 2026 coding agents, specifically Claude Code, OpenAI Codex, or Google Antigravity, actually produce a working Rust G.729AB encoder/decoder in a way that was not viable in 2025?

The answer from this project was yes, but not by going straight from prompt to implementation. The successful path was:

`PRD -> implementation plan -> specification -> repeated self-validation -> implementation`

## Summary

- The repo now contains a working pure-Rust G.729AB library.
- The canonical archived proof run is March 6, 2026 (`20260306T033253Z`).
- A direct PRD-to-code attempt was not enough. All of the coding agents failed to produce an encoder/decoder that actually worked according to the spec when asked to jump straight from PRD to implementation.
- The useful lesson was not that one agent did everything best. It was that different agents helped in different phases, and the planning/specification loop mattered more than raw code generation speed.
- The strongest technique was forcing the project through detailed planning artifacts and repeatedly self-validating those artifacts before trusting implementation.

## What We Learned

- A hard codec project like G.729AB was still not something to treat as a one-shot code-generation problem.
- The PRD mattered. It was iterated roughly 50 times with self-validation before coverage looked complete enough to trust.
- Even a good PRD was not enough by itself. When the agents were asked to implement directly from the PRD, none of them produced a working encoder/decoder that conformed to the spec.
- That failure is what drove the next step: create a detailed implementation plan before continuing.
- The specification mattered even more. It went through nearly 100 refinement and self-validation passes before it was considered to have effectively complete coverage.
- Claude Code was strongest at research, PRD quality, specification refinement, and the final algorithmic polish.
- OpenAI Codex was strongest at orchestration, early project momentum, and getting to an end-to-end implementation quickly.
- Google Antigravity was useful early for browsing/reference gathering, but did not remain competitive through the full implementation cycle.
- The winning pattern was not “pick the best model once”. It was “use the right agent at the right phase, with planning artifacts that can be checked and refined”.

## Why This Repo Exists

This repo preserves both of the things that ended up mattering:

- the library itself, for developers who want to use or integrate it
- the planning and retrospective material that explains why this version of the project succeeded

If you want the deeper story, start here:

- [PRD](PRD.md)
- [Implementation Plan](IMPLEMENTATION_PLAN.md)
- [Specification](SPECIFICATION.md)
- [Proof and Archived Result Summary](docs/PROOF.md)
- [Agent Bakeoff Retrospective](docs/AGENT_BAKEOFF.md)
- [References and Credits](docs/REFERENCES.md)

## Use The Library

Add the crate to `Cargo.toml`:

```toml
[dependencies]
rvoip_g729ab = { path = "../rvoip_g729ab" }
```

Encode and decode one frame:

```rust
use rvoip_g729ab::{
    FrameType, G729Config, G729Decoder, G729Encoder, FRAME_SAMPLES, SPEECH_FRAME_BYTES,
};

let cfg = G729Config { annex_b: false };
let mut encoder = G729Encoder::new(cfg);
let mut decoder = G729Decoder::new(cfg);

let pcm_in = [0i16; FRAME_SAMPLES];
let mut bitstream = [0u8; SPEECH_FRAME_BYTES];
let frame_type = encoder.encode(&pcm_in, &mut bitstream);
assert_eq!(frame_type, FrameType::Speech);

let mut pcm_out = [0i16; FRAME_SAMPLES];
decoder.decode(&bitstream, &mut pcm_out);
```

See also: [examples/basic_encode_decode.rs](examples/basic_encode_decode.rs)

## Test The Repo

Fast packaging/build sanity check:

```bash
cargo check --lib --bins --examples --features "std,annex_b,itu_serial"
```

Public end-to-end smoke test:

```bash
cargo test --test public_sample_roundtrip -- --ignored --nocapture
```

That smoke test downloads a public 8 kHz telephony WAV from the Open Speech Repository, extracts raw PCM, runs `g729-cli encode --no-vad`, runs `g729-cli decode`, and verifies that the decoded PCM is frame-aligned and non-silent.

Notes:

- The smoke test is `ignored` by default because it depends on network access.
- The sample source is `OSR_us_000_0030_8k.wav` from the Open Speech Repository.
- The test uses `curl` when available and falls back to `python3` for the download step.

## Archived Result Snapshot

- Canonical proof run: March 6, 2026 (`20260306T033253Z`)
- Tier 0 Rust-native tests: `81 passed / 0 failed`
- Tier 1 ITU conformance: `27/27` vectors bit-exact
- Tier 2 real-audio comparison: the archived March 6 run reports exact C-reference parity for all 12 recorded comparison sets
- Tier P structural checks: `no_std` build, feature matrix, send bounds, size assertions, `unsafe` ban, required layout checks, and benchmark capture all passed in the archived result

More detail: [Proof and Archived Result Summary](docs/PROOF.md)

## References and Credits

- TalkBank CallHome English corpus: [talkbank.org/ca/access/CallHome/eng.html](https://talkbank.org/ca/access/CallHome/eng.html)
- ITU-T G.729 recommendation: [itu.int/rec/T-REC-G.729](https://www.itu.int/rec/T-REC-G.729)
- `bcg729` open-source implementation: [github.com/linphone/bcg729](https://github.com/linphone/bcg729)
- Planned downstream integration target: [github.com/eisenzopf/rvoip](https://github.com/eisenzopf/rvoip)
- Open Speech Repository sample used by the public smoke test: [voiptroubleshooter.com/open_speech/american.html](https://www.voiptroubleshooter.com/open_speech/american.html)

CALLHOME is credited here as an evaluation corpus only. This extracted public repo does not redistribute CALLHOME-derived audio.
