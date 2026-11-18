# Contributing to Cardly

Thank you for your interest in contributing to **Cardly**! We welcome contributions ranging from bug fixes and performance improvements to documentation enhancements and internationalization.

This document outlines the development workflow, project architecture, and coding standards.

---

## Code of Conduct

All contributors and maintainers are expected to adhere to our [Code of Conduct](./CODE_OF_CONDUCT.md) to ensure an open, welcoming, and harassment-free environment for everyone.

---

## Project Architecture

Cardly consists of three primary components:

1. **Backend Server (`server/`)**:
   - Written in Rust using the Axum web framework.
   - Links directly against upstream `rslib` (`anki-upstream/rslib`) without custom forks.
   - Dispatches Protobuf RPCs dynamically via `anki_proto_gen` descriptors.
   - Mounts the built-in Anki Sync Server (`/sync/*` and `/msync/*`).
   - Manages per-user storage, PBKDF2 authentication, and automated backup rotation.

2. **Web Shell Frontend (`web/`)**:
   - Built with **Svelte 5** using modern Runes (`$state`, `$derived`, `$effect`) and TypeScript.
   - Implements the Desktop Anki styling system via `src/styles/anki-tokens.css` (light & dark mode).
   - Reuses compiled upstream SvelteKit pages (`ts/routes/*`) and reviewer runtimes (`ts/reviewer/`) inside sandboxed iframes.
   - PWA support with a dedicated service worker (`sw.js`) and manifest (`manifest.webmanifest`).

3. **End-to-End Test Suite (`e2e/`)**:
   - Written in TypeScript using **Playwright**.
   - Tests complete real-world workflows: study sessions, card creation, CSV imports, FSRS configuration, responsive mobile views, and touch controls.

---

## Getting Started

### Prerequisites
- **Rust** 1.80 or newer (`rustup default stable`)
- **Node.js** 20.x or 22.x LTS and `npm`
- **SQLite3** libraries and development headers
- **protobuf-compiler** (`protoc`)
- Optional: **Docker** and **Docker Compose**

### Setting Up Local Environment

1. Clone the repository with submodules:
   ```bash
   git clone --recurse-submodules https://github.com/ankitects/anki.git cardly
   cd cardly
   ```

2. Copy the sample environment file:
   ```bash
   cp .env.example .env
   ```

3. Build the frontend web shell:
   ```bash
   cd web
   npm install
   npm run build
   cd ..
   ```

4. Run the Rust server in development mode:
   ```bash
   CARDLY_USER1="alice:secret123" \
   CARDLY_PORT=8080 \
   CARDLY_SHELL_DIR="./web/dist" \
   CARDLY_UPSTREAM_WEB_DIR="./anki-upstream/out/qt/_aqt/data/web" \
   cargo run --manifest-path server/Cargo.toml
   ```

5. Open your browser at `http://localhost:8080` and log in with `alice` / `secret123`.

---

## Testing & Quality Assurance

Before submitting any code changes, ensure all automated verification suites pass:

### 1. Frontend Typecheck
```bash
npm --prefix web run check
```
Verifies that all Svelte 5 components and TypeScript modules compile without diagnostics or type errors.

### 2. Backend Integration Tests
```bash
cargo test --manifest-path server/Cargo.toml --test integration
```
Runs integration tests against the live Axum router, validating Protobuf RPC dispatch, cookie authentication, and the Anki Sync Server protocol with `HttpSyncClient`.

### 3. Playwright E2E Suite
```bash
npm --prefix e2e run test
```
Executes headless browser tests across all core desktop and mobile user flows.

---

## Coding Standards & Conventions

- **Rust**:
  - Follow idiomatic Rust styles.
  - Run `cargo fmt` and `cargo clippy --manifest-path server/Cargo.toml` before committing.
  - Avoid `unwrap()` in production server routes; handle errors with appropriate HTTP status codes.

- **Svelte 5 & TypeScript**:
  - Use Svelte 5 runes syntax (`$state`, `$derived`, `$props`) instead of legacy Svelte 3/4 reactive declarations (`$:`, `export let`).
  - Use CSS custom properties defined in `anki-tokens.css` to ensure full consistency between light and dark themes.
  - Keep components modular and accessible. Reviewer controls must maintain desktop keyboard shortcuts and mobile 44px touch targets.

- **Localization**:
  - When modifying user-facing documentation or strings, update the corresponding translations in English (`README.md`), Russian (`README.ru.md`), and Kazakh (`README.kk.md`).

---

## Licensing of Contributions

Cardly is licensed under the **GNU Affero General Public License, version 3 or later** (GNU AGPL-3.0-or-later).

By submitting a pull request or patch to Cardly, you agree that your contributions will be licensed under the GNU AGPL-3.0-or-later license, in harmony with upstream Anki.

---

## Submitting Pull Requests

1. Fork the repository and create a feature branch (`git checkout -b feature/my-enhancement`).
2. Implement your changes with accompanying tests if applicable.
3. Verify that `cargo test`, `npm run check`, and Playwright tests pass cleanly.
4. Commit your changes with clear, descriptive commit messages.
5. Push your branch to GitHub and open a Pull Request against `main`.
6. Maintainers will review your PR and provide constructive feedback.
