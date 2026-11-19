# Product Hunt Launch Kit & Awesome-Selfhosted PR

---

## 🚀 Product Hunt Launch Metadata

- **Product Name**: Cardly
- **Tagline**: The open-source Anki web client with FSRS & native mobile sync
- **Thumbnail / Logo**: `web/public/cardly-icon.svg` (or 240×240 PNG)
- **Gallery Images / Video**:
  - Video: `docs/demo/cardly-demo.mp4` (H.264, 1280×800)
  - Hero Preview: `docs/brand/social-preview.png` (1280×640)
  - Screenshot 1: `docs/screenshots/01-deck-list-light.png` (Deck Browser)
  - Screenshot 2: `docs/screenshots/04-reviewer-answer-light.png` (Reviewer with FSRS ease intervals)
  - Screenshot 3: `docs/screenshots/06-browser-light.png` (3-Pane Browser)
  - Screenshot 4: `docs/screenshots/08-deck-options-fsrs-light.png` (FSRS Optimizer & Simulator)
  - Screenshot 5: `docs/screenshots/14-mobile-deck-list-light.png` (Mobile PWA)
- **Topics / Tags**:
  - `Open Source`
  - `Productivity`
  - `Education`
  - `Developer Tools`
  - `Self-Hosted`

### Short Description (60 characters)
Open-source web clone of Anki Desktop with FSRS & mobile sync.

### Detailed Description (260 characters)
Cardly brings the full power of Anki Desktop to your web browser and mobile devices. Powered directly by Anki's official Rust core (rslib), it features FSRS v4/v5 scheduling, desktop-grade card browsing, and a built-in sync server for AnkiDroid and AnkiMobile.

---

## 💬 Maker Comment (Post immediately on launch)

Hey Product Hunt community! 👋

I'm the maker of **Cardly**.

If you've ever studied a new language, prepared for technical interviews, or survived medical/law exams, you know that **Anki** is the undisputed king of spaced repetition. However, taking Anki on the go without carrying your personal laptop has always been difficult:
- The official AnkiWeb interface is intentionally barebones—it lacks full card browsing, tag organization, and modern FSRS weight optimization.
- Most third-party web apps rewrite the scheduling formulas in JavaScript, which often drift from authentic Anki intervals or lock your flashcards in proprietary clouds.

We built **Cardly** (https://cardly.app) to give you the exact power, precision, and hotkey muscle memory of Anki Desktop in a modern, responsive web application you can self-host in 30 seconds.

### 🌟 What makes Cardly unique:
1. **Official Rust Core (`rslib`)**: Every scheduling calculation, database transaction, and stability interval runs through Anki's official Rust binaries. Zero approximation.
2. **Modern FSRS v4 & v5**: Includes the in-browser weights optimizer and retention simulator.
3. **Built-in Anki Sync Server**: Cardly implements Anki's official `/sync/*` protocol. You can point **AnkiDroid** (Android) or **AnkiMobile** (iOS) at your server and sync directly!
4. **Desktop Hotkeys & Parity**: 3-pane virtualized browser, MathJax 3, nested cloze, audio/video playback, and full keyboard navigation.
5. **Installable PWA**: Responsive mobile touch layout with 44px buttons, offline shell caching, and standalone display.
6. **AI Flashcard Generator**: Turns study notes, articles, or textbook photos (via Gemini multimodal OCR, OpenAI, or 100% offline local Ollama) into atomic flashcards.
7. **100% Open Source (GNU AGPL-3.0)**: Zero telemetry, self-hosted via Docker Compose, and your data stays completely yours.

Try the live demo at **https://cardly.app** or spin up your own instance with `docker compose up -d`.

I'd love to hear your thoughts, feedback, and study workflows!

---

## 📦 Pull Request Snippet for `awesome-selfhosted`

Repository: [awesome-selfhosted/awesome-selfhosted](https://github.com/awesome-selfhosted/awesome-selfhosted)
Target File: `README.md`
Target Category: `Communication - Custom / Flashcards` (or `Learning and Courses`)

### PR Title:
`Add Cardly to Communication - Custom / Flashcards`

### PR Description:
```markdown
### Changes
- Adds Cardly to the `Communication - Custom / Flashcards` section in alphabetical order.

Cardly is an open-source, self-hosted web clone of Anki Desktop powered directly by Anki's official Rust core (`rslib`). It features FSRS v4/v5 scheduling, desktop-grade card browsing, mobile PWA support, and a built-in sync server for AnkiDroid and AnkiMobile.

- License: AGPL-3.0
- Technology: Rust, Svelte 5, Docker
- Demo available: Yes
```

### Exact Markdown Snippet to Insert:
```markdown
- [Cardly](https://cardly.app) - Self-hosted web clone of Anki Desktop powered directly by official Anki Rust core (`rslib`), with FSRS v4/v5 scheduling, mobile PWA, and built-in AnkiDroid/AnkiMobile sync server. ([Demo](https://cardly.app), [Source Code](https://github.com/ankitects/anki)) `AGPL-3.0` `Rust/Docker`
```
*(Insert in alphabetical order between existing entries in the Flashcards section)*
