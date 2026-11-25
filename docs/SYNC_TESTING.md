# Mobile Sync Testing Checklist (AnkiDroid & AnkiMobile)

Cardly uses upstream Anki's own HTTP sync protocol (`/sync/*` and `/msync/*`, verified with Anki Desktop). AnkiDroid and AnkiMobile have **not yet been verified end-to-end** on physical devices—use this checklist to test and report results.

## Prerequisites

- Cardly running behind HTTPS (required by iOS / recommended for Android; e.g., Caddy, Nginx, or Tailscale Serve)
- A test user account (`CARDLY_USER1=alice:password`), not your primary collection
- A backup of any existing mobile collection before switching sync servers

## 1. AnkiDroid (Android)

1. Open **AnkiDroid** → **Settings** → **Syncing** → **Custom sync server**.
2. Enable **Use custom sync server**.
3. Set **Sync URL** to `https://<YOUR_DOMAIN>/sync/` (or `http://<LAN_IP>:8080/sync/` on trusted local LAN).
4. Set **Media sync URL** to `https://<YOUR_DOMAIN>/msync/`.
5. Tap **Sync**, enter your Cardly credentials, and verify:
   - [ ] Initial upload or download completes without HTTP/TLS errors
   - [ ] Deck counts and FSRS-6 / FSRS-5 parameters match Cardly web
   - [ ] Card media (images/audio) syncs in both directions
   - [ ] Reviewing a card on AnkiDroid and syncing updates due counts in Cardly (and vice versa)

## 2. AnkiMobile (iOS)

1. Open **AnkiMobile** → **Preferences** → **Syncing** → **Custom Server**.
2. Set **Sync URL** to `https://<YOUR_DOMAIN>/sync/`.
3. Enter your Cardly credentials and tap **Sync Now**, then verify:
   - [ ] Initial collection sync completes
   - [ ] Media files sync without errors
   - [ ] Bidirectional review sync updates due counts and FSRS memory state

## Reporting Results

Please open a GitHub Issue with your **AnkiDroid / AnkiMobile version**, **Android / iOS version**, whether you used HTTPS or HTTP, and any error message from Cardly server logs (`RUST_LOG=cardly_server=debug,anki=info`).
