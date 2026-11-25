<script lang="ts">
  import {
    checkDatabase,
    checkMedia,
    emptyTrash,
    getEmptyCards,
    removeCards,
    restoreTrash,
    trashMediaFiles,
  } from "@generated/backend";
  import type { EmptyCardsReport } from "@generated/anki/card_rendering_pb";
  import type { CheckMediaResponse } from "@generated/anki/media_pb";
  import { pluginManager } from "../lib/plugins";
  import { store } from "../lib/store.svelte";

  // Check Database
  let dbLoading = $state(false);
  let dbProblems = $state<string[] | null>(null);

  // Check Media
  let mediaLoading = $state(false);
  let mediaReport = $state<CheckMediaResponse | null>(null);

  // Empty Cards
  let emptyLoading = $state(false);
  let emptyReport = $state<EmptyCardsReport | null>(null);
  let keepNotes = $state(true);

  async function runCheckDb() {
    dbLoading = true;
    dbProblems = null;
    try {
      const res = await checkDatabase({});
      dbProblems = res.problems;
      store.bumpOpRevision();
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Database check failed", "error");
    } finally {
      dbLoading = false;
    }
  }

  async function runCheckMedia() {
    mediaLoading = true;
    mediaReport = null;
    try {
      mediaReport = await checkMedia({});
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Media check failed", "error");
    } finally {
      mediaLoading = false;
    }
  }

  async function handleTrashUnused() {
    if (!mediaReport || mediaReport.unused.length === 0) return;
    try {
      await trashMediaFiles({ fnames: mediaReport.unused });
      store.toast(`Moved ${mediaReport.unused.length} unused file(s) to trash`);
      await runCheckMedia();
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to trash files", "error");
    }
  }

  async function handleEmptyTrash() {
    const ok = await store.confirm("Permanently delete all files in media trash?", "Empty Trash", {
      danger: true,
      confirmLabel: "Empty Trash",
    });
    if (!ok) return;
    try {
      await emptyTrash({});
      store.toast("Media trash emptied");
      await runCheckMedia();
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to empty trash", "error");
    }
  }

  async function handleRestoreTrash() {
    try {
      await restoreTrash({});
      store.toast("Restored files from media trash");
      await runCheckMedia();
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to restore trash", "error");
    }
  }

  async function runEmptyCards() {
    emptyLoading = true;
    emptyReport = null;
    try {
      emptyReport = await getEmptyCards({});
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Empty cards scan failed", "error");
    } finally {
      emptyLoading = false;
    }
  }

  async function deleteEmptyCards() {
    if (!emptyReport) return;
    const ids: bigint[] = [];
    for (const n of emptyReport.notes) {
      if (keepNotes && n.willDeleteNote) continue;
      ids.push(...n.cardIds);
    }
    if (ids.length === 0) {
      store.toast("No empty cards selected for deletion");
      return;
    }
    try {
      const res = await removeCards({ cardIds: ids });
      store.bumpOpRevision();
      store.toast(`Deleted ${res.count} empty card(s)`);
      store.emptyCardsOpen = false;
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to delete empty cards", "error");
    }
  }

  $effect(() => {
    if (store.checkDbOpen) void runCheckDb();
  });

  $effect(() => {
    if (store.checkMediaOpen) void runCheckMedia();
  });

  $effect(() => {
    if (store.emptyCardsOpen) void runEmptyCards();
  });
</script>

{#if store.checkDbOpen}
  <div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Check Database">
    <div class="modal-window maint-window">
      <div class="modal-header">
        <h3 class="modal-title">Check Database</h3>
        <button class="modal-close" onclick={() => (store.checkDbOpen = false)}>×</button>
      </div>
      <div class="modal-body">
        {#if dbLoading}
          <div class="muted">Checking and optimizing database...</div>
        {:else if dbProblems}
          {#if dbProblems.length === 0}
            <p class="ok-msg">Database rebuilt and optimized. No problems found.</p>
          {:else}
            <p>The following issues were found and repaired:</p>
            <ul class="report-list">
              {#each dbProblems as p}
                <li>{p}</li>
              {/each}
            </ul>
          {/if}
        {/if}
      </div>
      <div class="modal-footer">
        <button class="anki-btn primary" onclick={() => (store.checkDbOpen = false)}>Close</button>
      </div>
    </div>
  </div>
{/if}

{#if store.checkMediaOpen}
  <div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Check Media">
    <div class="modal-window maint-window wide">
      <div class="modal-header">
        <h3 class="modal-title">Check Media</h3>
        <button class="modal-close" onclick={() => (store.checkMediaOpen = false)}>×</button>
      </div>
      <div class="modal-body">
        {#if mediaLoading}
          <div class="muted">Scanning media folder and note references...</div>
        {:else if mediaReport}
          <pre class="report-pre">{mediaReport.report}</pre>
          <div class="media-stats">
            <span>Missing: <strong>{mediaReport.missing.length}</strong></span>
            <span>Unused: <strong>{mediaReport.unused.length}</strong></span>
            <span>Trash: <strong>{mediaReport.haveTrash ? "Has files" : "Empty"}</strong></span>
          </div>
          {#if mediaReport.missing.length > 0}
            <details class="media-details">
              <summary>Missing files ({mediaReport.missing.length})</summary>
              <ul class="file-list">
                {#each mediaReport.missing.slice(0, 200) as f}<li>{f}</li>{/each}
              </ul>
            </details>
          {/if}
          {#if mediaReport.unused.length > 0}
            <details class="media-details">
              <summary>Unused files ({mediaReport.unused.length})</summary>
              <ul class="file-list">
                {#each mediaReport.unused.slice(0, 200) as f}<li>{f}</li>{/each}
              </ul>
            </details>
          {/if}
        {/if}
      </div>
      <div class="modal-footer between">
        <div class="left-btns">
          {#if mediaReport && mediaReport.unused.length > 0}
            <button class="anki-btn danger" onclick={handleTrashUnused}>Delete Unused</button>
          {/if}
          {#if mediaReport?.haveTrash}
            <button class="anki-btn" onclick={handleRestoreTrash}>Restore Trash</button>
            <button class="anki-btn danger" onclick={handleEmptyTrash}>Empty Trash</button>
          {/if}
        </div>
        <button class="anki-btn primary" onclick={() => (store.checkMediaOpen = false)}>Close</button>
      </div>
    </div>
  </div>
{/if}

{#if store.emptyCardsOpen}
  <div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Empty Cards">
    <div class="modal-window maint-window wide">
      <div class="modal-header">
        <h3 class="modal-title">Empty Cards</h3>
        <button class="modal-close" onclick={() => (store.emptyCardsOpen = false)}>×</button>
      </div>
      <div class="modal-body">
        {#if emptyLoading}
          <div class="muted">Searching for empty cards...</div>
        {:else if emptyReport}
          {#if emptyReport.notes.length === 0}
            <p class="ok-msg">No empty cards found.</p>
          {:else}
            <pre class="report-pre">{emptyReport.report}</pre>
            <label class="check-row">
              <input type="checkbox" bind:checked={keepNotes} />
              Keep notes with no valid cards (only delete extra empty cards)
            </label>
          {/if}
        {/if}
      </div>
      <div class="modal-footer">
        <button class="anki-btn" onclick={() => (store.emptyCardsOpen = false)}>Close</button>
        {#if emptyReport && emptyReport.notes.length > 0}
          <button class="anki-btn danger" onclick={deleteEmptyCards}>Delete</button>
        {/if}
      </div>
    </div>
  </div>
{/if}

{#if store.syncInfoOpen}
  <div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Sync">
    <div class="modal-window maint-window">
      <div class="modal-header">
        <h3 class="modal-title">Sync Status</h3>
        <button class="modal-close" onclick={() => (store.syncInfoOpen = false)}>×</button>
      </div>
      <div class="modal-body">
        <p class="sync-p">
          Your collection is saved directly on the Cardly server in real time as account
          <strong>{store.username}</strong>.
        </p>
        <p class="sync-p muted">
          To sync AnkiDroid, AnkiMobile, or Desktop clients with this server, set your custom sync server URL to:
        </p>
        <div class="sync-url-box">{window.location.origin}</div>
      </div>
      <div class="modal-footer">
        <button
          class="anki-btn"
          onclick={() => {
            store.syncInfoOpen = false;
            store.preferencesOpen = true;
          }}
        >
          Sync Settings...
        </button>
        <button
          class="anki-btn primary"
          onclick={() => {
            store.bumpOpRevision();
            pluginManager.emit("sync:completed", {
              source: "manual-refresh",
              timestamp: Date.now(),
            });
            store.syncInfoOpen = false;
            store.toast("Collection refreshed");
          }}
        >
          Refresh Now
        </button>
      </div>
    </div>
  </div>
{/if}

{#if store.aboutOpen}
  <div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="About Cardly">
    <div class="modal-window maint-window">
      <div class="modal-header">
        <h3 class="modal-title">About Cardly</h3>
        <button class="modal-close" onclick={() => (store.aboutOpen = false)}>×</button>
      </div>
      <div class="modal-body about-body">
        <div class="about-title">Cardly</div>
        <p class="about-sub">Self-hosted web spaced-repetition client</p>
        <p class="about-legal">
          Licensed under the <strong>GNU Affero General Public License v3.0 or later (AGPL-3.0-or-later)</strong>.
        </p>
        <p class="about-legal muted">
          Powered by the open-source spaced-repetition engine (rslib, FSRS, template engine, SQLite storage, and
          upstream web components). Source code and license notices are available with this installation per AGPL-3.0
          section 13.
        </p>
      </div>
      <div class="modal-footer">
        <button class="anki-btn primary" onclick={() => (store.aboutOpen = false)}>Close</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .maint-window {
    width: 480px;
    max-width: 94vw;
  }
  .maint-window.wide {
    width: 600px;
  }
  .ok-msg {
    margin: 4px 0;
    font-size: 14px;
    color: var(--stat-review);
    font-weight: 500;
  }
  .report-list {
    margin: 8px 0;
    padding-left: 20px;
    font-size: 13px;
  }
  .report-pre {
    background: var(--canvas-inset);
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    padding: 10px 12px;
    font-family: var(--font-mono);
    font-size: 12px;
    white-space: pre-wrap;
    max-height: 240px;
    overflow-y: auto;
    margin: 0 0 10px;
  }
  .media-stats {
    display: flex;
    gap: 16px;
    font-size: 13px;
    margin-bottom: 8px;
  }
  .media-details {
    margin-top: 6px;
    font-size: 12px;
  }
  .file-list {
    max-height: 120px;
    overflow-y: auto;
    font-family: var(--font-mono);
    padding-left: 18px;
    margin: 4px 0;
  }
  .between {
    justify-content: space-between;
  }
  .left-btns {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .check-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    cursor: pointer;
  }
  .sync-p {
    margin: 0 0 10px;
    font-size: 13px;
    line-height: 1.5;
  }
  .sync-url-box {
    font-family: var(--font-mono);
    font-size: 13px;
    padding: 8px 10px;
    background: var(--canvas-code);
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    word-break: break-all;
  }
  .about-body {
    text-align: center;
    padding: 20px;
  }
  .about-title {
    font-size: 24px;
    font-weight: 700;
    margin-bottom: 4px;
  }
  .about-sub {
    margin: 0 0 14px;
    font-size: 13px;
    color: var(--fg-subtle);
  }
  .about-legal {
    font-size: 12px;
    line-height: 1.55;
    margin: 8px 0;
  }
  .muted {
    color: var(--fg-subtle);
  }
</style>
