# Changelog

All notable changes to Cardly will be documented in this file.

## [Unreleased]

### Phase 3 — Polish, Mobile PWA & Production Deployment (2026-11-18)
- **PWA & Standalone App Support:**
  - Added Web App Manifest (`web/public/manifest.webmanifest`) with full metadata, standalone mode, and app shortcuts (Study, Add, Browse).
  - Generated crisp multi-resolution PNG and maskable icons (`icon-192.png`, `icon-512.png`, `icon-maskable-192.png`, `icon-maskable-512.png`, `apple-touch-icon.png`).
  - Implemented lightweight Service Worker (`web/public/sw.js`) with cache-first static shell strategy and background update notification.
  - Added PWA installation banner in `MenuBar.svelte` and standalone app settings in `PreferencesModal.svelte`.
  - Added network status detection with offline indicator and auto-reconnect toasts.
- **Mobile Responsive Layout & Touch Experience:**
  - Optimized viewport for mobile smartphones (tested on iPhone 390×844 and Android devices).
  - Rebuilt reviewer bottom bar for mobile screens: large touch-friendly buttons (height 44px, full-width distribution), status counters above buttons, and safe area insets.
  - Adapted Deck Browser and Deck Overview with responsive table columns, compact padding, and full-width touch actions.
  - Implemented collapsible drawer sidebar for Card Browser on screens <= 768px.
  - Fullscreen bottom-sheet modal behavior on mobile screens (< 640px).
- **Backups & Reliability:**
  - Added automatic backup pruning in `server/src/files.rs` (`prune_old_backups`) enforcing a maximum of 50 historical `.colpkg` snapshots to protect disk space.
  - Added global uncaught error boundaries and promise rejection handlers in `main.ts` and `App.svelte`.
- **E2E Testing & Visual Verification:**
  - Added Flow 4 to Playwright E2E suite (`tests/cardly.spec.ts`) testing mobile viewport (390×844), manifest retrieval, touch review, and layout responsiveness.
  - All 4 Playwright E2E tests pass (100% green, 45.8s).
  - All 4 Rust server integration tests pass (`cargo test --test integration`).
  - Added 6 mobile screenshots to gallery (`14-mobile-deck-list-light.png` through `19-mobile-browser-light.png`), bringing the verified screenshot gallery to 19 total images.
- **Documentation & Deployment:**
  - Authored comprehensive [README.md](./README.md) with architecture overview, screenshot table, one-command Docker Compose setup, and step-by-step sync guides for AnkiDroid and AnkiMobile.
  - Updated `docker-compose.yml` with explicit `CARDLY_SHELL_DIR`.

### Phase 2 — Core Web Shell, Screens & Quality Loop (2026-11-18)
- Built complete Svelte 5 web shell (`web/`) with exact Desktop Anki visual and behavioral parity:
  - `web/src/styles/anki-tokens.css`: Desktop Anki light and dark mode CSS variables, system font stack, buttons, inputs, context menus, shortcuts badges.
  - Global reactive store (`web/src/lib/store.svelte.ts`) with Svelte 5 runes (`$state`, `$derived`, `$effect`), auth, view routing (`deckBrowser`, `deckOverview`, `reviewer`, `browser`, `stats`), modals management, undo/redo state, opRevision reactivity.
  - Protobuf RPC and bridge interceptors (`web/src/lib/api.ts`, `web/src/lib/bridge.ts`, `web/src/lib/renderCard.ts`): Client-side bridge for `FrontendService` (file picker, audio playback, dialogs, clipboard, tag completion), auto-sync of browser timezone, batch row loading, robust sandbox `srcdoc` iframe isolation with origin-safe `postMessage` protocol.
  - Core desktop screens and dialogs:
    1. **Deck List** (`DeckBrowser.svelte`): Hierarchical deck tree with New/Learn/Due counts, collapse/expand subdecks, create/rename/delete deck, drag-and-drop reparenting, gear options menu, studied-today footer.
    2. **Deck Overview** (`DeckOverview.svelte`): Deck title, live New/Learn/Review counts, "Study Now" button, Markdown/HTML description display, Custom Study dialog (`c`), Filtered Deck rebuild/empty.
    3. **Reviewer** (`Reviewer.svelte`): Reused upstream `ts/reviewer/` inside sandboxed iframe with MathJax 3, Cloze, typed answer diffing, HTML5 audio + TTS, 4 rating buttons (Again/Hard/Good/Easy) with interval previews, full desktop keyboard shortcuts (`Space`, `1`–`4`, `u`/`Ctrl+Z`, flags `1`–`7`, mark `*`, bury `-`/`=`, suspend `@`/`!`, delete `Ctrl+Del`, forget, set due date, card info, timer, auto-advance).
    4. **Add / Edit Cards** (`UpstreamEditor.svelte`): Embedded upstream `ts/routes/editor` (Add mode, Current mode, Browser mode) with rich text, MathJax overlay, Cloze, tags, sticky fields, duplicate detection, image occlusion support.
    5. **Card Browser** (`Browser.svelte`): 3-pane resizable layout, full Anki search syntax with history & saved searches, virtualized Cards/Notes table with configurable sortable columns, row coloring (flags, suspended, marked), full sidebar tree (Saved Searches, Today, Flags, Card State, Decks, Note Types, Tags), embedded note editor, bulk actions (Change Deck, Tags, Suspend, Flag, Delete, Find & Replace, Find Duplicates, Grade Now, Export).
    6. **Statistics** (`ts/routes/graphs`): Fully embedded upstream statistics screen.
    7. **Deck Options** (`ts/routes/deck-options`): Fully embedded upstream SvelteKit deck options with FSRS parameters, optimizer, and simulator.
    8. **Note Types & Templates**: Manage Note Types modal, Fields modal, and Card Templates split editor with live Front/Back preview, night mode toggle, and deck override.
    9. **Import / Export & Maintenance**: Drag-and-drop `.apkg`, `.colpkg`, `.csv`/`.tsv` import, export dialog, Check Database, Check Media with Trash, Empty Cards.
    10. **Preferences**: Appearance (Light/Dark/Auto theme), Review (limits, intervals, timebox), Editing, Syncing guide, Backups snapshot & restore.
    11. **Menu Bar & Toolbar**: Top menu bar (`File`, `Edit`, `Tools`, `Help`) with keyboard mnemonics, top toolbar (`Decks`, `Add`, `Browse`, `Stats`, `Sync`), global shortcuts (`d`, `a`, `b`, `t`, `y`, `Ctrl+Z`, `Ctrl+P`, etc.).
- Backend server integration (`server/`):
  - Auto-serving of `web/dist` when `CARDLY_SHELL_DIR` is set or `./web/dist` exists.
  - Fallback `null` response for unset `getConfigJson` keys (e.g. `renderMathjax`).
  - Added multi-stage Docker build step for `web/` producing `/app/web-shell`.
- Testing & Quality Verification:
  - `web/`: `npm run check` passed (`0 errors` in `web/src`), `npm run build` passed (`dist/` generated).
  - `server/`: `cargo test` passed (**4 passed, 0 failed**).
  - `e2e/`: **All 3 Playwright E2E tests passed (100% green, 30.9s)** covering add/review/schedule flow, CSV import/search/bulk tag/suspend flow, and stats/deck options/templates/theme toggle flow.
  - Visual verification with Playwright browser subagent and full 13-screenshot gallery in `docs/screenshots/` (Light and Dark mode for every screen).
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
