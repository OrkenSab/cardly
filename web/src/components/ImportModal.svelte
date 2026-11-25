<script lang="ts">
  import { importColpkgUpload, uploadFile } from "../lib/api";
  import { store } from "../lib/store.svelte";
  import Icon from "./Icon.svelte";
  import UpstreamFrame from "./UpstreamFrame.svelte";

  let dragging = $state(false);
  let uploading = $state(false);
  let fileInput = $state<HTMLInputElement | null>(null);

  async function handleFile(file: File) {
    uploading = true;
    try {
      const uploaded = await uploadFile(file);
      if (uploaded.kind === "colpkg") {
        const ok = await store.confirm(
          `Importing "${uploaded.filename}" will replace your entire collection (a backup will be created automatically first). Continue?`,
          "Import Collection Package",
          { danger: true, confirmLabel: "Replace Collection" }
        );
        if (!ok) {
          uploading = false;
          return;
        }
        await importColpkgUpload(uploaded.upload_id);
        store.toast("Collection imported successfully");
        store.importModal = { open: false, mode: "pick" };
        store.view = "decks";
      } else if (uploaded.kind === "apkg") {
        store.importModal = {
          open: true,
          mode: "apkg",
          uploadId: uploaded.upload_id,
          filename: uploaded.filename,
        };
      } else {
        // csv / tsv / txt -> upstream CSV importer
        store.importModal = {
          open: true,
          mode: "csv",
          uploadId: uploaded.upload_id,
          filename: uploaded.filename,
        };
      }
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Import upload failed", "error");
    } finally {
      uploading = false;
    }
  }

  function onFileChange(ev: Event) {
    const input = ev.currentTarget as HTMLInputElement;
    const f = input.files?.[0];
    if (f) void handleFile(f);
    input.value = "";
  }

  function onDrop(ev: DragEvent) {
    ev.preventDefault();
    dragging = false;
    const f = ev.dataTransfer?.files?.[0];
    if (f) void handleFile(f);
  }

  function close() {
    store.importModal = { open: false, mode: "pick" };
    store.bumpOpRevision();
  }
</script>

<div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Import">
  <div
    class="modal-window"
    class:pick-window={store.importModal.mode === "pick"}
    class:frame-window={store.importModal.mode !== "pick"}
  >
    <div class="modal-header">
      <h3 class="modal-title">
        {#if store.importModal.mode === "pick"}
          Import File
        {:else if store.importModal.mode === "apkg"}
          Import Package — {store.importModal.filename ?? ""}
        {:else}
          Import Text / CSV — {store.importModal.filename ?? ""}
        {/if}
      </h3>
      <button class="modal-close" onclick={close}>×</button>
    </div>

    {#if store.importModal.mode === "pick"}
      <div class="modal-body">
        <div
          class="dropzone"
          class:dragging
          role="button"
          tabindex="0"
          onclick={() => fileInput?.click()}
          onkeydown={(e) => (e.key === "Enter" || e.key === " ") && fileInput?.click()}
          ondragover={(e) => {
            e.preventDefault();
            dragging = true;
          }}
          ondragleave={() => (dragging = false)}
          ondrop={onDrop}
        >
          {#if uploading}
            <div class="dz-title">Uploading file...</div>
          {:else}
            <div class="dz-icon"><Icon name="inbox" size={32} /></div>
            <div class="dz-title">Drop a file here, or click to browse</div>
            <div class="dz-sub">
              Supported formats: <code>.apkg</code> (Deck Package), <code>.colpkg</code> (Collection Package),
              <code>.csv</code>, <code>.tsv</code>, <code>.txt</code>
            </div>
          {/if}
        </div>
        <input
          bind:this={fileInput}
          type="file"
          accept=".apkg,.colpkg,.csv,.tsv,.txt"
          style="display:none"
          onchange={onFileChange}
        />
      </div>
      <div class="modal-footer">
        <button class="anki-btn" onclick={close}>Cancel</button>
      </div>
    {:else if store.importModal.mode === "apkg" && store.importModal.uploadId}
      <div class="modal-body flush">
        <UpstreamFrame
          src={`/import-anki-package/${encodeURIComponent(store.importModal.uploadId)}`}
          title="Import Deck Package"
        />
      </div>
    {:else if store.importModal.mode === "csv" && store.importModal.uploadId}
      <div class="modal-body flush">
        <UpstreamFrame
          src={`/import-csv/${encodeURIComponent(store.importModal.uploadId)}`}
          title="Import CSV"
        />
      </div>
    {/if}
  </div>
</div>

<style>
  .pick-window {
    width: 500px;
    max-width: 94vw;
  }
  .frame-window {
    width: min(920px, 94vw);
    height: min(720px, 88vh);
  }
  .dropzone {
    border: 2px dashed var(--border-strong);
    border-radius: var(--border-radius-lg);
    padding: 40px 24px;
    text-align: center;
    cursor: pointer;
    background: var(--canvas-inset);
    transition: border-color 0.15s, background 0.15s;
  }
  .dropzone:hover,
  .dropzone.dragging {
    border-color: var(--accent);
    background: var(--highlight-bg);
  }
  .dz-icon {
    font-size: 32px;
    margin-bottom: 10px;
  }
  .dz-title {
    font-size: 15px;
    font-weight: 500;
    margin-bottom: 6px;
  }
  .dz-sub {
    font-size: 12px;
    color: var(--fg-subtle);
    line-height: 1.5;
  }
  .dz-sub code {
    font-family: var(--font-mono);
    background: var(--canvas-code);
    padding: 1px 4px;
    border-radius: 3px;
  }
</style>
