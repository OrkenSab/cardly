<script lang="ts">
  import {
    getDeckNames,
    getNote,
    getNotetype,
    newNote,
    renderUncommittedCard,
    updateNotetype,
  } from "@generated/backend";
  import type { DeckNameId } from "@generated/anki/decks_pb";
  import type { Note } from "@generated/anki/notes_pb";
  import {
    Notetype,
    Notetype_Config_Kind,
    Notetype_Template,
    Notetype_Template_Config,
  } from "@generated/anki/notetypes_pb";
  import { UInt32 } from "@generated/anki/generic_pb";
  import { buildCardPreviewDoc, nodesToHtml } from "../lib/renderCard";
  import { store } from "../lib/store.svelte";

  type EditorTab = "front" | "back" | "styling";

  let loading = $state(true);
  let saving = $state(false);
  let nt = $state<Notetype | null>(null);
  let sampleNote = $state<Note | null>(null);
  let decks = $state<DeckNameId[]>([]);

  let selectedOrd = $state(0);
  let activeTab = $state<EditorTab>("front");
  let previewSide = $state<"front" | "back">("front");
  let previewNight = $state(false);
  let fillEmpty = $state(true);

  let templates = $state<
    Array<{
      name: string;
      qFormat: string;
      aFormat: string;
      targetDeckId: bigint;
      orig: Notetype_Template;
    }>
  >([]);
  let css = $state("");

  let frontHtml = $state("");
  let backHtml = $state("");
  let renderError = $state("");
  let optionsMenuOpen = $state(false);
  let fieldMenuOpen = $state(false);
  let textareaEl = $state<HTMLTextAreaElement | null>(null);

  const isCloze = $derived(nt?.config?.kind === Notetype_Config_Kind.CLOZE);
  const currentTmpl = $derived(templates[selectedOrd] ?? null);

  let renderTimer: ReturnType<typeof setTimeout> | null = null;

  async function load() {
    const id = store.cardTemplates.notetypeId;
    if (!id) return;
    loading = true;
    previewNight = store.isDark;
    try {
      const [loadedNt, deckList] = await Promise.all([
        getNotetype({ ntid: BigInt(id) }),
        getDeckNames({ skipEmptyDefault: false, includeFiltered: false }),
      ]);
      nt = loadedNt;
      decks = deckList.entries;
      css = loadedNt.config?.css ?? "";
      templates = loadedNt.templates.map((t) => ({
        name: t.name,
        qFormat: t.config?.qFormat ?? "",
        aFormat: t.config?.aFormat ?? "",
        targetDeckId: t.config?.targetDeckId ?? 0n,
        orig: t,
      }));
      selectedOrd = Math.min(store.cardTemplates.ord ?? 0, Math.max(0, templates.length - 1));

      if (store.cardTemplates.noteId) {
        try {
          sampleNote = await getNote({ nid: BigInt(store.cardTemplates.noteId) });
        } catch {
          sampleNote = await newNote({ ntid: BigInt(id) });
        }
      } else {
        sampleNote = await newNote({ ntid: BigInt(id) });
      }
      scheduleRender();
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to load card templates", "error");
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (store.cardTemplates.open && store.cardTemplates.notetypeId) {
      void load();
    }
  });

  function scheduleRender() {
    if (renderTimer) clearTimeout(renderTimer);
    renderTimer = setTimeout(() => void renderPreview(), 120);
  }

  async function renderPreview() {
    if (!nt || !sampleNote || !currentTmpl) return;
    try {
      const tmplProto = currentTmpl.orig.clone();
      tmplProto.name = currentTmpl.name;
      tmplProto.ord = new UInt32({ val: selectedOrd });
      const cfg = tmplProto.config ? tmplProto.config.clone() : new Notetype_Template_Config();
      cfg.qFormat = currentTmpl.qFormat;
      cfg.aFormat = currentTmpl.aFormat;
      cfg.targetDeckId = currentTmpl.targetDeckId;
      tmplProto.config = cfg;

      const resp = await renderUncommittedCard(
        {
          note: sampleNote,
          cardOrd: selectedOrd,
          template: tmplProto,
          fillEmpty,
          partialRender: false,
        },
        { alertOnError: false }
      );
      renderError = "";
      frontHtml = buildCardPreviewDoc({
        bodyHtml: nodesToHtml(resp.questionNodes),
        css,
        isDark: previewNight,
        includeMathJax: true,
      });
      backHtml = buildCardPreviewDoc({
        bodyHtml: nodesToHtml(resp.answerNodes),
        css,
        isDark: previewNight,
        includeMathJax: true,
      });
    } catch (err) {
      renderError = err instanceof Error ? err.message : "Template render error";
    }
  }

  $effect(() => {
    void selectedOrd;
    void currentTmpl?.qFormat;
    void currentTmpl?.aFormat;
    void css;
    void previewNight;
    void fillEmpty;
    if (nt && sampleNote && currentTmpl) {
      scheduleRender();
    }
  });

  function insertField(fieldName: string) {
    fieldMenuOpen = false;
    if (!currentTmpl || activeTab === "styling") return;
    const token = `{{${fieldName}}}`;
    const el = textareaEl;
    if (el) {
      const start = el.selectionStart;
      const end = el.selectionEnd;
      const cur = activeTab === "front" ? currentTmpl.qFormat : currentTmpl.aFormat;
      const next = cur.slice(0, start) + token + cur.slice(end);
      if (activeTab === "front") currentTmpl.qFormat = next;
      else currentTmpl.aFormat = next;
      setTimeout(() => {
        el.focus();
        el.setSelectionRange(start + token.length, start + token.length);
      }, 0);
    } else {
      if (activeTab === "front") currentTmpl.qFormat += token;
      else currentTmpl.aFormat += token;
    }
  }

  function flipFrontBack() {
    if (!currentTmpl) return;
    const tmp = currentTmpl.qFormat;
    currentTmpl.qFormat = currentTmpl.aFormat;
    currentTmpl.aFormat = tmp;
    store.toast("Swapped Front and Back templates");
  }

  function addCardType() {
    optionsMenuOpen = false;
    if (isCloze) {
      store.toast("Cloze note types only use one template", "warning");
      return;
    }
    const idx = templates.length + 1;
    const firstField = nt?.fields[0]?.name ?? "Front";
    const secondField = nt?.fields[1]?.name ?? firstField;
    const tmpl = new Notetype_Template({
      name: `Card ${idx}`,
      config: new Notetype_Template_Config({
        qFormat: `{{${firstField}}}`,
        aFormat: `{{FrontSide}}\n\n<hr id=answer>\n\n{{${secondField}}}`,
        qFormatBrowser: "",
        aFormatBrowser: "",
        targetDeckId: 0n,
        browserFontName: "",
        browserFontSize: 0,
        other: new Uint8Array(),
      }),
    });
    templates = [
      ...templates,
      {
        name: tmpl.name,
        qFormat: tmpl.config!.qFormat,
        aFormat: tmpl.config!.aFormat,
        targetDeckId: 0n,
        orig: tmpl,
      },
    ];
    selectedOrd = templates.length - 1;
  }

  async function renameCardType() {
    optionsMenuOpen = false;
    if (!currentTmpl) return;
    const name = await store.prompt("Card type name:", currentTmpl.name, "Rename Card Type");
    if (!name || !name.trim()) return;
    currentTmpl.name = name.trim();
  }

  async function removeCardType() {
    optionsMenuOpen = false;
    if (!currentTmpl) return;
    if (templates.length <= 1) {
      store.toast("A note type must have at least one card type", "warning");
      return;
    }
    const ok = await store.confirm(
      `Remove card type "${currentTmpl.name}"? All cards generated by this template will be deleted.`,
      "Remove Card Type",
      { danger: true, confirmLabel: "Remove" }
    );
    if (!ok) return;
    templates = templates.filter((_, i) => i !== selectedOrd);
    selectedOrd = Math.max(0, Math.min(selectedOrd, templates.length - 1));
  }

  async function repositionCardType() {
    optionsMenuOpen = false;
    if (!currentTmpl || templates.length < 2) return;
    const raw = await store.prompt(
      `Enter new position (1–${templates.length}):`,
      String(selectedOrd + 1),
      "Reposition Card Type"
    );
    if (!raw) return;
    const pos = parseInt(raw, 10);
    if (!Number.isFinite(pos) || pos < 1 || pos > templates.length) return;
    const next = [...templates];
    const [moved] = next.splice(selectedOrd, 1);
    next.splice(pos - 1, 0, moved);
    templates = next;
    selectedOrd = pos - 1;
  }

  async function save() {
    if (!nt) return;
    saving = true;
    try {
      const updated = nt.clone();
      if (updated.config) {
        updated.config.css = css;
      }
      updated.templates = templates.map((t, idx) => {
        const clone = t.orig.clone();
        clone.ord = new UInt32({ val: idx });
        clone.name = t.name;
        const cfg = clone.config ? clone.config.clone() : new Notetype_Template_Config();
        cfg.qFormat = t.qFormat;
        cfg.aFormat = t.aFormat;
        cfg.targetDeckId = t.targetDeckId;
        clone.config = cfg;
        return clone;
      });
      await updateNotetype(updated);
      store.bumpOpRevision();
      store.toast("Card templates saved");
      close();
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to save templates", "error");
    } finally {
      saving = false;
    }
  }

  function onKeydown(ev: KeyboardEvent) {
    if ((ev.ctrlKey || ev.metaKey) && ev.shiftKey && ev.key.toLowerCase() === "f") {
      ev.preventDefault();
      flipFrontBack();
    } else if ((ev.ctrlKey || ev.metaKey) && ev.key.toLowerCase() === "s") {
      ev.preventDefault();
      void save();
    }
  }

  function close() {
    store.cardTemplates = { open: false, notetypeId: 0 };
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Card Types">
  <div class="modal-window tmpl-window">
    <div class="modal-header">
      <div class="header-left">
        <h3 class="modal-title">Card Types for {nt?.name ?? "..."}</h3>
        {#if !loading}
          <select class="anki-select card-type-select" bind:value={selectedOrd}>
            {#each templates as t, idx}
              <option value={idx}>{idx + 1}: {t.name}</option>
            {/each}
          </select>
          <div class="menu-wrap">
            <button
              type="button"
              class="anki-btn small"
              onclick={() => (optionsMenuOpen = !optionsMenuOpen)}
            >
              Options ▾
            </button>
            {#if optionsMenuOpen}
              <div class="anki-menu opts-menu">
                {#if !isCloze}
                  <button class="anki-menu-item" onclick={addCardType}>Add Card Type...</button>
                  <button
                    class="anki-menu-item danger"
                    disabled={templates.length <= 1}
                    onclick={removeCardType}
                  >
                    Remove Card Type...
                  </button>
                {/if}
                <button class="anki-menu-item" onclick={renameCardType}>Rename Card Type...</button>
                {#if templates.length > 1}
                  <button class="anki-menu-item" onclick={repositionCardType}>Reposition Card Type...</button>
                {/if}
                <div class="anki-menu-sep"></div>
                <button class="anki-menu-item" onclick={() => { optionsMenuOpen = false; flipFrontBack(); }}>
                  Flip Front & Back <span class="shortcut">Ctrl+Shift+F</span>
                </button>
              </div>
            {/if}
          </div>
        {/if}
      </div>
      <button class="modal-close" onclick={close}>×</button>
    </div>

    <div class="modal-body flush tmpl-body">
      {#if loading}
        <div class="center-msg">Loading templates...</div>
      {:else if currentTmpl}
        <div class="left-pane">
          <div class="tab-bar">
            <label class="tab-radio" class:active={activeTab === "front"}>
              <input type="radio" name="tmpl-tab" value="front" bind:group={activeTab} />
              Front Template
            </label>
            <label class="tab-radio" class:active={activeTab === "back"}>
              <input type="radio" name="tmpl-tab" value="back" bind:group={activeTab} />
              Back Template
            </label>
            <label class="tab-radio" class:active={activeTab === "styling"}>
              <input type="radio" name="tmpl-tab" value="styling" bind:group={activeTab} />
              Styling
            </label>
          </div>

          {#if activeTab === "front"}
            <textarea
              bind:this={textareaEl}
              class="code-editor"
              spellcheck="false"
              bind:value={currentTmpl.qFormat}
            ></textarea>
          {:else if activeTab === "back"}
            <textarea
              bind:this={textareaEl}
              class="code-editor"
              spellcheck="false"
              bind:value={currentTmpl.aFormat}
            ></textarea>
          {:else}
            <textarea
              bind:this={textareaEl}
              class="code-editor"
              spellcheck="false"
              bind:value={css}
            ></textarea>
          {/if}

          <div class="editor-bar">
            <div class="menu-wrap">
              <button
                type="button"
                class="anki-btn small"
                disabled={activeTab === "styling"}
                onclick={() => (fieldMenuOpen = !fieldMenuOpen)}
              >
                Add Field ▾
              </button>
              {#if fieldMenuOpen && nt}
                <div class="anki-menu field-menu">
                  {#each nt.fields as f}
                    <button class="anki-menu-item" onclick={() => insertField(f.name)}>
                      {f.name}
                    </button>
                  {/each}
                  {#if activeTab === "back"}
                    <div class="anki-menu-sep"></div>
                    <button class="anki-menu-item" onclick={() => insertField("FrontSide")}>
                      FrontSide
                    </button>
                  {/if}
                </div>
              {/if}
            </div>

            <div class="deck-override">
              <label for="tmpl-deck-ovr">Deck Override:</label>
              <select
                id="tmpl-deck-ovr"
                class="anki-select small-sel"
                value={String(currentTmpl.targetDeckId)}
                onchange={(e) => {
                  currentTmpl.targetDeckId = BigInt((e.currentTarget as HTMLSelectElement).value);
                }}
              >
                <option value="0">Off</option>
                {#each decks as d}
                  <option value={String(d.id)}>{d.name}</option>
                {/each}
              </select>
            </div>
          </div>
        </div>

        <div class="right-pane">
          <div class="preview-bar">
            <div class="preview-tabs">
              <button
                type="button"
                class="anki-btn small"
                class:primary={previewSide === "front"}
                onclick={() => (previewSide = "front")}
              >
                Front Preview
              </button>
              <button
                type="button"
                class="anki-btn small"
                class:primary={previewSide === "back"}
                onclick={() => (previewSide = "back")}
              >
                Back Preview
              </button>
            </div>
            <div class="preview-toggles">
              <label class="mini-check">
                <input type="checkbox" bind:checked={previewNight} />
                Night Mode
              </label>
              <label class="mini-check">
                <input type="checkbox" bind:checked={fillEmpty} />
                Fill empty
              </label>
            </div>
          </div>

          {#if renderError}
            <div class="render-err">{renderError}</div>
          {/if}
          <iframe
            class="preview-frame"
            srcdoc={previewSide === "front" ? frontHtml : backHtml}
            sandbox="allow-scripts allow-same-origin"
            title="Template Preview"
          ></iframe>
        </div>
      {/if}
    </div>

    <div class="modal-footer">
      <button class="anki-btn" onclick={close}>Cancel</button>
      <button class="anki-btn primary" disabled={loading || saving} onclick={save}>
        {saving ? "Saving..." : "Save"}
      </button>
    </div>
  </div>
</div>

<style>
  .tmpl-window {
    width: min(1120px, 95vw);
    height: min(740px, 90vh);
  }
  .header-left {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  .card-type-select {
    min-width: 170px;
    font-size: 13px;
  }
  .menu-wrap {
    position: relative;
  }
  .opts-menu {
    position: absolute;
    top: 100%;
    left: 0;
    margin-top: 4px;
    min-width: 210px;
    z-index: 40;
  }
  .field-menu {
    position: absolute;
    bottom: 100%;
    left: 0;
    margin-bottom: 4px;
    min-width: 170px;
    max-height: 220px;
    overflow-y: auto;
    z-index: 40;
  }
  .tmpl-body {
    display: grid;
    grid-template-columns: 1fr 1fr;
    min-height: 0;
    overflow: hidden;
  }
  .left-pane,
  .right-pane {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }
  .left-pane {
    border-right: 1px solid var(--border);
  }
  .tab-bar,
  .preview-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 8px 12px;
    background: var(--canvas-elevated);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .tab-radio {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 13px;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
  }
  .tab-radio.active {
    font-weight: 600;
    color: var(--accent);
  }
  .code-editor {
    flex: 1;
    width: 100%;
    border: none;
    outline: none;
    resize: none;
    padding: 12px;
    background: var(--canvas-code);
    color: var(--fg);
    font-family: var(--font-mono);
    font-size: 13px;
    line-height: 1.5;
    tab-size: 2;
  }
  .editor-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 8px 12px;
    border-top: 1px solid var(--border);
    background: var(--canvas-elevated);
    flex-shrink: 0;
  }
  .deck-override {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--fg-subtle);
  }
  .small-sel {
    font-size: 12px;
    padding: 3px 6px;
    max-width: 180px;
  }
  .preview-tabs,
  .preview-toggles {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .mini-check {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    cursor: pointer;
  }
  .preview-frame {
    flex: 1;
    width: 100%;
    border: none;
    background: var(--canvas);
  }
  .render-err {
    padding: 8px 12px;
    background: rgba(239, 68, 68, 0.14);
    color: var(--danger);
    font-size: 12px;
    border-bottom: 1px solid var(--border);
  }
  .center-msg {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--fg-subtle);
  }
  @media (max-width: 820px) {
    .tmpl-body {
      grid-template-columns: 1fr;
      grid-template-rows: 1fr 1fr;
    }
    .left-pane {
      border-right: none;
      border-bottom: 1px solid var(--border);
    }
  }
</style>
