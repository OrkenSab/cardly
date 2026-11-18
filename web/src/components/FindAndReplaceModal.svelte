<script lang="ts">
  import { fieldNamesForNotes, findAndReplace } from "@generated/backend";
  import { store } from "../lib/store.svelte";

  interface Props {
    noteIds: number[];
    onClose: () => void;
    onDone: () => void;
  }

  let { noteIds, onClose, onDone }: Props = $props();

  let fieldNames = $state<string[]>([]);
  let selectedField = $state<string>(""); // "" = All fields
  let search = $state("");
  let replacement = $state("");
  let regex = $state(false);
  let matchCase = $state(false);
  let running = $state(false);

  $effect(() => {
    if (noteIds.length > 0) {
      void fieldNamesForNotes({ nids: noteIds.map((id) => BigInt(id)) })
        .then((res) => {
          fieldNames = res.fields;
        })
        .catch(() => {});
    }
  });

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!search) return;
    running = true;
    try {
      const res = await findAndReplace({
        nids: noteIds.map((id) => BigInt(id)),
        search,
        replacement,
        regex,
        matchCase,
        fieldName: selectedField,
      });
      store.toast(`Updated ${res.count} note(s)`);
      onDone();
      onClose();
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Find and replace failed", "error");
    } finally {
      running = false;
    }
  }
</script>

<div class="modal-backdrop" style="z-index: 1500" role="dialog" aria-modal="true">
  <form class="modal-window fr-win" onsubmit={submit}>
    <div class="modal-header">
      <h3 class="modal-title">Find and Replace ({noteIds.length} selected note{noteIds.length === 1 ? "" : "s"})</h3>
      <button type="button" class="modal-close" onclick={onClose}>×</button>
    </div>
    <div class="modal-body fr-body">
      <label class="row">
        <span>Find:</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="anki-input" type="text" bind:value={search} required autofocus />
      </label>
      <label class="row">
        <span>Replace With:</span>
        <input class="anki-input" type="text" bind:value={replacement} />
      </label>
      <label class="row">
        <span>In:</span>
        <select class="anki-select" bind:value={selectedField}>
          <option value="">All Fields</option>
          {#each fieldNames as f}
            <option value={f}>{f}</option>
          {/each}
        </select>
      </label>
      <div class="checks">
        <label><input type="checkbox" bind:checked={matchCase} /> Ignore case: off (Match case)</label>
        <label><input type="checkbox" bind:checked={regex} /> Treat input as regular expression</label>
      </div>
    </div>
    <div class="modal-footer">
      <button type="button" class="anki-btn" onclick={onClose}>Cancel</button>
      <button type="submit" class="anki-btn primary" disabled={running || !search}>
        {running ? "Replacing..." : "OK"}
      </button>
    </div>
  </form>
</div>

<style>
  .fr-win {
    width: 460px;
  }
  .fr-body {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .row {
    display: grid;
    grid-template-columns: 110px 1fr;
    align-items: center;
    gap: 10px;
    font-size: 13.5px;
  }
  .checks {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
    padding-left: 120px;
  }
  .checks label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
</style>
