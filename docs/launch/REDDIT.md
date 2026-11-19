# Reddit Launch Kit & Subreddit Guidelines

This document outlines the self-promotion policies and tailored launch posts for `r/Anki`, `r/selfhosted`, and `r/opensource`.

---

## 📋 Subreddit Community Rules & Audit

### 1. `r/Anki` (~180k members)
- **Self-Promotion & Commercial Rules**: Strictly forbids spam and commercial marketing. Free, open-source software and tools that genuinely help Anki users are welcomed if:
  - You clearly state you are the author.
  - The tool respects Anki trademarks and doesn't impersonate official software.
  - You are active in the comments answering technical and troubleshooting questions.
- **Audience Expectations**: Heavy power users (medical students, polyglots, law students). They care deeply about:
  - FSRS algorithm accuracy (zero interval drift).
  - Sync compatibility with AnkiDroid (Android) and AnkiMobile (iOS).
  - Not losing card progress, tags, or media during import/sync.

### 2. `r/selfhosted` (~450k members)
- **Self-Promotion Rules**: Rule 3 permits self-promotion if it is open-source, self-hostable, and transparently disclosed with a `[Self-Promotion]` or `[FOSS]` tag. Must include a direct link to the repository (GitHub/GitLab) and a Docker Compose file.
- **Audience Expectations**: Homelab enthusiasts, sysadmins, and privacy advocates. They care about:
  - Single-container or minimal Docker Compose setup.
  - Low RAM/CPU footprint.
  - No phone-home telemetry, analytics, or external cloud lock-in.
  - Automated SSL/TLS reverse proxy (Caddy / Traefik / Nginx).
  - Storage persistence and clean backup strategies.

### 3. `r/opensource` (~170k members)
- **Self-Promotion Rules**: Open-source releases must clearly state the license (OSI or FSF approved). AGPL-3.0 projects are celebrated for preserving software freedom.
- **Audience Expectations**: Developers, FOSS advocates, and engineers. They care about:
  - Code architecture (Rust, Svelte 5, Axum, SQLite).
  - Zero-fork upstream library reuse without fragmentation.
  - Clean licensing compliance and contributor attribution.

---

## 📝 Tailored Post 1: `r/Anki`

**Title**: I built Cardly: a faithful, self-hosted web version of Anki powered directly by the official Rust core (`rslib`), with FSRS and a built-in sync server

**Post Body**:

Hi everyone!

Like many of you, my partner is a heavy Anki user (med student studying 500+ cards a day). We frequently ran into situations where Desktop Anki wasn't an option—studying on shared hospital computers, iPads, or Chromebooks in the library. AnkiWeb is great as an emergency backup, but it doesn't give you the full 3-pane card browser, tag organization, or the modern FSRS weight optimizer.

Every third-party web flashcard app we tried had major drawbacks: they rewrite the algorithm in JavaScript (which drifts from true Anki intervals), lack nested Cloze support, or lock you into a proprietary cloud.

To solve this, I spent the last few months building **Cardly** (https://cardly.app): an open-source, self-hosted web application that looks and behaves like Desktop Anki, because it runs on official Anki code.

### How It Works Under the Hood:
- **No Algorithm Approximation**: Cardly links directly against Anki's official Rust core (`rslib`). Every review interval, stability calculation, and database transaction is handled by Anki's own code.
- **Full FSRS v4 & v5**: Includes the in-browser weights optimizer and retention simulator.
- **Built-in Anki Sync Server**: Cardly implements Anki's official `/sync/*` and `/msync/*` protocols. In **AnkiDroid** or **AnkiMobile**, you can simply point your custom sync server to Cardly and sync your cards and media directly!
- **Desktop Parity**: 3-pane virtualized card browser, MathJax 3, Cloze deletions, audio/video playback, card templates editor, and all desktop keyboard shortcuts (`Space`/`Enter`, `1`–`4`, `d`, `a`, `b`, `t`, `y`, `Ctrl+Z`, flags `1`–`7`).
- **AI Flashcard Generator**: Optional built-in generator that turns lecture notes or textbook photos (via Google Gemini multimodal OCR, OpenAI, or offline local Ollama) into atomic flashcards.

### Free & Open Source (AGPL-3.0)
Cardly is open-source under the **GNU AGPLv3** license, matching upstream Anki. A huge thank you to Damien Elmes and the Anki contributors for building such an incredible open ecosystem.

- **GitHub Repository**: https://github.com/cardly-app/cardly
- **Live Demo**: https://cardly.app
- **Docker Compose Quickstart**:
  ```bash
  git clone --recurse-submodules https://github.com/cardly-app/cardly.git
  cd cardly && cp .env.example .env
  docker compose up -d
  ```

I'd love to hear your thoughts, test cases, and feature suggestions!

---

## 📝 Tailored Post 2: `r/selfhosted`

**Title**: [Self-Promotion] Cardly – A self-hosted, lightweight web Anki stack with built-in sync server for your phone, FSRS, and single-command Docker Compose

**Post Body**:

Hey r/selfhosted!

If you use spaced repetition or flashcards for language learning, coding interview prep, or exams, you probably love Anki but wish there was a clean, self-hosted web client that doesn't compromise on features.

I built **Cardly** (https://cardly.app), a self-hosted web application that brings the full Anki Desktop experience to your browser and serves as a private sync server for your mobile devices.

### What makes it special for self-hosters:
- **Single-command Docker Compose**: Spins up a single lightweight Rust container (`cardly-server`) consuming under 60MB of RAM at idle.
- **Built-in Sync Server**: No need to run a separate `anki-sync-server`. Cardly handles both the web UI and mobile sync (`/sync/*`, `/msync/*`) using the same underlying SQLite database. Point AnkiDroid (Android) or AnkiMobile (iOS) at your server IP and sync without third-party cloud dependencies.
- **Direct Rust Core (`rslib`)**: We link directly against Anki's official Rust core. No node.js backend or database shims; database locks and schemas are managed natively by upstream code.
- **Full FSRS Scheduler**: Native multi-core FSRS v4/v5 weights optimization via Rayon.
- **Zero Telemetry**: No Google Analytics, no phone-home pings, no tracking cookies. Session authentication uses PBKDF2 password derivation and HttpOnly HMAC-SHA256 cookies.
- **Automated Backup Pruning**: Automatically snapshots your collection before syncs, keeping the 50 most recent `.colpkg` snapshots to prevent disk bloat.
- **Production Ready**: Includes `docker-compose.prod.yml` with automated Caddy SSL reverse proxy (Let's Encrypt / ZeroSSL, HTTP/3, and automatic HTTPS).

### Quickstart:
```bash
git clone --recurse-submodules https://github.com/cardly-app/cardly.git
cd cardly
cp .env.example .env
docker compose up -d
```
Visit `http://localhost:8080`, log in with `alice:supersecretpassword`, and your instance is ready.

- **Source Code**: https://github.com/cardly-app/cardly
- **Live Demo**: https://cardly.app
- **Deployment Guide (VPS / Fly.io / Railway / Caddy)**: https://github.com/cardly-app/cardly/blob/main/docs/DEPLOY.md

Feedback and PRs are warmly welcome!

---

## 📝 Tailored Post 3: `r/opensource`

**Title**: Cardly: Bringing Anki Desktop to the Web without forking its Rust core (AGPL-3.0, Axum, Svelte 5)

**Post Body**:

Hi r/opensource,

Anki is one of the most successful copyleft (AGPL) desktop applications in the world, with over 18 years of continuous community development. However, bringing its full power to the browser has always been an architectural challenge.

Most web flashcard projects take the fork-and-rewrite path: they reimplement the spaced repetition formulas in TypeScript, parse SQLite using custom parsers, and eventually diverge from upstream compatibility.

With **Cardly** (https://cardly.app), we took a zero-fork architectural approach:

1. **Native `rslib` Linking**:
   Our Axum backend server links `anki = { path = "../anki-upstream/rslib" }` directly. When Anki releases upstream improvements or new FSRS scheduler updates, Cardly receives them immediately via standard dependency updates.

2. **Automated Protobuf RPC Bridge**:
   Rather than manually writing hundreds of API endpoints, we inspect `anki_descriptors.bin` via `anki_proto_gen` to dynamically route every camelCase Protobuf RPC over HTTP (`POST /_anki/*`) directly into `rslib`'s `Backend::run_service_method`.

3. **Shared-State Sync Server**:
   Cardly initializes `Backend::new(tr, /*server:*/ true)`. In this mode, web edits write positive server USNs, allowing Anki's native HTTP sync server protocol (`/sync/*` and `/msync/*`) to operate concurrently on the same collection. AnkiDroid and AnkiMobile sync natively with the server without database locking conflicts.

4. **Svelte 5 Runes Shell**:
   The frontend is an installable PWA written in Svelte 5 (`$state`, `$derived`, `$effect`) using Desktop Anki's exact CSS tokens (`anki-tokens.css`).

5. **Copyleft Harmony (AGPL-3.0)**:
   Cardly is licensed under the GNU Affero General Public License v3 or later. All source code, build scripts, and documentation are completely open and credit the upstream Anki contributors.

- **GitHub Repository**: https://github.com/cardly-app/cardly
- **Architecture Plan**: https://github.com/cardly-app/cardly/blob/main/docs/PLAN.md
- **Live Demo**: https://cardly.app

Let me know what you think of the architecture!
