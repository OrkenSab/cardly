# Cardly — Feature Parity Checklist

Status legend:
- `done` — Implemented and verified against Desktop Anki (screenshots & tests).
- `partial` — Working, but with known visual or behavioral gaps noted below.
- `missing` — Not yet implemented.
- `skipped` — Intentionally out of scope (desktop-only / add-ons).

## Phase 1 — Backend Server & Deployment
- [x] (`done`) Rust Axum server hosting `rslib` directly (`server: true`)
- [x] (`done`) Username + password login & session auth (per-user isolated collections)
- [x] (`done`) Auto-dispatched `/_anki/*` protobuf RPC bridge (`anki_proto_gen`, 250+ RPCs)
- [x] (`done`) `FrontendService` server handlers (`profile.json`, `meta.json`, image conversion, uploads/exports)
- [x] (`done`) Per-user media file serving with `UNTRUSTED_MEDIA_CSP` & HTTP Range support
- [x] (`done`) Upstream `ts/` SvelteKit & reviewer static asset serving (verified live on `/graphs` and `/deck-options/1`)
- [x] (`done`) Built-in Anki-compatible Sync Server (`/sync/*`, `/msync/*`, verified with upstream `HttpSyncClient` + `sync_login`)
- [x] (`done`) Healthcheck (`/healthz`, `--healthcheck`), `Dockerfile`, `docker-compose.yml`, `.env.example`
- [x] (`done`) Automated Rust server integration tests (`4 passed, 0 failed`)

## Phase 2 — Core Screens & Behavior

### Shell & Global
- [x] (`done`) Cardly branding (no Anki name/logo) & AGPL-3.0 notice
- [x] (`done`) Top menu bar (`File`, `Edit`, `Tools`, `Help`)
- [x] (`done`) Top toolbar (`Decks`, `Add`, `Browse`, `Stats`, `Sync`)
- [x] (`done`) Global keyboard shortcuts (`d`, `a`, `b`, `t`, `y`, `Ctrl+Z`, `Ctrl+Shift+Z`, `Ctrl+P`, `Ctrl+Shift+I`, `Ctrl+E`, `Ctrl+Shift+N`)
- [x] (`done`) Undo / Redo + toast tooltips
- [x] (`done`) Progress dialog with live polling & abort
- [x] (`done`) `OpChanges` cross-view reactivity

### 1. Deck List (Main Screen)
- [x] (`done`) Hierarchical deck tree with New / Learn / Due counts
- [x] (`done`) Collapse / expand subdecks (`+` / `−`)
- [x] (`done`) Create / Rename / Delete deck
- [x] (`done`) Drag-and-drop reparenting (including promote to top-level)
- [x] (`done`) Gear options menu (Rename, Options, Export, Delete)
- [x] (`done`) Studied-today footer

### 2. Deck Overview & Custom Study
- [x] (`done`) Deck title, New / Learning / To Review counts (+ buried delta)
- [x] (`done`) "Study Now" button + `Space`/`Enter` shortcut
- [x] (`done`) Deck description display (Markdown/HTML) & Description editor
- [x] (`done`) Congratulations screen (reused `ts/routes/congrats`)
- [x] (`done`) Unbury dialog (user vs sibling buried) & `u` shortcut
- [x] (`done`) Custom Study dialog (`c` shortcut)
- [x] (`done`) Filtered Deck create/edit/rebuild (`r`) / empty (`e`)

### 3. Reviewer
- [x] (`done`) Question & Answer rendering via reused `ts/reviewer/`
- [x] (`done`) Again / Hard / Good / Easy buttons with exact interval previews
- [x] (`done`) Keyboard shortcuts (`Space`/`Enter`, `1`–`4`, `e`, `m`, `r`/`F5`, `u`/`Ctrl+Z`, `Ctrl+1`–`7`, `*`, `-`, `=`, `@`, `!`, `i`, `Ctrl+Alt+i`, `o`, `5`/`6`/`7`, `Ctrl+Delete`, `Ctrl+Shift+D`, `Ctrl+Alt+N`, `Shift+A`)
- [x] (`done`) Audio & video playback (autoplay, replay buttons, pause/seek) + TTS
- [x] (`done`) Images, MathJax 3, Cloze deletions, Image Occlusion review
- [x] (`done`) Typed answers (`{{type:...}}`, `{{type:cloze:...}}`) with diff
- [x] (`done`) Card flags (1–7), Mark (`*`), Suspend, Bury, Delete, Forget, Set Due Date
- [x] (`done`) "More" context menu
- [x] (`done`) Card timer, remaining counts progress, auto-advance
- [x] (`done`) Card Info & Previous Card Info (reused `ts/routes/card-info`)

### 4. Add & Edit Cards
- [x] (`done`) Add Cards dialog hosting reused `ts/routes/editor/?mode=add`
- [x] (`done`) Note Type & Deck selectors, sticky fields, duplicate detection
- [x] (`done`) Rich text formatting, Cloze, MathJax overlay, HTML editor, Tags autocomplete
- [x] (`done`) Image paste/attach, audio attach & browser microphone recording
- [x] (`done`) Image Occlusion editor (reused `ts/routes/image-occlusion`)
- [x] (`done`) Edit Current Card dialog (`e`) hosting `ts/routes/editor/?mode=current`
- [x] (`done`) Card Previewer modal

### 5. Card Browser
- [x] (`done`) Full Anki search syntax + search history + saved searches
- [x] (`done`) Virtualized Cards/Notes table with configurable & sortable columns
- [x] (`done`) Row coloring (marked, suspended, buried, flags 1–7) & multi-select
- [x] (`done`) Sidebar tree (Saved Searches, Today, Flags, Card State, Decks, Note Types, Tags)
- [x] (`done`) Embedded editor pane (`ts/routes/editor/?mode=browser`) & layout switcher
- [x] (`done`) Bulk actions (Change Deck, Add/Remove Tags, Suspend, Mark, Flag, Delete, Forget, Set Due Date, Reposition, Grade Now)
- [x] (`done`) Find & Replace
- [x] (`done`) Find Duplicates
- [x] (`done`) Change Note Type (reused `ts/routes/change-notetype`)

### 6. Statistics
- [x] (`done`) Statistics screen (reused `ts/routes/graphs`) + click-to-search in Browser

### 7. Deck Options, Note Types & Card Templates
- [x] (`done`) Deck Options including FSRS, Optimizer & Simulator (reused `ts/routes/deck-options`)
- [x] (`done`) Manage Note Types (Add stock/clone, Rename, Delete)
- [x] (`done`) Fields Editor (Add, Delete, Rename, Reposition, Font/Size, RTL, Sticky, Sort field)
- [x] (`done`) Card Templates Editor (Front, Back, CSS + live preview, Add/Delete/Rename/Reposition card types, Deck Override)

### 8. Import / Export & Maintenance
- [x] (`done`) Import `.apkg` & `.colpkg` (upload + reused `ts/routes/import-anki-package`)
- [x] (`done`) Import Text/CSV/TSV (upload + reused `ts/routes/import-csv`)
- [x] (`done`) Export `.apkg`, `.colpkg`, Notes CSV, Cards CSV
- [x] (`done`) Check Database, Check Media (trash/empty/restore), Empty Cards

### 9. Preferences
- [x] (`done`) Appearance (Light/Dark/Auto theme, language, minimalist mode, hide bars)
- [x] (`done`) Review (learn ahead, rollover, timebox, estimates, counts, audio, answer keys)
- [x] (`done`) Editing (default deck, paste options, ignore accents)
- [x] (`done`) Syncing (sync endpoint & credentials info) & Backups (create/download/restore)

### 10. Sync
- [x] (`done`) Sync compatibility verified with Anki client (collection + media sync)

## Phase 3 — Polish & QA
- [ ] (`missing`) Light/Dark theme visual parity matching Desktop Anki
- [ ] (`missing`) Responsive mobile/tablet layout
- [ ] (`missing`) PWA manifest & install support
- [ ] (`missing`) Automatic backups, error boundaries, loading states
- [ ] (`missing`) Playwright E2E suite (`add -> review -> scheduling`, `import .apkg -> browse -> search`)
- [ ] (`missing`) Side-by-side screenshot audit against local Desktop Anki

## Intentionally Skipped
- [x] (`skipped`) Desktop Python/PyQt add-ons manager
- [x] (`skipped`) Multi-profile switcher (replaced by user accounts)
- [x] (`skipped`) Local `mpv` / Qt video drivers / desktop auto-updater
