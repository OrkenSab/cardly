# Upgrading `anki-upstream`

Cardly links directly against Anki's upstream Rust libraries (`anki-upstream/rslib*`) and embeds upstream web assets (`ts/routes/*`). Never edit files inside `anki-upstream/` directly.

## 1. Choosing a Target Release Tag

1. Check stable releases at <https://github.com/ankitects/anki/releases>.
2. **Always pick a stable release tag** (for example `26.09.4`). Avoid `-rc`, `b1`/`b2`, or arbitrary `main` commits.
3. Review the Anki release notes for:
   - `rslib` Rust API or protobuf changes
   - Collection schema / database version bumps (check if older Anki clients can still open the collection)
   - Sync protocol changes (`SYNC_VERSION_MAX`)

## 2. Advice for Self-Hosters Before Upgrading

Back up your entire `CARDLY_DATA_DIR` (or Docker volume `cardly_data`) before pulling a new image:

```bash
docker compose down
tar -czf cardly-backup-$(date +%F).tar.gz ./data
```

If an upstream Anki release bumps the collection schema version, older Anki Desktop / AnkiDroid / AnkiMobile clients may require an upgrade before they can open or sync the updated collection.

## 3. Bumping the Submodule

```bash
git checkout -b chore/bump-anki-<version>
git -C anki-upstream fetch --tags
git -C anki-upstream checkout <version>
git add anki-upstream
```

## 4. Rebuilding Assets & Server

```bash
# Rebuild upstream web bundles + protobuf descriptors
cd anki-upstream
./ninja sveltekit qt:aqt:data:web rslib
cd ..

# Update Cargo.lock if needed and run Rust tests
cargo test --manifest-path server/Cargo.toml

# Rebuild Cardly web shell and run typecheck
npm --prefix web ci
npm --prefix web run check
npm --prefix web run build
```

## 5. What to Test

- `cargo test --manifest-path server/Cargo.toml` (including `fsrs6_params_and_backward_compat_work`)
- `cd e2e && npx playwright test` (including `mobile-layout.spec.ts` and `cardly.spec.ts`)
- Smoke-test embedded upstream pages in the browser: Add/Edit (`/editor`), Deck Options (`/deck-options/1`), Statistics (`/graphs`)
- Verify Docker build: `docker build -t cardly:test .`
- Update `README.md`, `docs/i18n/README.*.md`, `docs/PARITY.md`, `landing/`, `web/index.html`, and `CHANGELOG.md` with the new Anki version and FSRS version.

## 6. Rollback

If an upgrade introduces issues:

```bash
git checkout main -- anki-upstream server/Cargo.lock
git submodule update --init --recursive
```

Self-hosters can restore from their pre-upgrade `cardly-backup-*.tar.gz` archive and pin the previous Docker image tag.
