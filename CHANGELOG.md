# Changelog

All notable changes to Cardly will be documented in this file.

## [Unreleased]

### Phase 0 — Research & Planning (2026-11-18)
- Cloned upstream Anki (`ankitects/anki` @ `0302650`) into `./anki-upstream`.
- Audited `rslib`, `rslib/sync`, `proto/anki`, `ts/`, `qt/aqt` (including `mediasrv.py`), and `pylib`.
- Verified upstream local build pipeline (documented spaces-in-path Ninja workaround and disk-space cache handling).
- Authored [docs/PLAN.md](./docs/PLAN.md) with architecture decision (Server-hosted `rslib` vs WASM/OPFS), module map, Qt <-> Rust communication breakdown, exhaustive screen/feature classification, and phased roadmap.
- Initialized [docs/PARITY.md](./docs/PARITY.md) feature parity checklist.
