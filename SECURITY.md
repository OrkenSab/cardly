# Security Policy for Cardly

## Overview
Security is a core consideration in Cardly's design. Because spaced repetition cards frequently embed custom HTML, CSS, and user-provided JavaScript (e.g. for stroke order diagrams, custom cloze interactions, and audio controls), Cardly implements rigorous defense-in-depth boundaries to protect users, their data, and host systems.

---

## Supported Versions

Only the latest release and the current `main` branch receive active security updates.

| Version / Branch | Supported | Notes |
| :--- | :--- | :--- |
| `main` (latest Docker container) | Yes | Actively maintained and patched |
| Older releases | No | Please upgrade to the latest Docker image |

---

## Architecture & Threat Model

Cardly operates under the assumption that card templates and shared decks (`.apkg` / `.colpkg`) may contain untrusted, potentially malicious JavaScript. To mitigate this threat, the application enforces multiple layers of isolation:

### 1. Sandboxed Reviewer & Editor Previews
- Card templates and reviewer views are rendered inside isolated `srcdoc` `<iframe>` elements.
- The reviewer sandbox is governed by a strict Content Security Policy (`UNTRUSTED_MEDIA_CSP`), preventing unauthorized frame navigation and cross-origin access.
- Inter-frame communication between the Cardly Svelte shell and card frames uses a hardened `postMessage` protocol with origin validation.

### 2. Session and API Token Isolation (`X-Cardly-Api-Token`)
- All mutating RPC endpoints (`/_anki/*` and `/api/*`) require a per-session random header token: `X-Cardly-Api-Token`.
- Even if custom card JavaScript escapes its frame sandbox, it cannot read or forge this token because session tokens are scoped exclusively to the top-level application shell.
- Authentication cookies use `HttpOnly` and `SameSite` flags, preventing JavaScript from reading session secrets.

### 3. Multi-User Directory Isolation
- Every user account resides in a dedicated, isolated subdirectory under `$CARDLY_DATA_DIR/users/<username>/`.
- Collections (`collection.anki2`), media files (`media/`), media databases (`media.db`), and backups (`backups/`) are completely partitioned per user.
- A user cannot read, query, or mutate another user's collection or files.

### 4. File Upload & Media Path Traversal Defense
- All media file uploads and package imports (`.apkg`, `.colpkg`) validate file names against directory traversal sequences (such as `../` or absolute paths).
- Files are saved strictly within the user's isolated `media/` directory.

### 5. Automated Backup Limits
- To prevent denial-of-service via disk exhaustion, Cardly enforces an automatic rolling backup policy, maintaining at most 50 recent `.colpkg` snapshots per user.

---

## Reporting a Vulnerability

If you discover a security vulnerability in Cardly, please do **not** report it in a public GitHub issue.

Instead, please submit a responsible disclosure report privately:
1. Use GitHub's private vulnerability reporting feature on the repository: **Security &rarr; Advisories &rarr; Report a vulnerability**.
2. Alternatively, email the maintainers directly at: **security@cardly.local** (or repository owner's contact email).

### What to Include
To help us triage and patch the issue quickly, please include:
- A clear description of the vulnerability and its potential impact.
- Step-by-step reproduction instructions or a minimal Proof of Concept (e.g. an `.apkg` file demonstrating the issue).
- The affected component (frontend Svelte shell, Rust Axum server, or Docker configuration).

### Our Commitment
- We will acknowledge receipt of your vulnerability report within **48 hours**.
- We will provide a status update and estimated fix timeline within **7 business days**.
- Once a fix is verified, we will release a patched Docker container and credit the reporter (unless anonymity is requested).
