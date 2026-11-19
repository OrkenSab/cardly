# Hacker News Launch Kit: Show HN

## Submission Details

- **Title**: Show HN: Cardly – Open-source web clone of Anki with native sync, FSRS, and AI
- **URL**: `https://github.com/ankitects/anki` (or live demo URL: `https://cardly.app`)

---

## Author's First Comment (Submit immediately upon posting)

Hey HN! I'm the author of Cardly.

My girlfriend is in medical school, studying 600+ cards every single day on Anki. While Anki Desktop is one of the most remarkable pieces of open-source software ever created, using it across shared library computers, iPads, and Chromebooks was always frustrating:
- The official AnkiWeb interface is intentionally lightweight and read-only for editing; it doesn't support the full 3-pane browser, tag management, or the FSRS optimizer.
- Every third-party web flashcard app we tested reimplements spaced repetition math in JavaScript, which invariably drifts from true Anki intervals, lacks nested Cloze support, or locks cards into proprietary clouds.

So I spent the last few months building **Cardly** (https://cardly.app): a self-hosted, pixel-perfect web clone of Desktop Anki that directly reuses Anki's official Rust core.

### Architecture & Technical Decisions

1. **Zero-Fork Rust Core (`rslib`) via Axum**:
   Rather than attempting to port Anki to WASM with browser SQLite (which chokes on multi-GB collections, single-threaded FSRS optimization, and mobile Safari OPFS memory limits), Cardly runs Anki's native `rslib` inside a Rust Axum server binary. We link directly against `anki = { path = "../anki-upstream/rslib" }` with zero fork.

2. **Automated Protobuf RPC Router**:
   In official Anki, `qt/aqt/mediasrv.py` bridges the frontend and backend using Protobuf over HTTP (`POST /_anki/<method>`). We parse `anki_descriptors.bin` at startup with `anki_proto_gen` to dynamically map every camelCase method directly to `Backend::run_service_method`. Over 250 RPC endpoints work out of the box without hand-writing boilerplate.

3. **Built-in Anki Sync Server (`server: true` mode)**:
   By initializing Anki's backend with `server: true`, every web mutation automatically writes a positive server USN. We mounted Anki's upstream `SyncProtocol` and `MediaSyncProtocol` on `/sync/*` and `/msync/*`. This means your phone (AnkiDroid on Android or AnkiMobile on iOS) syncs directly to your Cardly instance—no AnkiWeb account required!

4. **Svelte 5 Web Shell with Runes**:
   The frontend is built with Svelte 5 (`$state`, `$derived`, `$effect`) and TypeScript, faithfully implementing Desktop Anki's design system (`anki-tokens.css`) across light, dark, and minimalist modes. All desktop hotkeys (`Space`/`Enter`, `1`–`4`, `d`, `a`, `b`, `t`, `y`, `Ctrl+Z`, flags `1`–`7`, mark `*`, suspend `@`/`!`) match desktop behavior.

5. **Sandbox Security for Untrusted Card JS**:
   Shared Anki decks often contain custom JavaScript (pitch accents, interactive cloze, diagrams). Cardly renders cards inside isolated `srcdoc` iframes governed by a strict Content Security Policy (`UNTRUSTED_MEDIA_CSP`). Crucially, all mutating backend RPCs require an `X-Cardly-Api-Token` header that only the top-level application shell possesses; card scripts cannot read session cookies or mutate collections.

6. **AI Card Generation & Multimodal OCR**:
   We added an AI generator that turns lecture notes, Markdown files, or textbook photos into atomic, high-retention Q&A or Cloze cards. It supports Google Gemini (multimodal OCR), OpenAI/OpenRouter, and 100% local Ollama instances (`llama3.2`).

### Licensing & Upstream Credits

Cardly is licensed under the **GNU Affero General Public License v3 or later (AGPL-3.0-or-later)**, in complete harmony with the upstream Anki project. We extend our deepest gratitude to Damien Elmes and the hundreds of Anki contributors who have spent almost two decades building the gold standard of spaced repetition.

### Getting Started

You can spin up a self-hosted instance in 30 seconds with Docker Compose:

```bash
git clone https://github.com/ankitects/anki.git cardly
cd cardly
cp .env.example .env
docker compose up -d
```

Open `http://localhost:8080`, log in with `alice:secret123`, and start studying.

- **GitHub Repository**: https://github.com/ankitects/anki
- **Live Demo**: https://cardly.app (or `http://localhost:8080`)
- **Documentation**: https://github.com/ankitects/anki/tree/main/docs

I'd love your feedback, bug reports, and thoughts on the architecture!
