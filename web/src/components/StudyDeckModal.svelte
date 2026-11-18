<script lang="ts">
  import { getDeckNames, setCurrentDeck } from "@generated/backend";
  import type { DeckNameId } from "@generated/anki/decks_pb";
  import { store } from "../lib/store.svelte";

  let decks = $state<DeckNameId[]>([]);
  let filter = $state("");
  let selectedIndex = $state(0);
  let inputEl = $state<HTMLInputElement | null>(null);

  const filtered = $derived.by(() => {
    const q = filter.trim().toLowerCase();
    if (!q) return decks;
    return decks.filter((d) => d.name.toLowerCase().includes(q));
  });

  async function load() {
    try {
      const res = await getDeckNames({ skipEmptyDefault: false, includeFiltered: true });
      decks = res.entries;
      selectedIndex = 0;
      setTimeout(() => inputEl?.focus(), 0);
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to load decks", "error");
    }
  }

  $effect(() => {
    if (store.studyDeckOpen) {
      filter = "";
      void load();
    }
  });

  $effect(() => {
    if (selectedIndex >= filtered.length) {
      selectedIndex = Math.max(0, filtered.length - 1);
    }
  });

  async function choose(d: DeckNameId) {
    await setCurrentDeck({ did: d.id });
    store.currentDeckId = Number(d.id);
    store.studyDeckOpen = false;
    store.view = "overview";
    store.bumpOpRevision();
  }

  function onKeydown(ev: KeyboardEvent) {
    if (ev.key === "ArrowDown") {
      ev.preventDefault();
      selectedIndex = Math.min(filtered.length - 1, selectedIndex + 1);
    } else if (ev.key === "ArrowUp") {
      ev.preventDefault();
      selectedIndex = Math.max(0, selectedIndex - 1);
    } else if (ev.key === "Enter" && filtered[selectedIndex]) {
      ev.preventDefault();
      void choose(filtered[selectedIndex]);
    } else if (ev.key === "Escape") {
      ev.preventDefault();
      store.studyDeckOpen = false;
    }
  }

  function close() {
    store.studyDeckOpen = false;
  }
</script>

<div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Study Deck">
  <div class="modal-window sd-window">
    <div class="modal-header">
      <h3 class="modal-title">Study Deck</h3>
      <button class="modal-close" onclick={close}>×</button>
    </div>
    <div class="modal-body">
      <input
        bind:this={inputEl}
        type="text"
        class="anki-input filter-input"
        placeholder="Filter decks..."
        bind:value={filter}
        onkeydown={onKeydown}
      />
      <div class="deck-list" role="listbox">
        {#each filtered as d, idx (d.id)}
          <button
            type="button"
            class="deck-opt"
            class:selected={idx === selectedIndex}
            onclick={() => (selectedIndex = idx)}
            ondblclick={() => void choose(d)}
          >
            {d.name}
          </button>
        {:else}
          <div class="empty">No matching decks.</div>
        {/each}
      </div>
    </div>
    <div class="modal-footer">
      <button class="anki-btn" onclick={close}>Cancel</button>
      <button
        class="anki-btn primary"
        disabled={!filtered[selectedIndex]}
        onclick={() => filtered[selectedIndex] && void choose(filtered[selectedIndex])}
      >
        Choose
      </button>
    </div>
  </div>
</div>

<style>
  .sd-window {
    width: 440px;
    max-width: 92vw;
  }
  .filter-input {
    width: 100%;
    margin-bottom: 10px;
  }
  .deck-list {
    max-height: 300px;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    background: var(--canvas-inset);
    display: flex;
    flex-direction: column;
  }
  .deck-opt {
    text-align: left;
    padding: 6px 10px;
    border: none;
    background: transparent;
    color: var(--fg);
    font-size: 13px;
    cursor: pointer;
  }
  .deck-opt:hover {
    background: var(--highlight-bg);
  }
  .deck-opt.selected {
    background: var(--accent);
    color: #fff;
  }
  .empty {
    padding: 16px;
    text-align: center;
    color: var(--fg-subtle);
    font-size: 13px;
  }
</style>
