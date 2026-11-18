# Changelog

All notable changes to Cardly will be documented in this file.

## [Unreleased]

### Phase 1 — Backend Server, Sync & Docker Deployment (2026-11-18)
- Built `server/` (`cardly-server`), a Rust Axum backend linking directly against upstream `anki-upstream/rslib` (zero fork).
- Implemented per-user runtime (`server/src/user_runtime.rs`) with `server: true` collection mode, shared Web/Sync `RwLock` coordination, isolated user folders (`collection.anki2`, `media/`, `media.db`, `backups/`, `tmp/`, `profile.json`, `meta.json`), and web-to-sync media mirroring.
- Implemented automatic protobuf-over-HTTP RPC dispatch (`POST /_anki/{method}`) from embedded `anki_descriptors.bin`, server-side `FrontendService` handlers, safe path-isolated upload/import/export/backups APIs, batch browser row endpoint (`/api/browser/rows`), and timezone sync (`/api/timezone`).
- Implemented username/password authentication (`/api/auth/{login,logout,me}`) using PBKDF2, HMAC-SHA256 HttpOnly cookies, and per-session `X-Cardly-Api-Token` isolation against untrusted card JS.
- Mounted upstream `rslib` Anki Sync Server protocol on `/sync/*` and `/msync/*` sharing the per-user collection and media directory.
- Served upstream `ts/` SvelteKit pages and built-in assets with exact `mediasrv.py` CSP rules, injected `/_anki/cardly-bridge.js` & `/_anki/cardly-theme.css`, and per-user media with `UNTRUSTED_MEDIA_CSP` + HTTP `Range` support.
- Added `Dockerfile`, `docker-compose.yml`, `.dockerignore`, `.env.example`, `/healthz`, and `--healthcheck`.
- Verified with 4 end-to-end Rust integration tests (`cargo test --test integration`) including real upstream `HttpSyncClient` sync interop, plus live browser verification of login, `/graphs`, and `/deck-options/1`.

### Phase 0 — Research & Planning (2026-11-18)
- Cloned upstream Anki (`ankitects/anki` @ `0302650`) into `./anki-upstream`.
- Audited `rslib`, `rslib/sync`, `proto/anki`, `ts/`, `qt/aqt` (including `mediasrv.py`), and `pylib`.
- Verified upstream local build pipeline (documented spaces-in-path Ninja workaround and disk-space cache handling).
- Authored [docs/PLAN.md](./docs/PLAN.md) with architecture decision (Server-hosted `rslib` vs WASM/OPFS), module map, Qt <-> Rust communication breakdown, exhaustive screen/feature classification, and phased roadmap.
- Initialized [docs/PARITY.md](./docs/PARITY.md) feature parity checklist.
