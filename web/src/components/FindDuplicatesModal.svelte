<script lang="ts">
  import {
    addNoteTags,
    getNote,
    getNotetypeLegacy,
    getNotetypeNames,
    searchNotes,
  } from "@generated/backend";
  import { store } from "../lib/store.svelte";

  interface Props {
    onClose: () => void;
    onOpenSearch: (query: string) => void;
  }

  let { onClose, onOpenSearch }: Props = $props();

  interface DupGroup {
    value: string;
    noteIds: number[];
  }

  let fieldNames = $state<string[]>(["Front"]);
  let selectedField = $state("Front");
  let optionalFilter = $state("");
  let searching = $state(false);
  let groups = $state<DupGroup[]>([]);
  let searchedOnce = $state(false);

  $effect(() => {
    void (async () => {
      try {
        const names = await getNotetypeNames({});
        const allFields = new Set<string>();
        for (const nt of names.entries) {
          const raw = (await getNotetypeLegacy({ ntid: nt.id })).json;
          const obj = JSON.parse(new TextDecoder().decode(raw));
          for (const f of obj.flds ?? []) {
            allFields.add(f.name);
          }
        }
        if (allFields.size > 0) {
          fieldNames = Array.from(allFields);
          if (!allFields.has(selectedField)) selectedField = fieldNames[0];
        }
      } catch {
        // ignore
      }
    })();
  });

  function stripHtml(s: string): string {
    return s.replace(/<[^>]*>/g, " ").replace(/\s+/g, " ").trim().toLowerCase();
  }

  async function runSearch(e?: Event) {
    e?.preventDefault();
    searching = true;
    searchedOnce = true;
    groups = [];
    try {
      const q = optionalFilter.trim();
      const res = await searchNotes({ search: q });
      const ntCache = new Map<number, string[]>();
      const buckets = new Map<string, { display: string; ids: number[] }>();

      for (const nid of res.ids) {
        const note = await getNote({ nid }, { alertOnError: false });
        const ntid = Number(note.notetypeId);
        let flds = ntCache.get(ntid);
        if (!flds) {
          const raw = (await getNotetypeLegacy({ ntid: note.notetypeId }, { alertOnError: false })).json;
          const obj = JSON.parse(new TextDecoder().decode(raw));
          flds = (obj.flds ?? []).map((f: any) => f.name as string);
          ntCache.set(ntid, flds!);
        }
        const idx = flds!.indexOf(selectedField);
        if (idx === -1) continue;
        const val = note.fields[idx] ?? "";
        const norm = stripHtml(val);
        if (!norm) continue;
        const b = buckets.get(norm);
        if (b) {
          b.ids.push(Number(nid));
        } else {
          buckets.set(norm, { display: val.replace(/<[^>]*>/g, "").slice(0, 80), ids: [Number(nid)] });
        }
      }

      groups = Array.from(buckets.values())
        .filter((b) => b.ids.length > 1)
        .map((b) => ({ value: b.display, noteIds: b.ids }));
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Duplicate search failed", "error");
    } finally {
      searching = false;
    }
  }

  async function tagAllDuplicates() {
    const allIds = groups.flatMap((g) => g.noteIds);
    if (allIds.length === 0) return;
    await addNoteTags({ noteIds: allIds.map((id) => BigInt(id)), tags: "duplicate" });
    store.toast(`Tagged ${allIds.length} note(s) with "duplicate"`);
  }
</script>

<div class="modal-backdrop" style="z-index: 1500" role="dialog" aria-modal="true">
  <div class="modal-window dup-win">
    <div class="modal-header">
      <h3 class="modal-title">Find Duplicates</h3>
      <button class="modal-close" onclick={onClose}>×</button>
    </div>
    <div class="modal-body dup-body">
      <form class="dup-form" onsubmit={runSearch}>
        <label class="row">
          <span>Search in:</span>
          <select class="anki-select" bind:value={selectedField}>
            {#each fieldNames as f}
              <option value={f}>{f}</option>
            {/each}
          </select>
        </label>
        <label class="row">
          <span>Optional filter:</span>
          <input
            class="anki-input"
            type="text"
            bind:value={optionalFilter}
            placeholder='e.g. deck:"Default"'
          />
        </label>
        <div class="dup-actions">
          <button type="submit" class="anki-btn primary small" disabled={searching}>
            {searching ? "Searching..." : "Search"}
          </button>
          {#if groups.length > 0}
            <button type="button" class="anki-btn small" onclick={tagAllDuplicates}>
              Tag Duplicates
            </button>
          {/if}
        </div>
      </form>

      <div class="dup-results">
        {#if searching}
          <div class="muted">Scanning notes...</div>
        {:else if searchedOnce && groups.length === 0}
          <div class="muted">No duplicates found.</div>
        {:else if groups.length > 0}
          <div class="summary">Found {groups.length} duplicate group(s):</div>
          {#each groups as g}
            <button
              class="dup-item"
              onclick={() => {
                onOpenSearch(`nid:${g.noteIds.join(",")}`);
                onClose();
              }}
            >
              <span class="dup-text">{g.value}</span>
              <span class="dup-count">{g.noteIds.length} notes</span>
            </button>
          {/each}
        {/if}
      </div>
    </div>
    <div class="modal-footer">
      <button class="anki-btn" onclick={onClose}>Close</button>
    </div>
  </div>
</div>

<style>
  .dup-win {
    width: 520px;
    height: 480px;
  }
  .dup-body {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .dup-form {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .row {
    display: grid;
    grid-template-columns: 110px 1fr;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }
  .dup-actions {
    display: flex;
    gap: 8px;
    padding-left: 118px;
  }
  .dup-results {
    flex: 1;
    overflow: auto;
    border: 1px solid var(--border-subtle);
    border-radius: var(--border-radius);
    background: var(--canvas-inset);
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .summary,
  .muted {
    font-size: 13px;
    color: var(--fg-subtle);
    padding: 4px;
  }
  .dup-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 6px 8px;
    border: none;
    background: transparent;
    color: var(--fg);
    border-radius: 5px;
    cursor: pointer;
    text-align: left;
    font-family: inherit;
    font-size: 13px;
  }
  .dup-item:hover {
    background: var(--border-subtle);
  }
  .dup-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dup-count {
    color: var(--fg-subtle);
    font-size: 12px;
    flex-shrink: 0;
  }
</style>
