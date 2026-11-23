# Cardly Brand Identity & Design Guide

> **Cardly** is an independent, open-source, pixel-perfect web and mobile application powered directly by Anki's official Rust core (`rslib`). This guide documents the brand identity, color system, typography, vector assets, and badge snippets.

---

## 🎨 Brand Concept & Emblem

The Cardly emblem represents the science of human memory:
- **Layered Flashcards**: Two offset geometric cards symbolize the active recall process and the progression through spaced intervals.
- **Electric Sapphire Front Face**: Represents modern software architecture, stability, and speed.
- **Mnemonic Spark (4-point Star)**: Floating in glowing gold (`#F59E0B`), the spark symbolizes memory retention, neural recall, and the optimization of learning intervals powered by FSRS.

---

## 📁 Vector Brand Assets

All production vector and raster brand assets are located in `web/public/` and `docs/brand/`:

| Asset File | Format & Size | Purpose |
| :--- | :--- | :--- |
| `web/public/cardly-icon.svg` | Vector (512×512 viewBox) | Primary square app emblem with layered cards & mnemonic spark |
| `web/public/cardly-logo.svg` | Vector (320×72 viewBox) | Horizontal lockup: Emblem + "Cardly" wordmark + "ANKI-CORE" badge |
| `web/public/icon-maskable.svg` | Vector (512×512 full-bleed) | PWA maskable icon with 80% safe zone for Android squircle/circle masks |
| `web/public/favicon.ico` | Multi-res ICO (16/32/48) | Desktop web browser tab icon |
| `web/public/favicon-16x16.png` | PNG (16×16) | Standard favicon |
| `web/public/favicon-32x32.png` | PNG (32×32) | High-DPI browser favicon |
| `web/public/apple-touch-icon.png` | PNG (180×180) | iOS Safari "Add to Home Screen" webclip icon |
| `web/public/icon-192.png` | PNG (192×192) | PWA standard home screen icon |
| `web/public/icon-512.png` | PNG (512×512) | PWA splash screen & app stores |
| `web/public/icon-maskable-192.png` | PNG (192×192) | PWA adaptive maskable icon |
| `web/public/icon-maskable-512.png` | PNG (512×512) | PWA high-res adaptive maskable icon |
| `docs/brand/social-preview.svg` | Vector (1280×640) | Source vector for social media cards and OpenGraph images |
| `web/public/social-preview.png` | PNG (1280×640) | Served directly via web server for social meta tags |

---

## 🌈 Color Palette & Design Tokens

Cardly's color tokens strictly follow Desktop Anki's design system (`web/src/styles/anki-tokens.css`) across light and dark modes:

### 1. Primary Brand Colors
| Token Name | Hex Code | Role |
| :--- | :--- | :--- |
| **Cardly Sapphire** | `#2563EB` | Primary brand accent, primary CTA buttons, active states |
| **Cardly Light Cobalt** | `#3B82F6` | Hover states, focus rings, link highlights |
| **Cardly Deep Navy** | `#1D4ED8` | Pressed states, active navigation |
| **Dark Obsidian** | `#0B0F19` | Editorial hero canvas, social banners |
| **FSRS Gold** | `#F59E0B` | Mnemonic spark, FSRS optimizer accents, retention metrics |
| **FSRS Light Amber** | `#FBBF24` | Spark secondary glow, highlighted intervals |

### 2. Reviewer Rating Buttons (Ease Ratings)
| Rating | Base Color | Dark Surface | Light Surface | Function |
| :--- | :--- | :--- | :--- | :--- |
| **Again (1)** | `#EF4444` (Red) | `#3B1219` | `#FEE2E2` | Lapsed card, repeat in current learning step |
| **Hard (2)** | `#F97316` (Orange) | `#381E11` | `#FFEDD5` | Difficult recall, reduced interval |
| **Good (3)** | `#10B981` (Green) | `#064E3B` | `#DCFCE7` | Successful recall, standard scheduled interval |
| **Easy (4)** | `#0284C7` (Cyan/Blue) | `#082F49` | `#E0F2FE` | Effortless recall, bonus interval multiplier |

### 3. Study Queue & Card State Tokens
| Token Variable | Hex Code | Meaning |
| :--- | :--- | :--- |
| `--state-new` | `#3B82F6` | Unseen cards ready to be introduced |
| `--state-learn` | `#DC2626` | Cards currently in the learning / relearning step |
| `--state-review` | `#16A34A` | Mature cards due for spaced review |
| `--state-buried` | `#F59E0B` | Sibling or user buried cards (hidden until tomorrow) |
| `--state-suspended` | `#FACC15` | Suspended cards taken out of rotation |
| `--state-marked` | `#6366F1` | Marked notes tagged with `*` |

### 4. Card Browser Flags (1–7)
| Flag | Token Variable | Hex Value |
| :--- | :--- | :--- |
| **Flag 1 (Red)** | `--flag-1` | `#EF4444` |
| **Flag 2 (Orange)** | `--flag-2` | `#FB923C` |
| **Flag 3 (Green)** | `--flag-3` | `#4ADE80` |
| **Flag 4 (Blue)** | `--flag-4` | `#3B82F6` |
| **Flag 5 (Pink)** | `--flag-5` | `#E879F9` |
| **Flag 6 (Turquoise)** | `--flag-6` | `#2DD4BF` |
| **Flag 7 (Purple)** | `--flag-7` | `#A855F7` |

### 5. UI Canvas & Surfaces
| Surface Token | Light Theme | Dark Theme (`.night-mode`) |
| :--- | :--- | :--- |
| `--canvas` (Base app background) | `#F5F5F5` | `#2C2C2C` |
| `--canvas-elevated` (Cards & dialogs) | `#FFFFFF` | `#363636` |
| `--canvas-inset` (Tables & inputs) | `#FFFFFF` | `#252525` |
| `--fg` (Primary text) | `#020202` | `#FCFCFC` |
| `--fg-subtle` (Muted labels) | `#737373` | `#9A9A9A` |
| `--border` (Window & panel borders) | `#C4C4C4` | `#202020` |

---

## 🔤 Typography Guidelines

Cardly uses the operating system's native font stack for zero-latency rendering, optimal memory footprint, and native OS feel:

### Body & User Interface
```css
font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
```
- **Regular (400)**: Card body text, field labels, metadata.
- **Medium (500)**: Table column headers, button labels, tabs.
- **Semi-Bold (600)**: Deck list headers, modal titles, menu bar headers.
- **Extra-Bold (800)**: "Cardly" wordmark in branding assets.

### Monospace (Templates, MathJax & Code)
```css
font-family: ui-monospace, "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
```
- Used in Card Templates editor, HTML source modal, LaTeX/MathJax raw inputs, and developer logs.

---

## 📐 Logo Usage Rules

### Do:
- Use `web/public/cardly-icon.svg` as the primary square emblem.
- Ensure clear margin space around the emblem equal to at least 25% of the emblem's height.
- Use `web/public/icon-maskable.svg` for any Android or adaptive app icon requirements.
- Use the dark background version (`web/public/social-preview.png`) for social media, marketing cards, and repository headers.

### Don't:
- Do not stretch, warp, or alter the aspect ratio of the flashcard geometry.
- Do not remove the gold mnemonic spark.
- Do not use official Anki logos, star emblems, or trademarks in Cardly interfaces.
- Do not change the primary brand blue (`#2563EB`) to arbitrary unapproved shades.

---

## 🏷️ Ready-to-Use Markdown Badges

Copy and paste these badges into READMEs, documentation, or external articles:

### Primary Project Badges
```markdown
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](./LICENSE)
[![Anki Upstream](https://img.shields.io/badge/Anki_Core-ankitects%2Fanki-orange.svg)](https://github.com/ankitects/anki)
[![FSRS Enabled](https://img.shields.io/badge/FSRS-v4%20%26%20v5-amber.svg)](https://github.com/open-spaced-repetition/fsrs4anki)
[![Docker Ready](https://img.shields.io/badge/Docker-Quickstart-2496ED.svg?logo=docker&logoColor=white)](./docker-compose.yml)
[![PWA Installable](https://img.shields.io/badge/PWA-Installable-purple.svg?logo=pwa&logoColor=white)]()
[![Tests](https://img.shields.io/badge/Tests-Rust%20%26%20Playwright%20Passing-brightgreen.svg)]()
```

### Preview:
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](./LICENSE)
[![Anki Upstream](https://img.shields.io/badge/Anki_Core-ankitects%2Fanki-orange.svg)](https://github.com/ankitects/anki)
[![FSRS Enabled](https://img.shields.io/badge/FSRS-v4%20%26%20v5-amber.svg)](https://github.com/open-spaced-repetition/fsrs4anki)
[![Docker Ready](https://img.shields.io/badge/Docker-Quickstart-2496ED.svg?logo=docker&logoColor=white)](./docker-compose.yml)
[![PWA Installable](https://img.shields.io/badge/PWA-Installable-purple.svg?logo=pwa&logoColor=white)]()
