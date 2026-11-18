<script lang="ts">
  import { getDeck, updateDeck } from "@generated/backend";
  import { Deck } from "@generated/anki/decks_pb";
  import { store } from "../lib/store.svelte";

  let loading = $state(true);
  let saving = $state(false);
  let deckObj = $state<Deck | null>(null);
  let deckName = $state("");
  let description = $state("");
  let markdown = $state(false);

  async function load() {
    const did = store.deckDescription.deckId;
    if (!did) return;
    loading = true;
    try {
      const deck = await getDeck({ did: BigInt(did) });
      deckObj = deck;
      deckName = deck.name;
      if (deck.kind.case === "normal") {
        description = deck.kind.value.description;
        markdown = deck.kind.value.markdownDescription;
      } else {
        description = "";
        markdown = false;
      }
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to load deck", "error");
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (store.deckDescription.open && store.deckDescription.deckId) {
      void load();
    }
  });

  async function save() {
    if (!deckObj || deckObj.kind.case !== "normal") return;
    saving = true;
    try {
      deckObj.kind.value.description = description;
      deckObj.kind.value.markdownDescription = markdown;
      await updateDeck(deckObj);
      store.bumpOpRevision();
      store.toast("Deck description saved");
      close();
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to save description", "error");
    } finally {
      saving = false;
    }
  }

  function close() {
    store.deckDescription = { open: false, deckId: 0 };
  }
</script>

<div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Deck Description">
  <div class="modal-window desc-window">
    <div class="modal-header">
      <h3 class="modal-title">Description — {deckName}</h3>
      <button class="modal-close" onclick={close}>×</button>
    </div>
    <div class="modal-body">
      {#if loading}
        <div class="loading-msg">Loading...</div>
      {:else if deckObj?.kind.case !== "normal"}
        <p class="hint">Filtered decks do not support descriptions.</p>
      {:else}
        <textarea
          class="anki-input desc-area"
          bind:value={description}
          rows="10"
          placeholder="Enter a description for this deck (shown on the study overview screen)..."
        ></textarea>
        <label class="md-check">
          <input type="checkbox" bind:checked={markdown} />
          Format as Markdown
        </label>
      {/if}
    </div>
    <div class="modal-footer">
      <button class="anki-btn" onclick={close}>Cancel</button>
      <button
        class="anki-btn primary"
        disabled={loading || saving || deckObj?.kind.case !== "normal"}
        onclick={save}
      >
        {saving ? "Saving..." : "Save"}
      </button>
    </div>
  </div>
</div>

<style>
  .desc-window {
    width: 520px;
    max-width: 94vw;
  }
  .desc-area {
    width: 100%;
    min-height: 200px;
    resize: vertical;
    font-family: var(--font-mono);
    font-size: 13px;
    line-height: 1.45;
  }
  .md-check {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    margin-top: 10px;
    font-size: 13px;
    cursor: pointer;
  }
  .loading-msg,
  .hint {
    color: var(--fg-subtle);
    font-size: 13px;
  }
</style>
