<script lang="ts">
  import { store } from "../lib/store.svelte";

  let promptValue = $state("");

  $effect(() => {
    if (store.dialog?.kind === "prompt") {
      promptValue = store.dialog.defaultValue ?? "";
    }
  });

  function submitDialog(e?: Event) {
    e?.preventDefault();
    const d = store.dialog;
    if (!d) return;
    if (d.kind === "prompt") d.resolve(promptValue);
    else if (d.kind === "confirm") d.resolve(true);
    else d.resolve(undefined);
  }

  function cancelDialog() {
    const d = store.dialog;
    if (!d) return;
    if (d.kind === "prompt") d.resolve(null);
    else if (d.kind === "confirm") d.resolve(false);
    else d.resolve(undefined);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!store.dialog) return;
    if (e.key === "Escape") {
      e.preventDefault();
      cancelDialog();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if store.dialog}
  {@const d = store.dialog}
  <div class="modal-backdrop" style="z-index: 3000" role="dialog" aria-modal="true">
    <form class="modal-window dialog-box" onsubmit={submitDialog}>
      <div class="modal-header">
        <h3 class="modal-title">{d.title}</h3>
        <button type="button" class="modal-close" onclick={cancelDialog}>×</button>
      </div>
      <div class="modal-body">
        <p class="dialog-text">{d.text}</p>
        {#if d.kind === "prompt"}
          <!-- svelte-ignore a11y_autofocus -->
          <input
            class="anki-input prompt-input"
            type="text"
            bind:value={promptValue}
            placeholder={d.placeholder ?? ""}
            autofocus
          />
        {/if}
      </div>
      <div class="modal-footer">
        {#if d.kind !== "alert"}
          <button type="button" class="anki-btn" onclick={cancelDialog}>
            {d.cancelLabel ?? "Cancel"}
          </button>
        {/if}
        <button type="submit" class="anki-btn {d.danger ? 'danger' : 'primary'}">
          {d.confirmLabel ?? "OK"}
        </button>
      </div>
    </form>
  </div>
{/if}

{#if store.toasts.length > 0}
  <div class="toast-stack" aria-live="polite">
    {#each store.toasts as t (t.id)}
      <div class="toast-pill {t.kind}">{t.text}</div>
    {/each}
  </div>
{/if}

<style>
  .dialog-box {
    width: 420px;
    max-width: 92vw;
  }
  .dialog-text {
    margin: 0 0 12px;
    font-size: 14px;
    line-height: 1.45;
    white-space: pre-wrap;
  }
  .prompt-input {
    width: 100%;
  }
  .toast-stack {
    position: fixed;
    bottom: 64px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    z-index: 4000;
    pointer-events: none;
  }
  .toast-pill {
    background: var(--canvas-overlay);
    color: var(--fg);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 7px 16px;
    font-size: 13px;
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.22);
  }
  .toast-pill.warning {
    border-color: var(--state-buried);
  }
  .toast-pill.error {
    border-color: var(--accent-danger);
    color: var(--accent-danger);
  }
</style>
