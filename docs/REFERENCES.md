# References and Credits

This repo preserves the key external references and credits that shaped the project.

## Evaluation Corpus

- TalkBank CallHome English: [https://talkbank.org/ca/access/CallHome/eng.html](https://talkbank.org/ca/access/CallHome/eng.html)

The corpus is credited here as an evaluation source only. This public repo does not redistribute CALLHOME-derived audio.

## Normative Standard

- ITU-T G.729 recommendation: [https://www.itu.int/rec/T-REC-G.729](https://www.itu.int/rec/T-REC-G.729)

This was the normative target for codec behavior, together with the relevant Annex A and Annex B material referenced during the original project.

## Comparative Open-Source Reference

- `bcg729`: [https://github.com/linphone/bcg729](https://github.com/linphone/bcg729)

`bcg729` was useful as an additional implementation reference and interoperability/debugging aid, but it was not treated as the normative source for bit-exact behavior.

## Public Telephony Sample Used By The Smoke Test

- Open Speech Repository: [https://www.voiptroubleshooter.com/open_speech/](https://www.voiptroubleshooter.com/open_speech/)
- American English sample listing: [https://www.voiptroubleshooter.com/open_speech/american.html](https://www.voiptroubleshooter.com/open_speech/american.html)
- Smoke-test file: [OSR_us_000_0030_8k.wav](https://www.voiptroubleshooter.com/open_speech/american/OSR_us_000_0030_8k.wav)

The Open Speech Repository states that the material is freely available for VoIP testing and related use, with attribution to "Open Speech Repository".

## Downstream Integration Target

- `rvoip`: [https://github.com/eisenzopf/rvoip](https://github.com/eisenzopf/rvoip)

This crate is being packaged as a standalone public repo now, with the expectation that it will later be incorporated into `rvoip`.

## Planning Documents Preserved Here

- [PRD](../PRD.md)
- [Implementation Plan](../IMPLEMENTATION_PLAN.md)
- [Specification](../SPECIFICATION.md)

Those documents are included because the main process lesson from the project was that the quality of the implementation improved once the work was driven through a strong PRD and a strong specification before coding.
