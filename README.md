# rvoip_g729ab

`rvoip_g729ab` is a working pure-Rust G.729AB library intended for standalone use now and later integration into [`rvoip`](https://github.com/eisenzopf/rvoip).

This repo is also a compact case study in AI-assisted systems work: the project got materially better once the work was forced through `PRD -> implementation plan -> specification -> implementation`, and the final result came from a bakeoff across Claude Code, OpenAI Codex, and Google Antigravity rather than a single uninterrupted agent run.

## Result Snapshot

- Canonical proof run: March 6, 2026 (`20260306T033253Z`)
- Tier 0 Rust-native tests: `81 passed / 0 failed`
- Tier 1 ITU conformance: `27/27` vectors bit-exact
- Tier 2 real-audio comparison: the archived March 6 run reports exact C-reference parity for all 12 recorded comparison sets; the first two archived examples each matched over `7,197,120` samples with `max_abs_error = 0`, `rms_error = 0.0`, and `SNR = inf`
- Tier P structural checks: `no_std` build, feature matrix, send bounds, size assertions, `unsafe` ban, required layout checks, and benchmark capture all passed in the archived result

Details: [Proof and Archived Result Summary](docs/PROOF.md)

## Why This Project Mattered

Working Rust implementations of G.729AB are rare, the algorithmic surface is large, and bit-exact conformance requires more than getting the high-level DSP ideas mostly right. The useful outcome here is not just that the library now exists. It is that a disciplined planning stack plus the right agent mix was enough to get a systems-level codec over the line.

The repo therefore preserves both:

- the library itself, for developers who want to use or integrate it
- the planning and retrospective documents that explain how the project got from failed attempts in 2025 to a working archived result in 2026

## Why It Worked This Time

The main process lesson was that implementation quality improved once the project was forced through explicit intermediate artifacts:

1. `PRD.md` defined the codec scope, compliance target, and constraints.
2. `IMPLEMENTATION_PLAN.md` turned that into phased architecture and test strategy.
3. `SPECIFICATION.md` and the deeper spec docs tightened the function-level details until the plan and the codec references aligned.
4. Only then did the implementation become stable enough to converge quickly.

Those artifacts are preserved here as first-class documents because they were part of the solution, not just project overhead.

## Agent Bakeoff Summary

This repo captures one concrete project retrospective, not a general benchmark:

- Claude Code was strongest at research, PRD generation, specification refinement, and the final detailed algorithmic polish.
- OpenAI Codex was strongest at orchestration, end-to-end project momentum, and the earliest full implementation pass.
- Google Antigravity was useful for early browsing and reference gathering, but did not remain competitive through the full implementation cycle.

More detail: [Agent Bakeoff Retrospective](docs/AGENT_BAKEOFF.md)

## Developer Quick Start

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

Basic packaging-only verification:

```bash
cargo check --lib --bins --examples --features "std,annex_b,itu_serial"
```

See also: [examples/basic_encode_decode.rs](examples/basic_encode_decode.rs)

## Public End-to-End Smoke Test

For a real download-and-run check, the repo includes an opt-in integration test that:

1. downloads a public 8 kHz telephony WAV from the Open Speech Repository
2. extracts the raw PCM payload
3. runs `g729-cli encode --no-vad`
4. runs `g729-cli decode`
5. verifies the decoded PCM is frame-aligned and non-silent

Run it with:

```bash
cargo test --test public_sample_roundtrip -- --ignored --nocapture
```

Notes:

- This test is `ignored` by default because it downloads a public sample over the network.
- The sample source is `OSR_us_000_0030_8k.wav` from the Open Speech Repository.
- The test uses `curl` when available and falls back to `python3` for the download step.
- The Open Speech Repository allows use of these files for VoIP testing and asks that the source be identified.

## Key Documents

- [PRD](PRD.md)
- [Implementation Plan](IMPLEMENTATION_PLAN.md)
- [Specification](SPECIFICATION.md)
- [Proof and Archived Result Summary](docs/PROOF.md)
- [Agent Bakeoff Retrospective](docs/AGENT_BAKEOFF.md)
- [References and Credits](docs/REFERENCES.md)

## References and Credits

- TalkBank CallHome English corpus: [talkbank.org/ca/access/CallHome/eng.html](https://talkbank.org/ca/access/CallHome/eng.html)
- ITU-T G.729 recommendation: [itu.int/rec/T-REC-G.729](https://www.itu.int/rec/T-REC-G.729)
- `bcg729` open-source implementation: [github.com/linphone/bcg729](https://github.com/linphone/bcg729)
- Planned downstream integration target: [github.com/eisenzopf/rvoip](https://github.com/eisenzopf/rvoip)
- Open Speech Repository sample used by the public smoke test: [voiptroubleshooter.com/open_speech/american.html](https://www.voiptroubleshooter.com/open_speech/american.html)

CALLHOME is credited here as an evaluation corpus only. This extracted public repo does not redistribute CALLHOME-derived audio.
