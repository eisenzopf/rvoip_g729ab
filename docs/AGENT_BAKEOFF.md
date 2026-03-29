# Agent Bakeoff Retrospective

This document records one concrete project retrospective from building a Rust G.729AB codec. It is not intended as a general benchmark of coding agents.

## Goal

The project was effectively a bakeoff between:

- Claude Code
- OpenAI Codex
- Google Antigravity

The evaluation criterion was not "which model writes the prettiest snippet". It was whether the agent could help produce a conformant, maintainable, documented codec project with enough rigor to survive a difficult DSP implementation.

## Why The Process Changed

The major process lesson was that jumping straight into implementation was not enough. The project only became reliable once the work was staged as:

1. product requirements
2. reference-material consolidation
3. implementation planning
4. specification refinement
5. implementation
6. comparative code-quality review

The PRD and the specification were not documentation side quests. They were part of what made the final implementation work.

## Phase-By-Phase Observations

## 1. Product Requirements

- Prompt shape: research the ITU G.729 specification first, then produce a detailed PRD, then move to a detailed implementation specification before implementation.
- Claude Code did the most research and produced the most comprehensive PRD.
- Codex and Antigravity produced PRDs with similar high-level structure but materially less detail.
- Antigravity was notably good at browser-driven retrieval of ITU files.
- Codex had more trouble fetching the same reference material directly.

## 2. Reference Material and PRD Consolidation

- Claude was used to consolidate the PRDs and reference material from the three source workspaces into one combined `PRD.md` and a shared reference directory.
- This consolidation step mattered because later implementation quality improved once the project had one authoritative planning document instead of three drifting ones.

## 3. Implementation Plan

- Claude produced the most complete implementation plan.
- Codex was second and still useful.
- Antigravity's plan was much shorter and less complete.
- Claude's output was strongest on phased testing strategy and completeness.

## 4. Specification Refinement

- The specification pass was the longest planning loop.
- Claude was iterated roughly 40 times before the specification was considered aligned with the PRD, implementation plan, G.729AB documents, and reference code.
- This was a major learning: the specification work materially reduced ambiguity before implementation.

## 5. Implementation

- Codex finished first and required the least intervention to get to an end-to-end implementation pass.
- Claude finished second but needed roughly a day of follow-up fixes.
- Antigravity had false starts and did not stay in the race long enough to evaluate on equal footing.

## 6. Code Quality Review

- Codex did a strong job orchestrating the project and produced a compliant Rust encoder/decoder early.
- Codex's weakness was plan adherence: it drifted toward monolithic files and did not follow the intended file structure closely enough.
- Claude followed the file-structure instructions better and was stronger at detailed algorithmic organization.
- Claude was weaker at integration and end-to-end fixture/output generation and required more ongoing guidance.
- After deeper review, Codex's code quality still looked slightly better in terms of tidy, idiomatic Rust, but both implementations required meaningful rework.

## Final Takeaways

- Claude Code was strongest at research, PRD creation, specification refinement, and the final detailed algorithmic polish.
- OpenAI Codex was strongest at orchestration and getting to an early full implementation quickly.
- Google Antigravity was useful at the very beginning, especially around browsing/reference retrieval, but did not hold up as a full-project implementation partner.
- The most important project lesson was not that one model dominated every phase. It was that the successful path combined:
  - strong up-front planning artifacts
  - the right agent for the right phase
  - iterative correction against authoritative references

## How This Repo Uses That Lesson

This extracted public repo keeps the planning stack visible:

- [PRD](../PRD.md)
- [Implementation Plan](../IMPLEMENTATION_PLAN.md)
- [Specification](../SPECIFICATION.md)

Those documents are preserved because they were part of what made the final March 6, 2026 archived result possible.
