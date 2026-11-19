# Anki Official Forums Launch Post

- **Forum**: [Official Anki Forums](https://forums.ankiweb.net/)
- **Category**: **Third-Party Apps** / **Development**
- **Topic Title**: Cardly: An open-source, self-hosted web client & sync server powered directly by official `rslib` (AGPL-3.0)

---

## Forum Post Body

Hello Damien and fellow Anki community members,

First and foremost, I want to express my deepest gratitude to Damien Elmes, the Ankitects team, and all the contributors who have made Anki the gold standard of spaced repetition for nearly two decades. The discipline, architectural elegance, and open-source principles of the Anki project are truly inspiring.

I am writing to share **Cardly** (https://cardly.app), an independent, open-source project created to solve a specific challenge: providing a full-featured, self-hostable web interface that maintains 100% fidelity to official Anki scheduling and collections without forking or reimplementing Anki's algorithms.

### Architecture & Design Decisions

When designing Cardly, our core priority was zero divergence from upstream Anki:

1. **Direct `rslib` Linking**:
   Cardly's backend server is written in Rust (using Axum) and directly links against Anki's official `rslib` crate without modifications. Every review interval, stability calculation, FSRS formula, and SQLite database migration is performed directly by `rslib`.

2. **Server-Mode Collection (`server: true`) & Native Sync**:
   By running `rslib`'s `Backend::new(tr, true)`, Cardly writes server-side USNs for all browser edits. Mounted on the same Axum process are Anki's upstream `SyncProtocol` and `MediaSyncProtocol` (`/sync/*` and `/msync/*`), allowing users to synchronize their AnkiDroid (Android) or AnkiMobile (iOS) apps directly against their Cardly collection.

3. **Reused SvelteKit Assets**:
   Cardly embeds the compiled upstream SvelteKit routes for complex screens (such as `/graphs` statistics and `/deck-options` with FSRS parameters, weight optimizer, and simulator), ensuring exact behavioral and visual alignment with Anki Desktop.

4. **Security & Sandboxed Card JavaScript**:
   Because community decks often contain custom JavaScript, reviewer cards are rendered inside isolated `srcdoc` iframes under strict Content Security Policies (`UNTRUSTED_MEDIA_CSP`). All backend mutation RPCs require a top-level session token (`X-Cardly-Api-Token`), ensuring untrusted card scripts cannot compromise user sessions or data.

### Trademark & Licensing Compliance

- **Trademark Integrity**: Cardly does not use the Anki name or the official star logo in its user interface, app icons, or distribution assets. It is branded independently as "Cardly".
- **GNU AGPL-3.0**: Cardly is released strictly under the GNU Affero General Public License v3 or later. Full attribution is documented in our `NOTICE` file, referencing Damien Elmes and the contributors listed in `anki-upstream/CONTRIBUTORS`.

### Links & Resources

- **GitHub Repository**: https://github.com/ankitects/anki
- **Live Demo Instance**: https://cardly.app
- **Feature Parity Checklist**: https://github.com/ankitects/anki/blob/main/docs/PARITY.md

We would be honored to receive your feedback, protocol insights, and suggestions for ensuring long-term compatibility with upcoming Anki releases. Thank you again for everything you do for the spaced repetition community!
