# Cardly GitHub Issue & Pull Request Labels

This document specifies the standard label taxonomy used across the Cardly repository for issue triage, task categorization, workflow tracking, and contributor onboarding.

---

## 🎨 Label Taxonomy & Color Conventions

Labels are grouped into distinct functional prefixes and semantic color codes:

| Category | Description | Prefix | Base Color |
| :--- | :--- | :--- | :--- |
| **Type** | The nature of the change or issue | *(none)* | Mixed semantic |
| **Community** | Guidance for external and new contributors | *(none)* | Purple / Teal |
| **Status / Triage** | Lifecycle status during review | *(none)* / `status:` | Grayscale / Semantic |
| **Area** | Functional subsystem affected | `area:*` | Cool Blues / Teals |
| **Priority** | Urgency and impact level | `priority:*` | Red to Blue spectrum |

---

## 1. Type Labels

| Label | Color | Hex | Description & Guidelines |
| :--- | :--- | :--- | :--- |
| `bug` | Red | `#d73a4a` | Something isn't working as expected, crashes, or deviates from desktop Anki behavior. |
| `enhancement` | Cyan | `#a2eeef` | New feature request, parity improvement, or enhancement to existing functionality. |
| `documentation` | Blue | `#0075ca` | Improvements, fixes, or additions to documentation (`README`, `docs/`, inline docstrings). |
| `maintenance` | Yellow | `#fbca04` | Tooling, dependency updates, CI/CD refactoring, performance improvements, and tests. |
| `security` | Dark Red | `#ee0701` | Vulnerability report, cryptographic issue, or authentication/session flaw. |
| `performance` | Indigo | `#5319e7` | Memory optimization, database query tuning, or frontend render speedup. |

---

## 2. Community & Onboarding Labels

| Label | Color | Hex | Description & Guidelines |
| :--- | :--- | :--- | :--- |
| `good first issue` | Purple | `#7057ff` | Ideal for newcomers and first-time contributors. Small, self-contained scope with clear requirements and minimal prerequisite knowledge. |
| `help wanted` | Teal | `#008672` | Maintainers actively seek community assistance, feedback, or implementation help on this item. |

### Guidelines for `good first issue`:
- Issue description must link to relevant source files and line ranges.
- Verification steps must be clearly described (e.g., `npm run check` or `cargo test`).
- Maintainers are available to provide code review guidance.

---

## 3. Triage & Lifecycle Labels

| Label | Color | Hex | Description & Guidelines |
| :--- | :--- | :--- | :--- |
| `triage` | Light Gray | `#ededed` | Newly submitted issue or PR awaiting initial maintainer review, verification, and labeling. |
| `confirmed` | Green | `#0e8a16` | Bug has been reproduced by maintainers or feature request has been formally accepted. |
| `blocked` | Black | `#000000` | Work cannot proceed until an external dependency, upstream Anki release, or related issue is resolved. |
| `duplicate` | Cool Gray | `#cfd3d7` | Issue or pull request duplicates an existing open or closed issue. Link to duplicate when closing. |
| `invalid` | Muted Yellow | `#e4e669` | Issue cannot be reproduced, is out of scope, or represents misconfiguration rather than a defect. |
| `wontfix` | White/Light | `#ffffff` | Maintainers have deliberately decided not to implement or fix this (e.g. conflicting with desktop parity goals). |

---

## 4. Area / Subsystem Labels (`area:*`)

| Label | Color | Hex | Component Description |
| :--- | :--- | :--- | :--- |
| `area:server` | Deep Blue | `#1d76db` | Rust Axum HTTP backend, RPC dispatch, user session auth, and SQLite storage. |
| `area:web` | Deep Purple | `#5319e7` | Svelte 5 web shell, Runes state management, theme styles (`anki-tokens.css`). |
| `area:sync` | Ocean Blue | `#006b75` | Anki sync protocol (`/sync/*`, `/msync/*`) compatibility with AnkiDroid and AnkiMobile. |
| `area:fsrs` | Forest Green | `#0e8a16` | FSRS v4/v5 scheduling algorithms, weight optimizer, simulation, and interval calculations. |
| `area:editor` | Pastel Blue | `#bfdadc` | Upstream Anki note editor, rich text editing, Cloze syntax, and MathJax/LaTeX rendering. |
| `area:browser` | Sky Blue | `#1f883d` | Card Browser, table virtualization, search syntax parsing, and bulk actions. |
| `area:mobile` | Coral | `#e99695` | Smartphone responsive layout, touch navigation (44px targets), safe area insets, and drawer. |
| `area:pwa` | Orange | `#f9d0c4` | Progressive Web App manifest, service worker caching, and standalone offline behavior. |
| `area:e2e` | Mint | `#c2e0c6` | Playwright end-to-end integration test suite and visual screenshot regression harness. |
| `area:docker` | Dark Gray | `#24292e` | Dockerfile multi-stage builds, `docker-compose.yml`, GHCR packaging, and container healthchecks. |

---

## 5. Priority Labels (`priority:*`)

| Label | Color | Hex | Criteria |
| :--- | :--- | :--- | :--- |
| `priority:critical` | Dark Crimson | `#b60205` | Severe data loss risk, collection corruption, or complete service outage. Requires immediate fix. |
| `priority:high` | Red-Orange | `#d93f0b` | Broken core workflow (e.g. reviews cannot be submitted, sync fails completely). Top scheduling priority. |
| `priority:medium` | Amber | `#fbca04` | Non-blocking bug, UI glitch, or standard feature enhancement for next planned release. |
| `priority:low` | Soft Blue | `#c5def5` | Minor aesthetic flaw, obscure edge case, or low-urgency cosmetic refactoring. |

---

## 🚀 Setting Up Labels via GitHub CLI (`gh`)

Repository administrators can automatically create or update all standard labels using the GitHub CLI:

```bash
# Type labels
gh label create "bug" --color "d73a4a" --description "Something isn't working as expected" --force
gh label create "enhancement" --color "a2eeef" --description "New feature or parity request" --force
gh label create "documentation" --color "0075ca" --description "Improvements to documentation or guides" --force
gh label create "maintenance" --color "fbca04" --description "Tooling, dependencies, or refactoring" --force
gh label create "security" --color "ee0701" --description "Security vulnerability or flaw" --force
gh label create "performance" --color "5319e7" --description "Performance optimizations" --force

# Community
gh label create "good first issue" --color "7057ff" --description "Good for newcomers" --force
gh label create "help wanted" --color "008672" --description "Extra attention or assistance needed" --force

# Triage & Status
gh label create "triage" --color "ededed" --description "Awaiting maintainer triage and review" --force
gh label create "confirmed" --color "0e8a16" --description "Reproduced bug or approved proposal" --force
gh label create "blocked" --color "000000" --description "Blocked by dependency or upstream issue" --force
gh label create "duplicate" --color "cfd3d7" --description "Issue already exists" --force
gh label create "invalid" --color "e4e669" --description "Not reproducible or out of scope" --force
gh label create "wontfix" --color "ffffff" --description "Will not be addressed or implemented" --force

# Area
gh label create "area:server" --color "1d76db" --description "Rust backend, RPC, auth, SQLite" --force
gh label create "area:web" --color "5319e7" --description "Svelte 5 web shell and styles" --force
gh label create "area:sync" --color "006b75" --description "AnkiDroid/AnkiMobile sync server" --force
gh label create "area:fsrs" --color "0e8a16" --description "FSRS scheduler & simulation" --force
gh label create "area:editor" --color "bfdadc" --description "Editor iframe, rich text, cloze" --force
gh label create "area:browser" --color "1f883d" --description "Card Browser & search syntax" --force
gh label create "area:mobile" --color "e99695" --description "Smartphone responsive layout" --force
gh label create "area:pwa" --color "f9d0c4" --description "PWA manifest & service worker" --force
gh label create "area:e2e" --color "c2e0c6" --description "Playwright E2E integration tests" --force
gh label create "area:docker" --color "24292e" --description "Docker, Compose & container packaging" --force

# Priority
gh label create "priority:critical" --color "b60205" --description "Severe data loss or crash" --force
gh label create "priority:high" --color "d93f0b" --description "High impact on core user workflows" --force
gh label create "priority:medium" --color "fbca04" --description "Standard priority enhancement or bug" --force
gh label create "priority:low" --color "c5def5" --description "Low priority or minor polish" --force
```
