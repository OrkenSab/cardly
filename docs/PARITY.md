# Cardly — Feature Parity Checklist

Status legend:
- `done` — Implemented and verified against Desktop Anki (screenshots & tests).
- `partial` — Working, but with known visual or behavioral gaps noted below.
- `missing` — Not yet implemented.
- `skipped` — Intentionally out of scope (desktop-only / add-ons).

## Phase 1 — Backend Server & Deployment
- [ ] (`missing`) Rust Axum server hosting `rslib` directly (`server: true`)
- [ ] (`missing`) Username + password login & session auth (per-user isolated collections)
- [ ] (`missing`) Auto-dispatched `/_anki/*` protobuf RPC bridge (`anki_proto_gen`)
- [ ] (`missing`) `FrontendService` server handlers (`profile.json`, `meta.json`, image conversion, uploads/exports)
- [ ] (`missing`) Per-user media file serving with `UNTRUSTED_MEDIA_CSP`
- [ ] (`missing`) Upstream `ts/` SvelteKit & reviewer static asset serving
- [ ] (`missing`) Built-in Anki-compatible Sync Server (`/sync/*`, `/msync/*`)
- [ ] (`missing`) Healthcheck (`/healthz`), `Dockerfile`, `docker-compose.yml`, `.env.example`
- [ ] (`missing`) Automated Rust server integration tests

## Phase 2 — Core Screens & Behavior

### Shell & Global
- [ ] (`missing`) Cardly branding (no Anki name/logo) & AGPL-3.0 notice
- [ ] (`missing`) Top menu bar (`File`, `Edit`, `Tools`, `Help`)
- [ ] (`missing`) Top toolbar (`Decks`, `Add`, `Browse`, `Stats`, `Sync`)
- [ ] (`missing`) Global keyboard shortcuts (`d`, `a`, `b`, `t`, `y`, `Ctrl+Z`, `Ctrl+Shift+Z`, `Ctrl+P`, `Ctrl+Shift+I`, `Ctrl+E`, `Ctrl+Shift+N`)
- [ ] (`missing`) Undo / Redo + toast tooltips
- [ ] (`missing`) Progress dialog with live polling & abort
- [ ] (`missing`) `OpChanges` cross-view reactivity

### 1. Deck List (Main Screen)
- [ ] (`missing`) Hierarchical deck tree with New / Learn / Due counts
- [ ] (`missing`) Collapse / expand subdecks (`+` / `−`)
- [ ] (`missing`) Create / Rename / Delete deck
- [ ] (`missing`) Drag-and-drop reparenting (including promote to top-level)
- [ ] (`missing`) Gear options menu (Rename, Options, Export, Delete)
- [ ] (`missing`) Studied-today footer

### 2. Deck Overview & Custom Study
- [ ] (`missing`) Deck title, New / Learning / To Review counts (+ buried delta)
- [ ] (`missing`) "Study Now" button + `Space`/`Enter` shortcut
- [ ] (`missing`) Deck description display (Markdown/HTML) & Description editor
- [ ] (`missing`) Congratulations screen (reused `ts/routes/congrats`)
- [ ] (`missing`) Unbury dialog (user vs sibling buried) & `u` shortcut
- [ ] (`missing`) Custom Study dialog (`c` shortcut)
- [ ] (`missing`) Filtered Deck create/edit/rebuild (`r`) / empty (`e`)

### 3. Reviewer
- [ ] (`missing`) Question & Answer rendering via reused `ts/reviewer/`
- [ ] (`missing`) Again / Hard / Good / Easy buttons with exact interval previews
- [ ] (`missing`) Keyboard shortcuts (`Space`/`Enter`, `1`–`4`, `e`, `m`, `r`/`F5`, `u`/`Ctrl+Z`, `Ctrl+1`–`7`, `*`, `-`, `=`, `@`, `!`, `i`, `Ctrl+Alt+i`, `o`, `5`/`6`/`7`, `Ctrl+Delete`, `Ctrl+Shift+D`, `Ctrl+Alt+N`, `Shift+A`)
- [ ] (`missing`) Audio & video playback (autoplay, replay buttons, pause/seek) + TTS
- [ ] (`missing`) Images, MathJax 3, Cloze deletions, Image Occlusion review
- [ ] (`missing`) Typed answers (`{{type:...}}`, `{{type:cloze:...}}`) with diff
- [ ] (`missing`) Card flags (1–7), Mark (`*`), Suspend, Bury, Delete, Forget, Set Due Date
- [ ] (`missing`) "More" context menu
- [ ] (`missing`) Card timer, remaining counts progress, auto-advance
- [ ] (`missing`) Card Info & Previous Card Info (reused `ts/routes/card-info`)

### 4. Add & Edit Cards
- [ ] (`missing`) Add Cards dialog hosting reused `ts/routes/editor/?mode=add`
- [ ] (`missing`) Note Type & Deck selectors, sticky fields, duplicate detection
- [ ] (`missing`) Rich text formatting, Cloze, MathJax overlay, HTML editor, Tags autocomplete
- [ ] (`missing`) Image paste/attach, audio attach & browser microphone recording
- [ ] (`missing`) Image Occlusion editor (reused `ts/routes/image-occlusion`)
- [ ] (`missing`) Edit Current Card dialog (`e`) hosting `ts/routes/editor/?mode=current`
- [ ] (`missing`) Card Previewer modal

### 5. Card Browser
- [ ] (`missing`) Full Anki search syntax + search history + saved searches
- [ ] (`missing`) Virtualized Cards/Notes table with configurable & sortable columns
- [ ] (`missing`) Row coloring (marked, suspended, buried, flags 1–7) & multi-select
- [ ] (`missing`) Sidebar tree (Saved Searches, Today, Flags, Card State, Decks, Note Types, Tags)
- [ ] (`missing`) Embedded editor pane (`ts/routes/editor/?mode=browser`) & layout switcher
- [ ] (`missing`) Bulk actions (Change Deck, Add/Remove Tags, Suspend, Mark, Flag, Delete, Forget, Set Due Date, Reposition, Grade Now)
- [ ] (`missing`) Find & Replace
- [ ] (`missing`) Find Duplicates
- [ ] (`missing`) Change Note Type (reused `ts/routes/change-notetype`)

### 6. Statistics
- [ ] (`missing`) Statistics screen (reused `ts/routes/graphs`) + click-to-search in Browser

### 7. Deck Options, Note Types & Card Templates
- [ ] (`missing`) Deck Options including FSRS, Optimizer & Simulator (reused `ts/routes/deck-options`)
- [ ] (`missing`) Manage Note Types (Add stock/clone, Rename, Delete)
- [ ] (`missing`) Fields Editor (Add, Delete, Rename, Reposition, Font/Size, RTL, Sticky, Sort field)
- [ ] (`missing`) Card Templates Editor (Front, Back, CSS + live preview, Add/Delete/Rename/Reposition card types, Deck Override)

### 8. Import / Export & Maintenance
- [ ] (`missing`) Import `.apkg` & `.colpkg` (upload + reused `ts/routes/import-anki-package`)
- [ ] (`missing`) Import Text/CSV/TSV (upload + reused `ts/routes/import-csv`)
- [ ] (`missing`) Export `.apkg`, `.colpkg`, Notes CSV, Cards CSV
- [ ] (`missing`) Check Database, Check Media (trash/empty/restore), Empty Cards

### 9. Preferences
- [ ] (`missing`) Appearance (Light/Dark/Auto theme, language, minimalist mode, hide bars)
- [ ] (`missing`) Review (learn ahead, rollover, timebox, estimates, counts, audio, answer keys)
- [ ] (`missing`) Editing (default deck, paste options, ignore accents)
- [ ] (`missing`) Syncing (sync endpoint & credentials info) & Backups (create/download/restore)

### 10. Sync
- [ ] (`missing`) Sync compatibility verified with Anki client (collection + media sync)

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
