<script lang="ts">
  import { renderExistingCard } from "@generated/backend";
  import { buildCardPreviewDoc, nodesToHtml } from "../lib/renderCard";
  import { store } from "../lib/store.svelte";

  let showBack = $state(false);
  let showBoth = $state(false);
  let html = $state("");
  let loading = $state(false);

  const currentCardId = $derived(store.previewer.cardIds[store.previewer.index] ?? 0);

  async function renderCurrent() {
    if (!currentCardId) return;
    loading = true;
    try {
      const r = await renderExistingCard({
        cardId: BigInt(currentCardId),
        browser: false,
        partialRender: false,
      });
      const qHtml = nodesToHtml(r.questionNodes);
      const aHtml = nodesToHtml(r.answerNodes);
      const content = showBoth ? aHtml : showBack ? aHtml : qHtml;
      html = buildCardPreviewDoc({
        bodyHtml: content,
        css: r.css,
        isDark: store.isDark,
        includeMathJax: true,
      });
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Preview render failed", "error");
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void currentCardId;
    void showBack;
    void showBoth;
    void store.isDark;
    void renderCurrent();
  });

  function prev() {
    if (store.previewer.index > 0) {
      store.previewer = { ...store.previewer, index: store.previewer.index - 1 };
      showBack = showBoth;
    }
  }

  function next() {
    if (store.previewer.index < store.previewer.cardIds.length - 1) {
      store.previewer = { ...store.previewer, index: store.previewer.index + 1 };
      showBack = showBoth;
    }
  }

  function close() {
    store.previewer = { open: false, cardIds: [], index: 0 };
  }
</script>

<div class="modal-backdrop" role="dialog" aria-modal="true">
  <div class="modal-window preview-window">
    <div class="modal-header">
      <h3 class="modal-title">
        Preview ({store.previewer.index + 1} / {store.previewer.cardIds.length})
      </h3>
      <button class="modal-close" onclick={close}>×</button>
    </div>
    <div class="modal-body flush">
      {#if loading && !html}
        <div class="center-msg">Rendering...</div>
      {:else}
        <iframe srcdoc={html} sandbox="allow-scripts allow-same-origin" title="Card Preview" class="preview-frame"></iframe>
      {/if}
    </div>
    <div class="modal-footer preview-footer">
      <label class="both-check">
        <input type="checkbox" bind:checked={showBoth} />
        Show both sides
      </label>
      <div class="nav-btns">
        <button class="anki-btn small" disabled={store.previewer.index <= 0} onclick={prev}>◀</button>
        {#if !showBoth}
          <button class="anki-btn small primary" onclick={() => (showBack = !showBack)}>
            {showBack ? "Show Front" : "Show Back"}
          </button>
        {/if}
        <button
          class="anki-btn small"
          disabled={store.previewer.index >= store.previewer.cardIds.length - 1}
          onclick={next}
        >
          ▶
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .preview-window {
    width: 680px;
    height: 540px;
  }
  .preview-frame {
    width: 100%;
    height: 100%;
    border: none;
    background: var(--canvas);
  }
  .preview-footer {
    justify-content: space-between;
  }
  .both-check {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
  }
  .nav-btns {
    display: flex;
    gap: 6px;
  }
  .center-msg {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--fg-subtle);
  }
</style>
