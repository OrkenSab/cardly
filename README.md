# Cardly

> **Cardly** is a faithful, self-hosted web app for Anki collections powered directly by Anki's official Rust core (`rslib`), featuring complete FSRS v4/v5 scheduling, desktop-grade card browsing, mobile PWA touch support, and a built-in Anki sync server for AnkiDroid and AnkiMobile.

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](./LICENSE)
[![Anki Upstream](https://img.shields.io/badge/Anki_Upstream-ankitects%2Fanki-orange.svg)](https://github.com/ankitects/anki)
[![Tests](https://img.shields.io/badge/Tests-Rust%20%26%20Playwright%20Passing-brightgreen.svg)]()

[English](README.md) | [Русский](README.ru.md) | [Қазақша](README.kk.md)

---

## 🎬 Live Interactive Demo

<p align="center">
  <img src="./docs/demo/cardly-demo.gif" alt="Cardly Showcase Demo" width="800" />
  <br />
  <em>Complete workflow showcase: Login &rarr; Rich Text / Cloze Card Creation &rarr; Reviewer &rarr; 3-Pane Browser &rarr; Statistics</em>
  <br />
  <a href="./docs/demo/cardly-demo.mp4">▶ Download HD Video (MP4, 320 KB)</a>
</p>

---

## 📸 Interface Gallery

Every screen in Cardly has been verified for visual and functional fidelity against official Anki Desktop across light and dark themes, as well as on mobile viewports:

### Desktop Interface
| Screen | Light Mode | Dark Mode |
| :--- | :--- | :--- |
| **Deck List (Main Screen)** | [01-deck-list-light.png](./docs/screenshots/01-deck-list-light.png) | [11-deck-list-dark.png](./docs/screenshots/11-deck-list-dark.png) |
| **Deck Overview** | [02-deck-overview-light.png](./docs/screenshots/02-deck-overview-light.png) | — |
| **Reviewer: Question** | [03-reviewer-question-light.png](./docs/screenshots/03-reviewer-question-light.png) | — |
| **Reviewer: Answer & Rating Buttons** | [04-reviewer-answer-light.png](./docs/screenshots/04-reviewer-answer-light.png) | [13-reviewer-dark.png](./docs/screenshots/13-reviewer-dark.png) |
| **Add Cards Dialog** | [05-add-cards-light.png](./docs/screenshots/05-add-cards-light.png) | — |
| **Card Browser (3-Pane Resizable)** | [06-browser-light.png](./docs/screenshots/06-browser-light.png) | [12-browser-dark.png](./docs/screenshots/12-browser-dark.png) |
| **Statistics & Visual Graphs** | [07-stats-light.png](./docs/screenshots/07-stats-light.png) | — |
| **Deck Options & FSRS Parameters** | [08-deck-options-fsrs-light.png](./docs/screenshots/08-deck-options-fsrs-light.png) | — |
| **Card Templates Split Editor** | [09-card-templates-light.png](./docs/screenshots/09-card-templates-light.png) | — |
| **Preferences Modal** | [10-preferences-light.png](./docs/screenshots/10-preferences-light.png) | — |

### Mobile & Progressive Web App (PWA 390×844)
| Mobile Screen | Screenshot Link |
| :--- | :--- |
| **Mobile Deck Tree** | [14-mobile-deck-list-light.png](./docs/screenshots/14-mobile-deck-list-light.png) |
| **Mobile Deck Overview** | [15-mobile-deck-overview-light.png](./docs/screenshots/15-mobile-deck-overview-light.png) |
| **Mobile Reviewer: Question State** | [16-mobile-reviewer-question-light.png](./docs/screenshots/16-mobile-reviewer-question-light.png) |
| **Mobile Reviewer: Answer State (44px touch buttons)** | [17-mobile-reviewer-answer-light.png](./docs/screenshots/17-mobile-reviewer-answer-light.png) |
| **Mobile Reviewer: Dark Theme** | [18-mobile-reviewer-dark.png](./docs/screenshots/18-mobile-reviewer-dark.png) |
| **Mobile Card Browser (Sidebar Drawer)** | [19-mobile-browser-light.png](./docs/screenshots/19-mobile-browser-light.png) |

---

## 🎯 Feature Parity Status

Cardly adheres to strict functional honesty. For full technical details and test verifications, see [docs/PARITY.md](./docs/PARITY.md).

### Implemented & Verified (`done`)
- **Authentic Anki Core**: Directly links upstream `rslib` (Rust). No algorithm rewrites or approximation.
- **Modern Scheduling & FSRS**: Complete support for default Anki 23+ scheduler, FSRS v4, and FSRS v5 with in-browser weight optimization and retention simulation.
- **Card Content & Media**: Nested cloze deletions (`{{c1::...}}`), MathJax 3 (client-side rendering), HTML5 audio and video playback, `{{tts ...}}` Text-to-Speech via Web Speech API, typed answers (`{{type:...}}` and `{{type:cloze:...}}`) with character diff highlights, and Image Occlusion.
- **Desktop Keyboard Shortcuts**: Identical key bindings: `Space`/`Enter`, `1`–`4`, `d` (Decks), `a` (Add), `b` (Browse), `t` (Stats), `y` (Sync), `u` / `Ctrl+Z` (Undo), flags `1`–`7`, mark `*`, bury `-`/`=`, suspend `@`/`!`, delete `Ctrl+Del`, reset `Ctrl+Alt+N`, card info `i`/`Ctrl+Alt+I`.
- **Card Browser**: 3-pane virtualized layout with Anki search syntax (`deck:`, `tag:`, `is:due`, `prop:reps>5`, `flag:1`), search history, saved searches, sidebar tree, note editor, and bulk actions (change deck, tag/untag, suspend, delete, find & replace, find duplicates, export).
- **Deck Options & Templates**: Reuses upstream SvelteKit deck options interface; full Card Templates editor with live Front/Back HTML/CSS split preview.
- **Import & Export**: Drag-and-drop import for `.apkg`, `.colpkg`, and CSV/TSV text files; full export to `.apkg`, `.colpkg`, and CSV.
- **Built-in Anki Sync Server**: Full implementation of Anki's sync endpoints (`/sync/*` and `/msync/*`), allowing AnkiDroid (Android) and AnkiMobile (iOS) to sync directly with your self-hosted Cardly instance.
- **Multi-User Isolation**: Isolated SQLite collections and media directories per account; PBKDF2 password hashing; HMAC-SHA256 HttpOnly session cookies; `X-Cardly-Api-Token` security boundary preventing card JavaScript from mutating user collections.
- **Mobile Responsive PWA**: Standalone installable PWA with web app manifest, service worker offline shell, safe area insets, and ergonomic 44px touch targets.
- **Automated Backups**: Automatic rolling backups before sync/mutations, pruned to the 50 most recent snapshots.

### Intentionally Skipped (`skipped`)
- **PyQt / Python Add-ons**: Python desktop add-ons require a local Python and Qt environment and cannot safely run in a web browser sandbox.
- **Multi-Profile Switcher**: Replaced by standard multi-user web accounts with individual login credentials.
- **Desktop Audio Drivers (`mpv`)**: Replaced by standard Web Audio, HTML5 media elements, and the Web Speech API.

---

## ✨ AI Card Generator (Text & Photo OCR)

Cardly includes a built-in AI flashcard generator that converts lecture notes, articles, and textbook photos into atomic, high-retention spaced repetition cards:

- **Multimodal Support**: Paste raw text or upload photos of book pages and diagrams for OCR extraction.
- **Card Formats**: Generates atomic **Basic (Q&A)** or **Cloze Deletion (`{{c1::...}}`)** cards.
- **Provider Choice**:
  - **Google Gemini** (`gemini-1.5-flash`, `gemini-1.5-pro` via Google AI Studio).
  - **OpenAI & OpenRouter** (`gpt-4o-mini`, `deepseek-chat`).
  - **Local Ollama** (`llama3.2`, `mistral`, `llava`) for offline, private study without cloud dependencies.
- **Review & Batch Add**: Interactive preview lets you edit prompts, answers, and tags before adding them directly into your selected deck.

Shortcut: **`Ctrl+Shift+G`** or click **✨ AI** in the top toolbar. Full setup details in [docs/AI_GENERATOR.md](./docs/AI_GENERATOR.md).

---

## ⚖️ How Cardly Differs from AnkiWeb and Desktop Anki

| Capability / Feature | Official Desktop Anki | Official AnkiWeb | Cardly (This Project) |
| :--- | :--- | :--- | :--- |
| **Application Type** | Local desktop software (PyQt6 + Python + Rust) | Cloud web service hosted by Anki | Self-hosted web application & PWA |
| **Core Scheduling Engine** | Native `rslib` (Rust) | Custom web backend | Official `rslib` (Rust) linked directly |
| **FSRS v4 / v5 Support** | Full support + weights optimizer + simulator | Review only (no optimizer or simulator) | Full support + live optimizer & simulator |
| **Card Browser** | Full 3-pane browser with bulk edits & search tree | Read-only basic search list | Full 3-pane virtualized browser with bulk actions |
| **Card Template Editor** | Full HTML/CSS editor with live preview | Very limited | Full split-view editor with live preview |
| **Deck Options Configuration** | Full options presets & scheduling controls | Minimal settings | Full upstream Deck Options SvelteKit interface |
| **Media & Math Support** | MathJax, LaTeX (`latex`/`dvipng`), audio, video | Basic MathJax & audio playback | Client-side MathJax 3, HTML5 audio/video, Web TTS |
| **Mobile & Touch UX** | Desktop only (separate AnkiDroid / AnkiMobile apps) | Basic non-PWA responsive web page | Installable PWA with 44px touch buttons & drawer |
| **Sync Protocol Role** | Sync client (syncs to AnkiWeb or custom server) | Central proprietary cloud server | Built-in Anki Sync Server for mobile & desktop |
| **Multi-User Support** | Local profile switching | Single account per email | Multi-user accounts with per-user data isolation |
| **Extensibility (Add-ons)** | PyQt Python add-on manager | No add-on support | Web-safe customizations & Svelte components |
| **Data Privacy & Self-Hosting** | Local on user machine | Data stored on AnkiWeb servers | Fully self-hosted on your own hardware / Docker |
| **Installation Requirement** | Requires OS installer, Python, Qt libraries | No installation (browser-only) | Single command via Docker Compose |

---

## 🚀 One-Command Quickstart

Cardly is packaged as a lightweight, production-ready container requiring only Docker and Docker Compose.

### 1. Clone & Configure
```bash
git clone --recurse-submodules https://github.com/cardly-app/cardly.git
cd cardly
# If already cloned without submodules: git submodule update --init --recursive
cp .env.example .env
```

Edit `.env` to define your user accounts:
```ini
CARDLY_USER1=alice:supersecretpassword
CARDLY_PORT=8080
```

### 2. Launch Cardly
```bash
docker compose up -d
```

### 3. Start Studying
Open your browser and navigate to:
**`http://localhost:8080`**

Log in using the credentials defined in `.env`.

To stop the service:
```bash
docker compose down
```

---

## ⚙️ Configuration Reference (`.env`)

Cardly reads environment variables from `.env` in the project root:

| Variable | Default Value | Description |
| :--- | :--- | :--- |
| `CARDLY_PORT` | `8080` | Host port on which the web application and sync server listen. |
| `CARDLY_HOST` | `0.0.0.0` | Bind address for the HTTP server inside the container. |
| `CARDLY_DATA_DIR` | `./data` (host) / `/data` (container) | Persistent directory storing user collections, media files, and backups. |
| `CARDLY_USER1` | *Required* | First user credentials in `username:password` format. |
| `CARDLY_USER2`, `CARDLY_USER3`, ... | *Optional* | Additional user accounts. Each account receives an isolated collection and sync endpoint. |
| `PASSWORDS_HASHED` | `0` | Set to `1` if passwords specified in `CARDLY_USER*` are pre-hashed PBKDF2 PHC strings. |
| `CARDLY_SESSION_SECRET` | *Auto-generated* | 256-bit secret used to sign session cookies. If empty, auto-generated and stored in `$CARDLY_DATA_DIR/.session_secret`. |
| `CARDLY_COOKIE_SECURE` | `false` | Set to `true` when serving Cardly behind an HTTPS reverse proxy (e.g., Caddy, Traefik, Nginx). |
| `CARDLY_MAX_UPLOAD_MB` | `512` | Maximum allowable file size in MiB for `.apkg`, `.colpkg`, and media file uploads. |
| `RUST_LOG` | `cardly_server=info,anki=info` | Tracing log level filter for the backend server. |

---

## 📱 Mobile Sync Setup (AnkiDroid & AnkiMobile)

Cardly includes a built-in Anki Sync Server. You can synchronize your cards, review progress, and media seamlessly between Cardly and official mobile apps:

### AnkiDroid (Android)
1. Open **AnkiDroid** &rarr; **Settings** &rarr; **Syncing** &rarr; **Custom sync server**.
2. Check **Use custom sync server**.
3. Set **Sync url**: `http://<YOUR_SERVER_IP>:8080/sync/`
4. Set **Media sync url**: `http://<YOUR_SERVER_IP>:8080/msync/`
5. Return to the main screen, tap **Sync**, and enter your `CARDLY_USER1` username and password.

### AnkiMobile (iOS)
1. Open **AnkiMobile** &rarr; **Preferences** &rarr; **Syncing** &rarr; **Custom Server**.
2. Set **Sync URL**: `http://<YOUR_SERVER_IP>:8080/sync/`
3. Enter your username and password.
4. Tap **Sync Now**.

> [!TIP]
> When accessing your server over the internet or public Wi-Fi, it is strongly recommended to run Cardly behind an HTTPS reverse proxy (such as Caddy or Nginx) with Let's Encrypt certificates, and set `CARDLY_COOKIE_SECURE=true`.

---

## 🛠 Local Development & Testing

If you want to contribute to Cardly or run it without Docker:

### Prerequisites
- **Rust** 1.80+ (`cargo`)
- **Node.js** 20+ / 22+ (`npm`)
- **SQLite3** and **protobuf** build tools

### Build & Run
```bash
# 1. Build the Svelte 5 frontend shell
cd web
npm install
npm run build
cd ..

# 2. Start the Rust server
CARDLY_USER1="alice:secret123" \
CARDLY_PORT=8080 \
CARDLY_SHELL_DIR="./web/dist" \
CARDLY_UPSTREAM_WEB_DIR="./anki-upstream/out/qt/_aqt/data/web" \
cargo run --manifest-path server/Cargo.toml
```

### Running Test Suites
```bash
# 1. Rust backend integration tests (validates Protobuf RPC and Anki sync protocol)
cargo test --manifest-path server/Cargo.toml --test integration

# 2. Frontend Svelte 5 type checks
npm --prefix web run check

# 3. Playwright End-to-End tests (desktop and mobile flows)
npm --prefix e2e run test
```

---

## ❓ Frequently Asked Questions (FAQ)

### Is Cardly an official Anki product?
No. Cardly is an independent, open-source project. It is not affiliated with, endorsed by, or maintained by Ankitects Pty Ltd or Damien Elmes. It incorporates upstream open-source code from Anki in accordance with the GNU AGPLv3 license.

### Does Cardly calculate card intervals identically to Anki Desktop?
Cardly links directly against Anki's official `rslib` Rust core rather than re-implementing or approximating spaced repetition algorithms in JavaScript. Every scheduling state calculation, interval calculation, FSRS formula, and database transaction runs through upstream Anki code.

### Can I import my existing `.apkg` or `.colpkg` collections?
Yes. You can import any `.apkg` deck package or `.colpkg` collection file via drag-and-drop or the **File &rarr; Import** menu. All cards, notes, custom note types, tags, scheduling histories, and media are fully preserved.

### Does Cardly support Python add-ons?
No. Desktop Anki add-ons are written in Python and rely directly on desktop PyQt6 GUI APIs. Running arbitrary Python code inside a browser or on a multi-user web server poses major security and architectural incompatibilities. Features commonly added via add-ons (such as FSRS optimization, advanced stats, image occlusion, and dark mode) are already built directly into Cardly.

### How is my data protected?
Each user account has a completely isolated data directory with its own SQLite database (`collection.anki2`), media store, and backup snapshots. Web session authentication uses PBKDF2 key derivation and HttpOnly secure cookies. Crucially, untrusted user-submitted JavaScript from card templates is executed in sandboxed iframes without access to session credentials or sensitive backend APIs.

### How do automatic backups work?
Cardly creates full `.colpkg` snapshots of your collection before major sync operations and schema updates. Backups are saved to the persistent volume under `/data/users/<username>/backups/`. Cardly automatically maintains the latest 50 backups, pruning older snapshots to prevent disk exhaustion. You can download or restore backups at any time from **Preferences &rarr; Backups**.

---

## 🗺️ Roadmap & Community

We are building Cardly in the open for students, language learners, and self-hosters worldwide:

- **[Product Roadmap](./docs/ROADMAP.md)**: Explore planned features from v0.1.0 to v1.0.0 (in-browser audio recording, image occlusion editor, shared deck subscriptions, client-side WASM core).
- **[GitHub Discussions](https://github.com/cardly-app/cardly/discussions)**: Propose features, share custom card CSS themes, and get self-hosting help (see [Discussions Setup Guide](./docs/DISCUSSIONS.md)).
- **[Issue Triage Guide](./docs/TRIAGE.md)**: How issues are prioritized and triaged in under 2 minutes.
- **[Contributing Guide](./CONTRIBUTING.md)**: How to set up a development environment, run tests, and submit PRs.
- **[Security Policy](./SECURITY.md)**: Confidential reporting of vulnerabilities.

---

## 📜 License & Upstream Attribution

Cardly is licensed under the **GNU Affero General Public License, version 3 or later** ([GNU AGPL-3.0-or-later](./LICENSE)).

Cardly is built upon the foundational work of the Anki open-source project:
- **Upstream Repository**: [https://github.com/ankitects/anki](https://github.com/ankitects/anki)
- **Original Author & Maintainer**: Damien Elmes and the Anki contributors
- **Upstream License**: GNU AGPL-3.0-or-later (with portions under BSD-3 and MIT licenses)

For comprehensive attribution, third-party software notices, and contributor listings, please refer to the [NOTICE](./NOTICE) file.
