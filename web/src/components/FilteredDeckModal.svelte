<script lang="ts">
  import {
    addOrUpdateFilteredDeck,
    filteredDeckOrderLabels,
    getOrCreateFilteredDeck,
  } from "@generated/backend";
  import { Deck_Filtered, Deck_Filtered_SearchTerm, FilteredDeckForUpdate } from "@generated/anki/decks_pb";
  import { store } from "../lib/store.svelte";

  let loading = $state(true);
  let saving = $state(false);
  let orderLabels = $state<string[]>([]);
  let deckId = $state(0n);
  let name = $state("Filtered Deck 1");
  let reschedule = $state(true);
  let customDelays = $state(false);
  let delaysText = $state("1 10");
  let previewAgain = $state(60);
  let previewHard = $state(600);
  let previewGood = $state(0);

  let search1 = $state("is:due");
  let limit1 = $state(100);
  let order1 = $state(0);

  let enableSecond = $state(false);
  let search2 = $state("");
  let limit2 = $state(100);
  let order2 = $state(0);

  async function load() {
    loading = true;
    try {
      const [labelsRes, fd] = await Promise.all([
        filteredDeckOrderLabels({}),
        getOrCreateFilteredDeck({ did: BigInt(store.filteredDeck.deckId ?? 0) }),
      ]);
      orderLabels = labelsRes.vals;
      deckId = fd.id;
      name = fd.name || "Filtered Deck 1";
      const cfg = fd.config;
      if (cfg) {
        reschedule = cfg.reschedule;
        customDelays = cfg.delays.length > 0;
        delaysText = cfg.delays.length > 0 ? cfg.delays.join(" ") : "1 10";
        previewAgain = cfg.previewAgainSecs || 60;
        previewHard = cfg.previewHardSecs || 600;
        previewGood = cfg.previewGoodSecs || 0;
        if (cfg.searchTerms[0]) {
          search1 = cfg.searchTerms[0].search;
          limit1 = cfg.searchTerms[0].limit;
          order1 = cfg.searchTerms[0].order;
        }
        if (cfg.searchTerms[1]) {
          enableSecond = true;
          search2 = cfg.searchTerms[1].search;
          limit2 = cfg.searchTerms[1].limit;
          order2 = cfg.searchTerms[1].order;
        } else {
          enableSecond = false;
          search2 = "";
          limit2 = 100;
          order2 = 0;
        }
      }
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to load filtered deck", "error");
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (store.filteredDeck.open) {
      void load();
    }
  });

  async function submit() {
    if (!name.trim()) {
      store.toast("Deck name cannot be empty", "warning");
      return;
    }
    saving = true;
    try {
      const terms: Deck_Filtered_SearchTerm[] = [
        new Deck_Filtered_SearchTerm({
          search: search1.trim(),
          limit: Math.max(1, Number(limit1) || 100),
          order: order1,
        }),
      ];
      if (enableSecond && search2.trim()) {
        terms.push(
          new Deck_Filtered_SearchTerm({
            search: search2.trim(),
            limit: Math.max(1, Number(limit2) || 100),
            order: order2,
          })
        );
      }
      const delays = customDelays
        ? delaysText
            .split(/\s+/)
            .map((s) => parseFloat(s))
            .filter((n) => Number.isFinite(n) && n > 0)
        : [];

      const payload = new FilteredDeckForUpdate({
        id: deckId,
        name: name.trim(),
        allowEmpty: false,
        config: new Deck_Filtered({
          reschedule,
          searchTerms: terms,
          delays,
          previewAgainSecs: Number(previewAgain) || 60,
          previewHardSecs: Number(previewHard) || 600,
          previewGoodSecs: Number(previewGood) || 0,
        }),
      });
      const res = await addOrUpdateFilteredDeck(payload);
      store.bumpOpRevision();
      store.filteredDeck = { open: false };
      if (res.id) {
        store.currentDeckId = Number(res.id);
        store.view = "overview";
      }
      store.toast("Filtered deck built");
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Could not build filtered deck", "error");
    } finally {
      saving = false;
    }
  }

  function close() {
    store.filteredDeck = { open: false };
  }
</script>

<div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Filtered Deck">
  <div class="modal-window fd-window">
    <div class="modal-header">
      <h3 class="modal-title">{deckId ? "Options for Filtered Deck" : "Create Filtered Deck"}</h3>
      <button class="modal-close" onclick={close}>×</button>
    </div>
    <div class="modal-body">
      {#if loading}
        <div class="muted">Loading...</div>
      {:else}
        <div class="form-row">
          <label for="fd-name">Name:</label>
          <input id="fd-name" type="text" class="anki-input" bind:value={name} />
        </div>

        <fieldset class="filter-group">
          <legend>Filter</legend>
          <div class="form-row">
            <label for="fd-search1">Search:</label>
            <input id="fd-search1" type="text" class="anki-input mono" bind:value={search1} />
          </div>
          <div class="split-row">
            <div class="form-row">
              <label for="fd-limit1">Limit to:</label>
              <input id="fd-limit1" type="number" class="anki-input num" min="1" max="99999" bind:value={limit1} />
            </div>
            <div class="form-row grow">
              <label for="fd-order1">cards selected by:</label>
              <select id="fd-order1" class="anki-select" bind:value={order1}>
                {#each orderLabels as lbl, idx}
                  <option value={idx}>{lbl}</option>
                {/each}
              </select>
            </div>
          </div>
        </fieldset>

        <label class="check-row">
          <input type="checkbox" bind:checked={enableSecond} />
          Enable second filter
        </label>

        {#if enableSecond}
          <fieldset class="filter-group">
            <legend>Filter 2</legend>
            <div class="form-row">
              <label for="fd-search2">Search:</label>
              <input id="fd-search2" type="text" class="anki-input mono" bind:value={search2} />
            </div>
            <div class="split-row">
              <div class="form-row">
                <label for="fd-limit2">Limit to:</label>
                <input id="fd-limit2" type="number" class="anki-input num" min="1" max="99999" bind:value={limit2} />
              </div>
              <div class="form-row grow">
                <label for="fd-order2">cards selected by:</label>
                <select id="fd-order2" class="anki-select" bind:value={order2}>
                  {#each orderLabels as lbl, idx}
                    <option value={idx}>{lbl}</option>
                  {/each}
                </select>
              </div>
            </div>
          </fieldset>
        {/if}

        <fieldset class="filter-group">
          <legend>Options</legend>
          <label class="check-row">
            <input type="checkbox" bind:checked={reschedule} />
            Reschedule cards based on my answers in this deck
          </label>
          {#if reschedule}
            <label class="check-row">
              <input type="checkbox" bind:checked={customDelays} />
              Custom steps (in minutes):
              <input
                type="text"
                class="anki-input steps-in"
                disabled={!customDelays}
                bind:value={delaysText}
                placeholder="1 10"
              />
            </label>
          {:else}
            <div class="preview-delays">
              <div class="form-row">
                <label for="fd-pa">Again delay (secs):</label>
                <input id="fd-pa" type="number" class="anki-input num" min="0" bind:value={previewAgain} />
              </div>
              <div class="form-row">
                <label for="fd-ph">Hard delay (secs):</label>
                <input id="fd-ph" type="number" class="anki-input num" min="0" bind:value={previewHard} />
              </div>
              <div class="form-row">
                <label for="fd-pg">Good delay (secs):</label>
                <input id="fd-pg" type="number" class="anki-input num" min="0" bind:value={previewGood} />
              </div>
            </div>
          {/if}
        </fieldset>
      {/if}
    </div>
    <div class="modal-footer">
      <button class="anki-btn" onclick={close}>Cancel</button>
      <button class="anki-btn primary" disabled={loading || saving} onclick={submit}>
        {saving ? "Building..." : deckId ? "Rebuild" : "Build"}
      </button>
    </div>
  </div>
</div>

<style>
  .fd-window {
    width: 560px;
    max-width: 95vw;
  }
  .form-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
    font-size: 13px;
  }
  .form-row label {
    white-space: nowrap;
    color: var(--fg-subtle);
  }
  .form-row input[type="text"],
  .form-row select {
    flex: 1;
  }
  .split-row {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
  }
  .grow {
    flex: 1;
  }
  .num {
    width: 82px;
  }
  .mono {
    font-family: var(--font-mono);
  }
  .filter-group {
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    padding: 10px 12px 6px;
    margin: 10px 0;
  }
  .filter-group legend {
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
    margin: 6px 0;
    cursor: pointer;
  }
  .steps-in {
    width: 120px;
  }
  .preview-delays {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
    margin-top: 6px;
  }
  .muted {
    color: var(--fg-subtle);
  }
</style>
