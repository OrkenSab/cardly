<script lang="ts">
  import {
    addNotetypeLegacy,
    getNotetype,
    getNotetypeLegacy,
    getNotetypeNamesAndCounts,
    getStockNotetypeLegacy,
    removeNotetype,
    updateNotetype,
  } from "@generated/backend";
  import {
    NotetypeNameIdUseCount,
    StockNotetype_Kind,
  } from "@generated/anki/notetypes_pb";
  import { store } from "../lib/store.svelte";

  let entries = $state<NotetypeNameIdUseCount[]>([]);
  let selectedId = $state<number>(0);
  let loading = $state(true);
  let addPickerOpen = $state(false);
  let addChoice = $state("stock:0");

  const stockOptions: Array<{ label: string; kind: StockNotetype_Kind }> = [
    { label: "Add: Basic", kind: StockNotetype_Kind.BASIC },
    { label: "Add: Basic (and reversed card)", kind: StockNotetype_Kind.BASIC_AND_REVERSED },
    { label: "Add: Basic (optional reversed card)", kind: StockNotetype_Kind.BASIC_OPTIONAL_REVERSED },
    { label: "Add: Basic (type in the answer)", kind: StockNotetype_Kind.BASIC_TYPING },
    { label: "Add: Cloze", kind: StockNotetype_Kind.CLOZE },
    { label: "Add: Image Occlusion", kind: StockNotetype_Kind.IMAGE_OCCLUSION },
  ];

  const selectedEntry = $derived(entries.find((e) => Number(e.id) === selectedId) ?? null);

  async function load() {
    loading = true;
    try {
      const res = await getNotetypeNamesAndCounts({});
      entries = res.entries;
      if (!selectedId || !entries.some((e) => Number(e.id) === selectedId)) {
        selectedId = entries[0] ? Number(entries[0].id) : 0;
      }
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to load note types", "error");
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void store.opRevision;
    if (store.manageNotetypesOpen) {
      void load();
    }
  });

  async function confirmAdd() {
    const newName = await store.prompt("Name for new note type:", "", "Add Note Type");
    if (!newName || !newName.trim()) return;
    try {
      let jsonBytes: Uint8Array;
      if (addChoice.startsWith("stock:")) {
        const kind = parseInt(addChoice.slice(6), 10) as StockNotetype_Kind;
        jsonBytes = (await getStockNotetypeLegacy({ kind })).json;
      } else {
        const cloneId = BigInt(addChoice.slice(6));
        jsonBytes = (await getNotetypeLegacy({ ntid: cloneId })).json;
      }
      const parsed = JSON.parse(new TextDecoder().decode(jsonBytes));
      parsed.id = 0;
      parsed.name = newName.trim();
      const encoded = new TextEncoder().encode(JSON.stringify(parsed));
      const res = await addNotetypeLegacy({ json: encoded });
      selectedId = Number(res.id);
      addPickerOpen = false;
      store.bumpOpRevision();
      store.toast(`Created note type "${newName.trim()}"`);
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to add note type", "error");
    }
  }

  async function renameSelected() {
    if (!selectedEntry) return;
    const newName = await store.prompt("New name:", selectedEntry.name, "Rename Note Type");
    if (!newName || !newName.trim() || newName.trim() === selectedEntry.name) return;
    try {
      const nt = await getNotetype({ ntid: selectedEntry.id });
      nt.name = newName.trim();
      await updateNotetype(nt);
      store.bumpOpRevision();
      store.toast("Note type renamed");
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to rename note type", "error");
    }
  }

  async function deleteSelected() {
    if (!selectedEntry) return;
    if (entries.length <= 1) {
      store.toast("You must keep at least one note type", "warning");
      return;
    }
    const ok = await store.confirm(
      `Delete note type "${selectedEntry.name}" and all ${selectedEntry.useCount} note(s) using it?`,
      "Delete Note Type",
      { danger: true, confirmLabel: "Delete" }
    );
    if (!ok) return;
    try {
      await removeNotetype({ ntid: selectedEntry.id });
      selectedId = 0;
      store.bumpOpRevision();
      store.toast(`Deleted "${selectedEntry.name}"`);
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to delete note type", "error");
    }
  }

  function close() {
    store.manageNotetypesOpen = false;
  }
</script>

<div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Manage Note Types">
  <div class="modal-window nt-window">
    <div class="modal-header">
      <h3 class="modal-title">Note Types</h3>
      <button class="modal-close" onclick={close}>×</button>
    </div>
    <div class="modal-body nt-body">
      <div class="nt-list" role="listbox">
        {#if loading}
          <div class="muted pad">Loading...</div>
        {:else}
          {#each entries as e (e.id)}
            <button
              type="button"
              class="nt-item"
              class:selected={Number(e.id) === selectedId}
              onclick={() => (selectedId = Number(e.id))}
              ondblclick={() => store.openCardTemplates(Number(e.id))}
            >
              <span class="nt-name">{e.name}</span>
              <span class="nt-count">{e.useCount} note{e.useCount === 1 ? "" : "s"}</span>
            </button>
          {/each}
        {/if}
      </div>

      <div class="nt-actions">
        <button class="anki-btn" onclick={() => (addPickerOpen = true)}>Add</button>
        <button class="anki-btn" disabled={!selectedEntry} onclick={renameSelected}>Rename</button>
        <button class="anki-btn danger" disabled={!selectedEntry} onclick={deleteSelected}>Delete</button>
        <div class="spacer"></div>
        <button
          class="anki-btn"
          disabled={!selectedEntry}
          onclick={() => selectedEntry && store.openFields(Number(selectedEntry.id))}
        >
          Fields...
        </button>
        <button
          class="anki-btn"
          disabled={!selectedEntry}
          onclick={() => selectedEntry && store.openCardTemplates(Number(selectedEntry.id))}
        >
          Cards...
        </button>
      </div>
    </div>

    {#if addPickerOpen}
      <div class="add-picker">
        <div class="picker-title">Choose a base note type to add or clone:</div>
        <select class="anki-select" bind:value={addChoice} size="8">
          {#each stockOptions as opt}
            <option value={`stock:${opt.kind}`}>{opt.label}</option>
          {/each}
          {#each entries as e}
            <option value={`clone:${e.id}`}>Clone: {e.name}</option>
          {/each}
        </select>
        <div class="picker-btns">
          <button class="anki-btn small" onclick={() => (addPickerOpen = false)}>Cancel</button>
          <button class="anki-btn small primary" onclick={confirmAdd}>OK</button>
        </div>
      </div>
    {/if}

    <div class="modal-footer">
      <button class="anki-btn" onclick={close}>Close</button>
    </div>
  </div>
</div>

<style>
  .nt-window {
    width: 520px;
    max-width: 94vw;
  }
  .nt-body {
    display: flex;
    gap: 12px;
    min-height: 280px;
  }
  .nt-list {
    flex: 1;
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    background: var(--canvas-inset);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    max-height: 320px;
  }
  .nt-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 7px 10px;
    border: none;
    background: transparent;
    color: var(--fg);
    font-size: 13px;
    cursor: pointer;
    text-align: left;
  }
  .nt-item:hover {
    background: var(--highlight-bg);
  }
  .nt-item.selected {
    background: var(--accent);
    color: #fff;
  }
  .nt-count {
    font-size: 12px;
    opacity: 0.78;
  }
  .nt-actions {
    width: 112px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .spacer {
    height: 10px;
  }
  .add-picker {
    border-top: 1px solid var(--border);
    padding: 12px 16px;
    background: var(--canvas-elevated);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .picker-title {
    font-size: 13px;
    font-weight: 500;
  }
  .picker-btns {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .muted {
    color: var(--fg-subtle);
  }
  .pad {
    padding: 16px;
  }
</style>
