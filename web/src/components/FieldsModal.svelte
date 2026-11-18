<script lang="ts">
  import { getNotetype, updateNotetype } from "@generated/backend";
  import {
    Notetype,
    Notetype_Field,
    Notetype_Field_Config,
  } from "@generated/anki/notetypes_pb";
  import { UInt32 } from "@generated/anki/generic_pb";
  import { store } from "../lib/store.svelte";

  let loading = $state(true);
  let saving = $state(false);
  let nt = $state<Notetype | null>(null);
  let selectedIdx = $state(0);

  // Editable draft fields
  let fields = $state<
    Array<{
      name: string;
      fontName: string;
      fontSize: number;
      rtl: boolean;
      sticky: boolean;
      plainText: boolean;
      collapsed: boolean;
      excludeFromSearch: boolean;
      description: string;
      preventDeletion: boolean;
      origField: Notetype_Field;
    }>
  >([]);
  let sortFieldIdx = $state(0);

  const selected = $derived(fields[selectedIdx] ?? null);

  async function load() {
    const id = store.fieldsModal.notetypeId;
    if (!id) return;
    loading = true;
    try {
      const loaded = await getNotetype({ ntid: BigInt(id) });
      nt = loaded;
      sortFieldIdx = loaded.config?.sortFieldIdx ?? 0;
      fields = loaded.fields.map((f) => ({
        name: f.name,
        fontName: f.config?.fontName || "Arial",
        fontSize: f.config?.fontSize || 20,
        rtl: f.config?.rtl ?? false,
        sticky: f.config?.sticky ?? false,
        plainText: f.config?.plainText ?? false,
        collapsed: f.config?.collapsed ?? false,
        excludeFromSearch: f.config?.excludeFromSearch ?? false,
        description: f.config?.description ?? "",
        preventDeletion: f.config?.preventDeletion ?? false,
        origField: f,
      }));
      selectedIdx = 0;
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to load fields", "error");
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (store.fieldsModal.open && store.fieldsModal.notetypeId) {
      void load();
    }
  });

  async function addField() {
    const name = await store.prompt("Field name:", "", "Add Field");
    if (!name || !name.trim()) return;
    const trimmed = name.trim();
    if (fields.some((f) => f.name.toLowerCase() === trimmed.toLowerCase())) {
      store.toast("A field with that name already exists", "warning");
      return;
    }
    const newField = new Notetype_Field({
      name: trimmed,
      config: new Notetype_Field_Config({
        fontName: "Arial",
        fontSize: 20,
        rtl: false,
        sticky: false,
        plainText: false,
        collapsed: false,
        excludeFromSearch: false,
        description: "",
        preventDeletion: false,
        other: new Uint8Array(),
      }),
    });
    fields = [
      ...fields,
      {
        name: trimmed,
        fontName: "Arial",
        fontSize: 20,
        rtl: false,
        sticky: false,
        plainText: false,
        collapsed: false,
        excludeFromSearch: false,
        description: "",
        preventDeletion: false,
        origField: newField,
      },
    ];
    selectedIdx = fields.length - 1;
  }

  async function renameField() {
    if (!selected) return;
    const name = await store.prompt("New field name:", selected.name, "Rename Field");
    if (!name || !name.trim()) return;
    const trimmed = name.trim();
    if (
      fields.some((f, i) => i !== selectedIdx && f.name.toLowerCase() === trimmed.toLowerCase())
    ) {
      store.toast("A field with that name already exists", "warning");
      return;
    }
    selected.name = trimmed;
  }

  async function deleteField() {
    if (!selected) return;
    if (fields.length <= 1) {
      store.toast("A note type must have at least one field", "warning");
      return;
    }
    if (selected.preventDeletion) {
      store.toast("This field cannot be deleted", "warning");
      return;
    }
    const ok = await store.confirm(
      `Delete field "${selected.name}"? Content in this field will be lost on all notes of this type.`,
      "Delete Field",
      { danger: true, confirmLabel: "Delete" }
    );
    if (!ok) return;
    fields = fields.filter((_, i) => i !== selectedIdx);
    if (sortFieldIdx === selectedIdx) {
      sortFieldIdx = 0;
    } else if (sortFieldIdx > selectedIdx) {
      sortFieldIdx -= 1;
    }
    selectedIdx = Math.max(0, Math.min(selectedIdx, fields.length - 1));
  }

  async function repositionField() {
    if (!selected) return;
    const raw = await store.prompt(
      `Enter new position (1–${fields.length}):`,
      String(selectedIdx + 1),
      "Reposition Field"
    );
    if (!raw) return;
    const pos = parseInt(raw, 10);
    if (!Number.isFinite(pos) || pos < 1 || pos > fields.length) {
      store.toast(`Position must be between 1 and ${fields.length}`, "warning");
      return;
    }
    const targetIdx = pos - 1;
    if (targetIdx === selectedIdx) return;
    const next = [...fields];
    const [item] = next.splice(selectedIdx, 1);
    next.splice(targetIdx, 0, item);
    // Keep sortFieldIdx tracking the same logical field
    const sortName = fields[sortFieldIdx]?.name;
    fields = next;
    const newSortIdx = fields.findIndex((f) => f.name === sortName);
    sortFieldIdx = newSortIdx >= 0 ? newSortIdx : 0;
    selectedIdx = targetIdx;
  }

  async function save() {
    if (!nt) return;
    saving = true;
    try {
      const updated = nt.clone();
      if (updated.config) {
        updated.config.sortFieldIdx = sortFieldIdx;
      }
      updated.fields = fields.map((f, idx) => {
        const clone = f.origField.clone();
        clone.ord = new UInt32({ val: idx });
        clone.name = f.name;
        const cfg = clone.config ? clone.config.clone() : new Notetype_Field_Config();
        cfg.fontName = f.fontName || "Arial";
        cfg.fontSize = Math.max(6, Math.min(200, Number(f.fontSize) || 20));
        cfg.rtl = f.rtl;
        cfg.sticky = f.sticky;
        cfg.plainText = f.plainText;
        cfg.collapsed = f.collapsed;
        cfg.excludeFromSearch = f.excludeFromSearch;
        cfg.description = f.description;
        clone.config = cfg;
        return clone;
      });
      await updateNotetype(updated);
      store.bumpOpRevision();
      store.toast("Fields saved");
      close();
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to save fields", "error");
    } finally {
      saving = false;
    }
  }

  function close() {
    store.fieldsModal = { open: false, notetypeId: 0 };
  }
</script>

<div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Fields">
  <div class="modal-window fields-window">
    <div class="modal-header">
      <h3 class="modal-title">Fields for {nt?.name ?? "..."}</h3>
      <button class="modal-close" onclick={close}>×</button>
    </div>
    <div class="modal-body">
      {#if loading}
        <div class="muted">Loading fields...</div>
      {:else}
        <div class="top-row">
          <div class="field-list" role="listbox">
            {#each fields as f, idx (idx)}
              <button
                type="button"
                class="field-item"
                class:selected={idx === selectedIdx}
                onclick={() => (selectedIdx = idx)}
              >
                <span>{idx + 1}: {f.name}</span>
                {#if idx === sortFieldIdx}
                  <span class="sort-badge">sort</span>
                {/if}
              </button>
            {/each}
          </div>
          <div class="side-btns">
            <button class="anki-btn" onclick={addField}>Add</button>
            <button class="anki-btn danger" disabled={!selected || selected.preventDeletion} onclick={deleteField}>Delete</button>
            <button class="anki-btn" disabled={!selected} onclick={renameField}>Rename</button>
            <button class="anki-btn" disabled={!selected || fields.length < 2} onclick={repositionField}>Reposition</button>
          </div>
        </div>

        {#if selected}
          <fieldset class="opt-group">
            <legend>Editing</legend>
            <div class="opt-grid">
              <div class="form-row">
                <label for="fld-font">Editing font:</label>
                <input id="fld-font" type="text" class="anki-input" bind:value={selected.fontName} />
                <label for="fld-size">Size:</label>
                <input id="fld-size" type="number" class="anki-input num" min="6" max="120" bind:value={selected.fontSize} />
              </div>
              <label class="check-row">
                <input
                  type="checkbox"
                  checked={sortFieldIdx === selectedIdx}
                  onchange={() => (sortFieldIdx = selectedIdx)}
                />
                Sort by this field in the browser
              </label>
              <label class="check-row">
                <input type="checkbox" bind:checked={selected.sticky} />
                Remember last input when adding (sticky)
              </label>
              <label class="check-row">
                <input type="checkbox" bind:checked={selected.rtl} />
                Reverse text direction (RTL)
              </label>
              <label class="check-row">
                <input type="checkbox" bind:checked={selected.plainText} />
                Use HTML editor by default
              </label>
              <label class="check-row">
                <input type="checkbox" bind:checked={selected.collapsed} />
                Collapse by default
              </label>
              <label class="check-row">
                <input type="checkbox" bind:checked={selected.excludeFromSearch} />
                Exclude from unqualified searches
              </label>
              <div class="form-row">
                <label for="fld-desc">Description:</label>
                <input
                  id="fld-desc"
                  type="text"
                  class="anki-input"
                  bind:value={selected.description}
                  placeholder="Optional hint shown in editor"
                />
              </div>
            </div>
          </fieldset>
        {/if}
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
  .fields-window {
    width: 560px;
    max-width: 94vw;
  }
  .top-row {
    display: flex;
    gap: 12px;
    margin-bottom: 12px;
  }
  .field-list {
    flex: 1;
    min-height: 150px;
    max-height: 200px;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    background: var(--canvas-inset);
    display: flex;
    flex-direction: column;
  }
  .field-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 10px;
    border: none;
    background: transparent;
    color: var(--fg);
    font-size: 13px;
    cursor: pointer;
    text-align: left;
  }
  .field-item:hover {
    background: var(--highlight-bg);
  }
  .field-item.selected {
    background: var(--accent);
    color: #fff;
  }
  .sort-badge {
    font-size: 11px;
    padding: 1px 6px;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.22);
    border: 1px solid currentColor;
  }
  .side-btns {
    width: 110px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .opt-group {
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    padding: 10px 12px;
    margin: 0;
  }
  .opt-group legend {
    font-size: 12px;
    font-weight: 600;
    color: var(--fg-subtle);
    padding: 0 4px;
  }
  .opt-grid {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .form-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }
  .form-row label {
    white-space: nowrap;
    color: var(--fg-subtle);
  }
  .form-row input[type="text"] {
    flex: 1;
  }
  .num {
    width: 68px;
  }
  .check-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    cursor: pointer;
  }
  .muted {
    color: var(--fg-subtle);
  }
</style>
