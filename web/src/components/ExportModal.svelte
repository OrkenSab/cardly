<script lang="ts">
  import { getDeckNames } from "@generated/backend";
  import type { DeckNameId } from "@generated/anki/decks_pb";
  import { downloadExport, type ExportOptionsPayload } from "../lib/api";
  import { store } from "../lib/store.svelte";

  let decks = $state<DeckNameId[]>([]);
  let format = $state<ExportOptionsPayload["format"]>("apkg");
  let selectedDeckId = $state<string>("all");
  let includeScheduling = $state(true);
  let includeDeckConfigs = $state(false);
  let includeMedia = $state(true);
  let legacy = $state(false);
  let includeHtml = $state(true);
  let includeTags = $state(true);
  let includeDeck = $state(false);
  let includeNotetype = $state(false);
  let includeGuid = $state(false);
  let exporting = $state(false);

  const hasSelectedNotes = $derived((store.exportModal.noteIds?.length ?? 0) > 0);

  async function load() {
    try {
      const res = await getDeckNames({ skipEmptyDefault: false, includeFiltered: false });
      decks = res.entries;
      if (hasSelectedNotes) {
        selectedDeckId = "selected";
      } else if (store.exportModal.deckId) {
        selectedDeckId = String(store.exportModal.deckId);
      } else {
        selectedDeckId = "all";
      }
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to load decks", "error");
    }
  }

  $effect(() => {
    if (store.exportModal.open) {
      void load();
    }
  });

  async function doExport() {
    exporting = true;
    try {
      const payload: ExportOptionsPayload = {
        format,
        include_scheduling: includeScheduling,
        include_deck_configs: includeDeckConfigs,
        include_media: includeMedia,
        legacy,
        include_html: includeHtml,
        include_tags: includeTags,
        include_deck: includeDeck,
        include_notetype: includeNotetype,
        include_guid: includeGuid,
      };
      if (format !== "colpkg") {
        if (selectedDeckId === "selected" && store.exportModal.noteIds?.length) {
          payload.note_ids = store.exportModal.noteIds;
        } else if (selectedDeckId !== "all" && selectedDeckId !== "selected") {
          payload.deck_id = Number(selectedDeckId);
        }
      }
      await downloadExport(payload);
      store.toast("Export downloaded");
      close();
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Export failed", "error");
    } finally {
      exporting = false;
    }
  }

  function close() {
    store.exportModal = { open: false };
  }
</script>

<div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Export">
  <div class="modal-window exp-window">
    <div class="modal-header">
      <h3 class="modal-title">Export</h3>
      <button class="modal-close" onclick={close}>×</button>
    </div>
    <div class="modal-body">
      <div class="form-row">
        <label for="exp-fmt">Export format:</label>
        <select id="exp-fmt" class="anki-select" bind:value={format}>
          <option value="apkg">Deck Package (*.apkg)</option>
          <option value="colpkg">Collection Package (*.colpkg)</option>
          <option value="notes_csv">Notes in Plain Text (*.txt)</option>
          <option value="cards_csv">Cards in Plain Text (*.txt)</option>
        </select>
      </div>

      {#if format !== "colpkg"}
        <div class="form-row">
          <label for="exp-scope">Include:</label>
          <select id="exp-scope" class="anki-select" bind:value={selectedDeckId}>
            <option value="all">All Decks</option>
            {#if hasSelectedNotes}
              <option value="selected">Selected Notes ({store.exportModal.noteIds?.length})</option>
            {/if}
            {#each decks as d}
              <option value={String(d.id)}>{d.name}</option>
            {/each}
          </select>
        </div>
      {/if}

      <fieldset class="opt-group">
        <legend>Include</legend>
        {#if format === "apkg"}
          <label class="check-row">
            <input type="checkbox" bind:checked={includeScheduling} />
            Include scheduling information
          </label>
          <label class="check-row">
            <input type="checkbox" bind:checked={includeDeckConfigs} />
            Include deck presets
          </label>
          <label class="check-row">
            <input type="checkbox" bind:checked={includeMedia} />
            Include media
          </label>
          <label class="check-row">
            <input type="checkbox" bind:checked={legacy} />
            Support older versions (slower/larger files)
          </label>
        {:else if format === "colpkg"}
          <label class="check-row">
            <input type="checkbox" bind:checked={includeMedia} />
            Include media
          </label>
          <label class="check-row">
            <input type="checkbox" bind:checked={legacy} />
            Support older versions (slower/larger files)
          </label>
        {:else if format === "notes_csv"}
          <label class="check-row">
            <input type="checkbox" bind:checked={includeHtml} />
            Include HTML and media references
          </label>
          <label class="check-row">
            <input type="checkbox" bind:checked={includeTags} />
            Include tags
          </label>
          <label class="check-row">
            <input type="checkbox" bind:checked={includeDeck} />
            Include deck name
          </label>
          <label class="check-row">
            <input type="checkbox" bind:checked={includeNotetype} />
            Include note type name
          </label>
          <label class="check-row">
            <input type="checkbox" bind:checked={includeGuid} />
            Include unique identifier
          </label>
        {:else}
          <label class="check-row">
            <input type="checkbox" bind:checked={includeHtml} />
            Include HTML and media references
          </label>
        {/if}
      </fieldset>
    </div>
    <div class="modal-footer">
      <button class="anki-btn" onclick={close}>Cancel</button>
      <button class="anki-btn primary" disabled={exporting} onclick={doExport}>
        {exporting ? "Exporting..." : "Export..."}
      </button>
    </div>
  </div>
</div>

<style>
  .exp-window {
    width: 480px;
    max-width: 94vw;
  }
  .form-row {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 10px;
    font-size: 13px;
  }
  .form-row label {
    width: 110px;
    color: var(--fg-subtle);
  }
  .form-row select {
    flex: 1;
  }
  .opt-group {
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    padding: 10px 12px;
    margin-top: 8px;
  }
  .opt-group legend {
    font-size: 12px;
    font-weight: 600;
    color: var(--fg-subtle);
    padding: 0 4px;
  }
  .check-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    margin: 5px 0;
    cursor: pointer;
  }
</style>
