<script lang="ts">
  import { getNote } from "@generated/backend";
  import { store } from "../lib/store.svelte";

  interface Props {
    mode: "add" | "current" | "browser";
    noteId?: number | null;
    notetypeId?: number | null;
    deckId?: number | null;
    reviewerCardId?: number | null;
    onSaved?: () => void;
  }

  let {
    mode,
    noteId = null,
    notetypeId = null,
    deckId = null,
    reviewerCardId = null,
    onSaved,
  }: Props = $props();

  let iframeEl = $state<HTMLIFrameElement | null>(null);
  let ready = $state(false);
  let lastLoadedKey = "";

  const src = $derived(`/editor/?mode=${mode}`);

  function syncTheme() {
    if (!iframeEl) return;
    try {
      const doc = iframeEl.contentDocument;
      if (!doc) return;
      const dark = store.isDark;
      doc.documentElement.classList.toggle("night-mode", dark);
      doc.documentElement.classList.toggle("nightMode", dark);
      if (doc.body) {
        doc.body.classList.toggle("night-mode", dark);
        doc.body.classList.toggle("nightMode", dark);
      }
    } catch {
      // ignore
    }
  }

  async function pushNoteToEditor(force = false) {
    if (!ready || !iframeEl?.contentWindow) return;
    const key = `${mode}:${noteId ?? ""}:${notetypeId ?? ""}:${deckId ?? ""}:${reviewerCardId ?? ""}`;
    if (!force && key === lastLoadedKey) return;
    lastLoadedKey = key;

    const win = iframeEl.contentWindow as any;
    if (typeof win.loadNote !== "function") return;

    if (mode !== "add" && !noteId) {
      return;
    }

    let effectiveNotetypeId = notetypeId;
    if (mode !== "add" && noteId && !effectiveNotetypeId) {
      try {
        const n = await getNote({ nid: BigInt(noteId) }, { alertOnError: false });
        effectiveNotetypeId = Number(n.notetypeId);
      } catch {
        // fallback
      }
    }

    store.activeEditorNoteId = noteId ?? 0;
    if (effectiveNotetypeId) store.activeEditorNotetypeId = effectiveNotetypeId;

    const loadArgs: Record<string, unknown> = {
      nid: noteId ?? null,
      notetypeId: effectiveNotetypeId ?? null,
      focusTo: mode === "add" ? 0 : null,
      originalNoteId: null,
      reviewerCardId: reviewerCardId ?? null,
      deckId: deckId ?? null,
      initial: true,
    };

    try {
      if (typeof win.require === "function") {
        win.require("anki/ui").loaded.then(() => {
          win.loadNote(loadArgs);
        });
      } else {
        win.loadNote(loadArgs);
      }
    } catch {
      win.loadNote(loadArgs);
    }
  }

  function handleWindowMessage(ev: MessageEvent) {
    if (ev.origin !== window.location.origin) return;
    if (!iframeEl || ev.source !== iframeEl.contentWindow) return;
    if (!ev.data || ev.data.type !== "cardly:bridge") return;

    const cmd = String(ev.data.cmd ?? "");
    if (cmd === "editorReady") {
      ready = true;
      syncTheme();
      pushNoteToEditor(true);
    } else if (cmd === "saved") {
      onSaved?.();
    }
  }

  $effect(() => {
    void store.isDark;
    syncTheme();
  });

  $effect(() => {
    void ready;
    void mode;
    void noteId;
    void notetypeId;
    void deckId;
    void reviewerCardId;
    pushNoteToEditor(false);
  });
</script>

<svelte:window onmessage={handleWindowMessage} />

<div class="editor-host">
  {#if mode === "browser" && !noteId}
    <div class="empty-editor">Select a card or note to edit</div>
  {/if}
  <iframe
    bind:this={iframeEl}
    {src}
    title="Note Editor"
    class="editor-iframe"
    class:hidden-frame={mode === "browser" && !noteId}
    onload={() => syncTheme()}
  ></iframe>
</div>

<style>
  .editor-host {
    position: relative;
    flex: 1;
    width: 100%;
    height: 100%;
    display: flex;
    min-height: 0;
  }
  .editor-iframe {
    width: 100%;
    height: 100%;
    border: none;
    display: block;
    background: var(--canvas);
  }
  .editor-iframe.hidden-frame {
    visibility: hidden;
    position: absolute;
    inset: 0;
  }
  .empty-editor {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--fg-subtle);
    font-size: 14px;
    background: var(--canvas);
  }
</style>
