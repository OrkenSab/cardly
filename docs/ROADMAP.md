# Cardly Product Roadmap

This document outlines the concrete development direction, active near-term milestones, and exploratory ideas for **Cardly** — the self-hosted web app for Anki collections powered directly by upstream Rust `rslib` and Svelte 5.

---

## 🧭 Guiding Principles

1. **Strict Upstream Parity & Compatibility:**  
   Never reinvent the card scheduler, database schema, or template engine. Always leverage upstream `rslib` to maintain data compatibility with `.apkg`, `.colpkg`, and official Anki sync clients (AnkiDroid, AnkiMobile, Desktop).
2. **Privacy & True Self-Hosting:**  
   Cardly belongs on your server. Zero telemetry, zero analytics tracking, zero third-party dependencies required for core functionality.
3. **Desktop Fidelity with Modern Web Ergonomics:**  
   Keyboard-first navigation, instant response times (< 15 ms RPCs), sub-millisecond local caching, and seamless responsive design that scales from desktop multi-monitor workflows down to smartphones.
4. **Pragmatic Extensibility:**  
   Provide high-impact modern additions (multimodal AI card generation, PWA, built-in sync server) without compromising core reliability.

---

## 🗺️ Milestones Overview

```mermaid
flowchart LR
    v010["v0.1.0 (Launch)<br/>Core Parity + AI"] --> v020["v0.2.0 (Near-term)<br/>Audio Polish & Shortcuts"]
    v020 -.-> ideas["Future Ideas (Community)<br/>Add-ons, Sync Relay, WASM"]
```

---

## 📦 Active Milestones

### 🟢 v0.1.0 — Public Launch Baseline (Shipped 🎉)
*Focus: Full desktop parity, self-hosted sync server, mobile PWA, multimodal AI generation, single-command deployment.*

- [x] **Rust RPC Core:** Axum backend hosting upstream `rslib` via Protobuf binary RPC over HTTP.
- [x] **Desktop Parity UI:** Built with Svelte 5 runes (`web/`), matching desktop Anki layout, styling, and keybindings (`Space`, `1`-`4`, `Ctrl+Z`, `Ctrl+F`, `/`).
- [x] **Core Screens:** Deck List with hierarchical collapse/expand, Study/Reviewer, Add Note, Browse with advanced Anki search syntax, Stats graphs, Deck Options (including FSRS), Note Types editor, Preferences, Import/Export (`.apkg`, `.colpkg`, CSV).
- [x] **Built-in Anki Sync Server:** Full `/sync/*` and `/msync/*` endpoint support for seamless two-way synchronization with AnkiDroid, AnkiMobile, and Desktop Anki.
- [x] **Mobile PWA:** Responsive touch-friendly layout (44px touch targets), offline static asset caching via Service Worker, installable to iOS/Android home screens.
- [x] **Multimodal AI Card Generator:** Generate high-quality Basic and Cloze cards from raw text, lecture notes, textbook photos, and diagrams using Google Gemini, OpenAI/OpenRouter, or local Ollama with single-step undo (`Ctrl+Z`).
- [x] **Public Demo Mode:** Sandboxed trial mode with pre-seeded decks, 60-minute automatic reset, sliding rate-limiting, and destructive action protection.
- [x] **One-Command Deployment:** Production Docker Compose with multi-stage build, Caddy automatic Let's Encrypt TLS reverse proxy, and zero-downtime health checks.

---

### 🟡 v0.2.0 — Multimedia Polish & Shortcuts (Active Focus)
*Focus: Deep multimedia support, in-browser recording, playback controls, and starter content.*

- [ ] **In-Browser Audio Recorder:**
  - One-click microphone recording directly inside the card editor using Web Audio API.
  - Real-time audio waveform visualizer and non-destructive silence trimming.
- [ ] **Reviewer Audio Controls:**
  - Dedicated playback speed controls (`0.8x`, `1.0x`, `1.2x`, `1.5x`, `2.0x`) with hotkeys (`[` and `]`).
  - Auto-replay toggle and audio repeat hotkey (`R`).
- [ ] **Configurable Hotkey Manager:**
  - Custom keybinding remapping interface in Preferences for all review, browse, and editor actions.
  - Pre-configured presets: Standard Anki, Vim-style (`j`/`k`/`h`/`l`), Single-Handed (`Q`/`W`/`E`/`R`).
- [ ] **Curated Community Starter Decks:**
  - Integrated starter deck repository for language learners (Top 3,000 high-frequency words).
  - Standardized Kazakhstan UNT (ЕНТ) starter decks for History and Math.
- [ ] **Audio Text-to-Speech (TTS) Polish:**
  - Browser-native TTS generation (Web Speech API) for vocabulary fields with voice picker in Preferences.

---

## 💡 Future Ideas & Explorations (Under Community Evaluation)

The following items are concepts currently under exploration. They are not committed releases and will be prioritized based on community feedback and contributions in [GitHub Discussions](https://github.com/cardly-app/cardly/discussions).

### Idea: Visual Image Occlusion Web Editor
- Browser-native visual image occlusion note type editor: draw masks (rectangles, ellipses, polygons) directly on uploaded textbook diagrams.
- Toggle individual mask vs. hide-all-reveal-one during review.

### Idea: Multi-User Web Admin Console
- Instance administrator portal: manage users, view disk usage per collection, enforce storage quotas.
- Server telemetry (active sync sessions, RPC response percentiles, SQLite database health checks).

### Idea: Shared Decks & Read-Only Deck Subscriptions
- Publish public or unlisted read-only deck links.
- Subscribers pull card updates without losing personal scheduling intervals or review history.

### Idea: Automated Cloud Backup Snapshots
- Scheduled background backups to AWS S3, Cloudflare R2, Backblaze B2, or local mounted volumes.
- One-click snapshot rollback with verification checksums.

### Idea: Sandboxed Web Add-on Architecture
- Isolated Web Worker / WebAssembly execution runtime for community extensions.
- Secure hook points: Reviewer bottom bar widgets, answer buttons modifications, custom card timer sounds, automated tagging rules.

### Idea: Community Card Themes Gallery
- One-click import and preview of card styles (e.g., Notion minimalist, Obsidian Dark, Cyberpunk, Academic Classic).
- Live CSS variable inspector and real-time card preview.

### Idea: Client-Side WebAssembly Core (Serverless Offline)
- Compile `rslib` directly to WASM and execute SQLite inside browser Origin Private File System (OPFS).
- Enable standalone, serverless offline Cardly that runs entirely in any browser tab without a backend server.

### Idea: Native Desktop Shells (Tauri)
- Ultra-lightweight Tauri wrapper for macOS, Linux, and Windows (< 15 MB installer).

---

## 🗳️ How Features are Prioritized

We prioritize features according to three criteria:
1. **Core Review Integrity:** Does this improve the speed, reliability, and accuracy of memorization?
2. **Desktop Parity:** Does this fix a gap where desktop Anki behaves differently or better?
3. **Self-Hosting Simplicity:** Can it run on a \$4/month VPS with minimal RAM without complex dependencies?

---

## 💬 How to Suggest or Vote on Features

We want Cardly to be shaped by real study workflows:
- **Vote on Ideas:** Visit the [Cardly GitHub Discussions](https://github.com/cardly-app/cardly/discussions) and upvote features under **Ideas**.
- **Submit a Feature Request:** Open an issue using the [Feature Request Template](https://github.com/cardly-app/cardly/issues/new?template=feature_request.yml).
- **Contribute Code:** Check out [CONTRIBUTING.md](../CONTRIBUTING.md) and browse issues labeled [`good first issue`](https://github.com/cardly-app/cardly/labels/good%20first%20issue).
