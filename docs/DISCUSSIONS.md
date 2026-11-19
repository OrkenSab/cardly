# GitHub Discussions Guide & Pinned Discussion Templates

This document provides ready-to-use templates and setup recommendations for **GitHub Discussions** in the Cardly repository.

---

## 🛠️ Recommended GitHub Discussions Setup

To enable discussions on GitHub:
1. Navigate to **Settings** -> **General** -> scroll down to **Features** -> check **Discussions**.
2. Go to the new **Discussions** tab -> click **Categories** (gear icon) to configure:

| Category | Format | Description |
| :--- | :--- | :--- |
| 🚀 **Ideas & Feature Requests** | Open discussion (Voting enabled) | Propose new capabilities, UI improvements, and vote on what to build next. |
| 🎨 **Show and Tell** | Open discussion | Share your custom card styles, CSS themes, note templates, and study setups. |
| 💬 **Q&A & Self-Hosting** | Question / Answer | Troubleshooting Docker setups, reverse proxies (Caddy/Traefik), TLS, and mobile sync. |
| 📢 **Announcements** | Announcement (Staff only) | Official release notes, milestone progress, and project updates. |
| 🌐 **Localizations & Decks** | Open discussion | Community-contributed translations and starter deck sharing (languages, sciences, exams). |

---

## 📌 Pinned Discussion #1: Welcome & Feature Requests (Copy & Paste)

*Title:* **👋 Welcome to Cardly! What features do you want to see next? (Vote & Discuss)**  
*Category:* 🚀 **Ideas & Feature Requests** (Pin this discussion to the top)

```markdown
# 👋 Welcome to the Cardly Community!

Thank you for checking out **Cardly**! Whether you are here to self-host a private instance, replace your desktop client, or sync your cards across devices, we're glad to have you.

Cardly was built with a clear philosophy:
1. **100% Upstream Anki Compatibility:** Powered by Anki's official Rust core (`rslib`) and SQLite schema.
2. **True Self-Hosting:** Lightweight Docker container, zero telemetry, zero forced cloud lock-in.
3. **Modern Web Speed:** Pixel-accurate desktop interface in Svelte 5 with responsive mobile PWA and multimodal AI card generation.

---

### 🗳️ What should we build next?

We want our development priorities to be driven by real users. Take a look at our [Product Roadmap](https://github.com/cardly-app/cardly/blob/main/docs/ROADMAP.md) and let us know what matters most to you!

Here are some candidates currently under consideration for upcoming milestones:

- 🎙️ **In-browser Audio Recorder & Waveform Editor** (Record pronunciations directly from the browser card editor)
- 🖼️ **Visual Image Occlusion Web Editor** (Mask anatomy diagrams and formulas with interactive boxes)
- 👥 **Shared Decks & Read-Only Deck Subscriptions** (Subscribe to a peer's deck with automatic updates without affecting your own review history)
- 🧩 **Sandboxed Web Add-on API** (Run lightweight community extensions in safe Web Workers)
- 🎨 **Community Card Themes & Template Gallery** (One-click themes: Notion, Obsidian Dark, Academic, Minimalist)
- 💾 **Client-Side WASM Core (Offline Web)** (Run Cardly 100% in-browser with zero backend required via OPFS SQLite)
- 📱 **Native Desktop (Tauri) and Mobile (Android APK) Shells**

---

### 💬 How to participate:

1. **Vote on existing comments below** using the 👍 emoji to help us gauge demand.
2. **Leave a comment** describing your specific use case, workflow, or pain points with existing flashcard tools.
3. If you have an entirely new concept, feel free to open a new discussion under the **Ideas** category!

Let's make spaced repetition better together. Happy studying! 🚀
```

---

## 📌 Pinned Discussion #2: Show & Tell (Copy & Paste)

*Title:* **🎨 Show and Tell: Share your custom card themes, CSS styling, and workflows!**  
*Category:* 🎨 **Show and Tell** (Pin this discussion)

```markdown
# 🎨 Show and Tell: Decks, CSS Styling & Workflows

One of the best things about Anki has always been the power of HTML/CSS card templates. In Cardly, every card renders exactly as it does on desktop, but with the full flexibility of the modern web.

### 🌟 What to share here:
- **Card Templates & CSS:** Share screenshots and CSS snippets of your favorite card layouts (minimalist dark mode, vocabulary cards with pitch accent colors, code syntax highlighting).
- **Study Workflows:** How do you organize your decks, tags, and FSRS retention targets?
- **AI Prompts:** How are you using Cardly's Multimodal AI Generator? What prompts produce the best cards for your textbooks or notes?
- **Self-Hosting Setups:** Share your Docker Compose setups, reverse proxy configs, or Kubernetes manifests.

Drop your screenshots, code snippets, and configs below! 👇
```

---

## 📌 Discussion Template for GitHub (`.github/DISCUSSION_TEMPLATE/ideas.yml`)

Cardly also includes a structured template for community feature proposals in `.github/DISCUSSION_TEMPLATE/ideas.yml`.
Users opening new discussions in the **Ideas** category will be guided to provide clear motivation, proposed solutions, and alternatives considered.
