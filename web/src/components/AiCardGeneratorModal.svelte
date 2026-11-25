<script lang="ts">
  import { onMount } from "svelte";
  import { getDeckNames, getNotetypeNames } from "@generated/backend";
  import type { DeckNameId } from "@generated/anki/decks_pb";
  import type { NotetypeNameId } from "@generated/anki/notetypes_pb";
  import { store } from "../lib/store.svelte";
  import Icon from "./Icon.svelte";

  type TabMode = "text" | "image";
  type CardType = "basic" | "cloze";
  type AiProvider = "gemini" | "openai";

  interface GeneratedCard {
    id: string;
    front: string;
    back: string;
    cloze: string;
    tags: string[];
    selected: boolean;
  }

  // Active step: "configure" or "review"
  let step = $state<"configure" | "review">("configure");
  let activeTab = $state<TabMode>("text");

  // Inputs
  let sourceText = $state<string>("");
  let imageBase64 = $state<string>("");
  let imageMime = $state<string>("image/jpeg");
  let imageFileName = $state<string>("");

  // Options
  let decks = $state<DeckNameId[]>([]);
  let notetypes = $state<NotetypeNameId[]>([]);
  let selectedDeckId = $state<number>(1);
  let cardType = $state<CardType>("basic");
  let targetCount = $state<number>(5);
  let language = $state<string>("auto");

  // Provider config
  let provider = $state<AiProvider>("gemini");
  let apiKey = $state<string>("");
  let showApiKey = $state<boolean>(false);
  let modelName = $state<string>("gemini-1.5-flash");
  let baseUrl = $state<string>("");
  let showProviderSettings = $state<boolean>(false);

  // Generation state
  let loading = $state<boolean>(false);
  let errorMessage = $state<string>("");
  let generatedCards = $state<GeneratedCard[]>([]);

  const selectedCount = $derived(generatedCards.filter((c) => c.selected).length);

  onMount(async () => {
    // Load stored AI preferences
    const storedProvider = localStorage.getItem("cardly_ai_provider") as AiProvider | null;
    if (storedProvider) provider = storedProvider;

    const storedKey = localStorage.getItem("cardly_ai_api_key");
    if (storedKey) apiKey = storedKey;

    const storedModel = localStorage.getItem("cardly_ai_model");
    if (storedModel) modelName = storedModel;

    const storedBaseUrl = localStorage.getItem("cardly_ai_base_url");
    if (storedBaseUrl) baseUrl = storedBaseUrl;

    try {
      const [deckResp, ntResp] = await Promise.all([
        getDeckNames({ skipEmptyDefault: false, includeFiltered: false }, { alertOnError: false }),
        getNotetypeNames({}, { alertOnError: false }),
      ]);
      decks = deckResp.entries;
      notetypes = ntResp.entries;

      if (store.currentDeckId && decks.some((d) => Number(d.id) === store.currentDeckId)) {
        selectedDeckId = store.currentDeckId;
      } else if (decks.length > 0) {
        selectedDeckId = Number(decks[0].id);
      }
    } catch (err) {
      console.warn("Failed to load decks or notetypes:", err);
    }
  });

  function handleProviderChange(newProvider: AiProvider) {
    provider = newProvider;
    localStorage.setItem("cardly_ai_provider", newProvider);
    if (newProvider === "gemini") {
      modelName = "gemini-1.5-flash";
      baseUrl = "";
    } else {
      modelName = "gpt-4o-mini";
    }
    localStorage.setItem("cardly_ai_model", modelName);
  }

  function handleKeyChange(val: string) {
    apiKey = val;
    localStorage.setItem("cardly_ai_api_key", val);
  }

  function handleModelChange(val: string) {
    modelName = val;
    localStorage.setItem("cardly_ai_model", val);
  }

  function handleBaseUrlChange(val: string) {
    baseUrl = val;
    localStorage.setItem("cardly_ai_base_url", val);
  }

  function handleImageUpload(e: Event) {
    const input = e.target as HTMLInputElement;
    if (!input.files || input.files.length === 0) return;
    const file = input.files[0];
    imageFileName = file.name;
    imageMime = file.type || "image/jpeg";

    const reader = new FileReader();
    reader.onload = () => {
      const result = reader.result as string;
      const base64Index = result.indexOf(",");
      imageBase64 = base64Index !== -1 ? result.substring(base64Index + 1) : result;
    };
    reader.readAsDataURL(file);
  }

  function removeImage() {
    imageBase64 = "";
    imageFileName = "";
  }

  async function handleGenerate() {
    errorMessage = "";
    if (activeTab === "text" && !sourceText.trim()) {
      errorMessage = "Please enter some source text or study notes.";
      return;
    }
    if (activeTab === "image" && !imageBase64) {
      errorMessage = "Please select or drop an image file.";
      return;
    }

    loading = true;

    try {
      const payload: Record<string, unknown> = {
        mode: cardType,
        target_count: targetCount,
        language,
        provider,
        api_key: apiKey.trim(),
        model: modelName.trim(),
      };

      if (baseUrl.trim()) {
        payload.base_url = baseUrl.trim();
      }

      if (activeTab === "text") {
        payload.text = sourceText.trim();
      } else {
        payload.image_base64 = imageBase64;
        payload.image_mime = imageMime;
      }

      const res = await fetch("/api/ai/generate", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-Cardly-Api-Token": store.apiToken,
        },
        body: JSON.stringify(payload),
      });

      if (!res.ok) {
        const errJson = await res.json().catch(() => ({}));
        throw new Error(errJson.error || `Generation failed with status ${res.status}`);
      }

      const data = await res.json();
      if (!Array.isArray(data.cards) || data.cards.length === 0) {
        throw new Error("No flashcards returned from the model. Try rephrasing or using a different prompt.");
      }

      generatedCards = data.cards.map((c: any, i: number) => ({
        id: `card-${Date.now()}-${i}`,
        front: c.front || "",
        back: c.back || "",
        cloze: c.cloze || c.text || "",
        tags: Array.isArray(c.tags) ? c.tags : ["ai-generated"],
        selected: true,
      }));

      step = "review";
    } catch (err) {
      errorMessage = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  function selectAll(checked: boolean) {
    generatedCards = generatedCards.map((c) => ({ ...c, selected: checked }));
  }

  function removeCard(id: string) {
    generatedCards = generatedCards.filter((c) => c.id !== id);
  }

  async function handleBatchAdd() {
    const toAdd = generatedCards.filter((c) => c.selected);
    if (toAdd.length === 0) return;

    loading = true;
    errorMessage = "";

    try {
      // Find matching notetype id
      let notetypeId = 1;
      if (cardType === "cloze") {
        const clozeNt = notetypes.find((n) => n.name.toLowerCase().includes("cloze"));
        if (clozeNt) notetypeId = Number(clozeNt.id);
      } else {
        const basicNt = notetypes.find((n) => n.name.toLowerCase().includes("basic"));
        if (basicNt) notetypeId = Number(basicNt.id);
      }

      const payload = {
        deck_id: selectedDeckId,
        notetype_id: notetypeId,
        cards: toAdd.map((c) => ({
          front: c.front,
          back: c.back,
          text: c.cloze || c.front,
          tags: c.tags,
        })),
      };

      const res = await fetch("/api/ai/batch-add", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-Cardly-Api-Token": store.apiToken,
        },
        body: JSON.stringify(payload),
      });

      if (!res.ok) {
        const errJson = await res.json().catch(() => ({}));
        throw new Error(errJson.error || `Failed to add cards: HTTP ${res.status}`);
      }

      const resData = await res.json().catch(() => ({ added_count: toAdd.length }));
      const count = resData.added_count ?? toAdd.length;

      const targetDeck = decks.find((d) => Number(d.id) === selectedDeckId);
      const deckName = targetDeck ? targetDeck.name : "deck";

      store.toast(`Added ${count} AI flashcards to "${deckName}"!`);
      store.opRevision++;
      store.aiGeneratorOpen = false;
    } catch (err) {
      errorMessage = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  function closeModal() {
    store.aiGeneratorOpen = false;
  }
</script>

<div class="modal-backdrop" onclick={closeModal} role="presentation">
  <div
    class="modal-window ai-generator-window"
    onclick={(e) => e.stopPropagation()}
    role="dialog"
    aria-modal="true"
    aria-labelledby="ai-modal-title"
  >
    <!-- Modal Header -->
    <div class="modal-header">
      <div class="title-with-badge">
        <h2 id="ai-modal-title"><Icon name="sparkles" size={18} /> AI Card Generator</h2>
        <span class="badge-pill">FSRS Ready</span>
      </div>
      <button class="modal-close" onclick={closeModal} title="Close (Esc)" aria-label="Close">
        <Icon name="x" size={16} />
      </button>
    </div>

    <!-- Modal Body -->
    <div class="modal-body">
      {#if errorMessage}
        <div class="error-banner" role="alert">
          <span class="error-icon"><Icon name="alert-triangle" size={16} /></span>
          <span>{errorMessage}</span>
        </div>
      {/if}

      {#if step === "configure"}
        <!-- Tabs: Text vs Image -->
        <div class="tabs-nav" role="tablist">
          <button
            class="tab-btn"
            class:active={activeTab === "text"}
            onclick={() => (activeTab = "text")}
            role="tab"
            aria-selected={activeTab === "text"}
          >
            <Icon name="file-text" size={15} /> Text &amp; Notes
          </button>
          <button
            class="tab-btn"
            class:active={activeTab === "image"}
            onclick={() => (activeTab = "image")}
            role="tab"
            aria-selected={activeTab === "image"}
          >
            <Icon name="camera" size={15} /> Photo / Image OCR
          </button>
        </div>

        {#if activeTab === "text"}
          <div class="tab-pane">
            <label for="ai-source-text" class="field-label">Source Text or Lecture Notes:</label>
            <textarea
              id="ai-source-text"
              class="text-input notes-textarea"
              placeholder="Paste article excerpts, textbook chapters, vocabulary lists, or raw notes here. Cardly will distill them into high-retention atomic flashcards..."
              bind:value={sourceText}
            ></textarea>
          </div>
        {:else}
          <div class="tab-pane">
            <label for="ai-image-file" class="field-label">Upload Textbook Page or Diagram:</label>
            <div class="image-dropzone">
              {#if imageBase64}
                <div class="image-preview-wrapper">
                  <img src={`data:${imageMime};base64,${imageBase64}`} alt="Uploaded preview" class="preview-img" />
                  <div class="preview-meta">
                    <span class="filename">{imageFileName}</span>
                    <button type="button" class="btn-text-danger" onclick={removeImage}>Remove</button>
                  </div>
                </div>
              {:else}
                <div class="dropzone-empty">
                  <span class="dropzone-icon"><Icon name="camera" size={28} /></span>
                  <p>Drag and drop a photo or document screenshot, or browse files</p>
                  <input
                    id="ai-image-file"
                    type="file"
                    accept="image/png,image/jpeg,image/webp"
                    class="file-input"
                    onchange={handleImageUpload}
                  />
                </div>
              {/if}
            </div>
            <p class="field-hint">Multimodal AI analyzes visual text, diagrams, and formulas directly.</p>
          </div>
        {/if}

        <!-- Configuration Grid -->
        <div class="config-grid">
          <div class="config-item">
            <label for="ai-deck-select" class="field-label">Target Deck:</label>
            <select id="ai-deck-select" class="select-input" bind:value={selectedDeckId}>
              {#each decks as d}
                <option value={Number(d.id)}>{d.name}</option>
              {/each}
            </select>
          </div>

          <div class="config-item">
            <label for="ai-card-type" class="field-label">Card Type:</label>
            <select id="ai-card-type" class="select-input" bind:value={cardType}>
              <option value="basic">Basic (Question &amp; Answer)</option>
              <option value="cloze">Cloze Deletion (&#123;&#123;c1::...&#125;&#125;)</option>
            </select>
          </div>

          <div class="config-item">
            <label for="ai-card-count" class="field-label">Card Count: {targetCount}</label>
            <input id="ai-card-count" type="range" min="3" max="20" step="1" class="range-slider" bind:value={targetCount} />
          </div>

          <div class="config-item">
            <label for="ai-lang-select" class="field-label">Language:</label>
            <select id="ai-lang-select" class="select-input" bind:value={language}>
              <option value="auto">Auto (Match text)</option>
              <option value="en">English</option>
              <option value="ru">Russian (Русский)</option>
              <option value="kk">Kazakh (Қазақша)</option>
            </select>
          </div>
        </div>

        <!-- AI Provider Settings Accordion -->
        <div class="provider-section">
          <button
            type="button"
            class="provider-toggle-btn"
            onclick={() => (showProviderSettings = !showProviderSettings)}
            aria-expanded={showProviderSettings}
          >
            <span><Icon name="settings" size={14} /> AI Provider: <strong>{provider === "gemini" ? "Google Gemini" : "OpenAI / Custom"}</strong> ({modelName})</span>
            <span class="toggle-arrow">{showProviderSettings ? "▲" : "▼"}</span>
          </button>

          {#if showProviderSettings}
            <div class="provider-settings-panel">
              <div class="provider-row">
                <span class="sub-label">Provider:</span>
                <div class="radio-group">
                  <label class="radio-label">
                    <input
                      type="radio"
                      name="ai-provider"
                      value="gemini"
                      checked={provider === "gemini"}
                      onchange={() => handleProviderChange("gemini")}
                    />
                    <span>Google Gemini</span>
                  </label>
                  <label class="radio-label">
                    <input
                      type="radio"
                      name="ai-provider"
                      value="openai"
                      checked={provider === "openai"}
                      onchange={() => handleProviderChange("openai")}
                    />
                    <span>OpenAI / Ollama</span>
                  </label>
                </div>
              </div>

              <div class="field-group">
                <label for="ai-api-key" class="field-label">API Key:</label>
                <div class="key-input-row">
                  <input
                    id="ai-api-key"
                    type={showApiKey ? "text" : "password"}
                    class="text-input"
                    placeholder={provider === "gemini" ? "AIzaSy..." : "sk-..."}
                    value={apiKey}
                    oninput={(e) => handleKeyChange((e.target as HTMLInputElement).value)}
                  />
                  <button type="button" class="btn-toggle-key" onclick={() => (showApiKey = !showApiKey)}>
                    {showApiKey ? "Hide" : "Show"}
                  </button>
                </div>
                <span class="field-hint">Stored strictly in local browser storage. Never logged or shared.</span>
              </div>

              <div class="field-group">
                <label for="ai-model-name" class="field-label">Model Name:</label>
                <input
                  id="ai-model-name"
                  type="text"
                  class="text-input"
                  value={modelName}
                  oninput={(e) => handleModelChange((e.target as HTMLInputElement).value)}
                />
              </div>

              {#if provider === "openai"}
                <div class="field-group">
                  <label for="ai-base-url" class="field-label">Custom Base URL (optional):</label>
                  <input
                    id="ai-base-url"
                    type="text"
                    class="text-input"
                    placeholder="http://localhost:11434/v1 for Ollama"
                    value={baseUrl}
                    oninput={(e) => handleBaseUrlChange((e.target as HTMLInputElement).value)}
                  />
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {:else}
        <!-- Review & Edit Step -->
        <div class="review-header">
          <div class="review-stats">
            <span>Generated <strong>{generatedCards.length}</strong> cards (<strong>{selectedCount}</strong> selected)</span>
          </div>
          <div class="review-actions">
            <button type="button" class="btn-small" onclick={() => selectAll(true)}>Select All</button>
            <button type="button" class="btn-small" onclick={() => selectAll(false)}>Deselect All</button>
            <button type="button" class="btn-small btn-secondary" onclick={() => (step = "configure")}><Icon name="settings" size={13} /> Change Prompt</button>
          </div>
        </div>

        <div class="cards-review-list">
          {#each generatedCards as card, i (card.id)}
            <div class="review-card-item" class:disabled={!card.selected}>
              <div class="card-item-header">
                <label class="card-checkbox-label">
                  <input type="checkbox" bind:checked={card.selected} />
                  <span class="card-num">Card #{i + 1}</span>
                </label>
                <button type="button" class="btn-delete-card" onclick={() => removeCard(card.id)} title="Delete card" aria-label="Delete card">
                  <Icon name="x" size={14} />
                </button>
              </div>

              {#if cardType === "basic"}
                <div class="card-field-row">
                  <span class="field-tag">Front:</span>
                  <input type="text" class="text-input card-front-input" bind:value={card.front} />
                </div>
                <div class="card-field-row">
                  <span class="field-tag">Back:</span>
                  <input type="text" class="text-input card-back-input" bind:value={card.back} />
                </div>
              {:else}
                <div class="card-field-row">
                  <span class="field-tag">Cloze:</span>
                  <textarea class="text-input card-cloze-input" rows="2" bind:value={card.cloze}></textarea>
                </div>
              {/if}

              <div class="card-field-row">
                <span class="field-tag">Tags:</span>
                <input
                  type="text"
                  class="text-input card-tags-input"
                  value={card.tags.join(" ")}
                  oninput={(e) => {
                    card.tags = (e.target as HTMLInputElement).value.split(/\s+/).filter(Boolean);
                  }}
                />
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Modal Footer -->
    <div class="modal-footer">
      {#if step === "configure"}
        <button type="button" class="btn-secondary" onclick={closeModal} disabled={loading}>Cancel</button>
        <button type="button" class="btn-primary" onclick={handleGenerate} disabled={loading}>
          {#if loading}
            <span class="spinner-small"></span>
            <span>Generating Flashcards...</span>
          {:else}
            <span><Icon name="sparkles" size={15} /> Generate Flashcards</span>
          {/if}
        </button>
      {:else}
        <button type="button" class="btn-secondary" onclick={() => (step = "configure")} disabled={loading}>
          ← Back to Settings
        </button>
        <button type="button" class="btn-primary" onclick={handleBatchAdd} disabled={loading || selectedCount === 0}>
          {#if loading}
            <span class="spinner-small"></span>
            <span>Adding to Deck...</span>
          {:else}
            <span>Add {selectedCount} Cards to Deck</span>
          {/if}
        </button>
      {/if}
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    background: rgba(0, 0, 0, 0.65);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
  }

  .ai-generator-window {
    width: min(840px, 96vw);
    max-height: min(840px, 92vh);
    background: var(--canvas-elevated);
    color: var(--fg);
    border: 1px solid var(--border);
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.4);
    overflow: hidden;
  }

  .modal-header {
    padding: 14px 20px;
    background: var(--canvas);
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .title-with-badge {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .title-with-badge h2 {
    font-size: 1.15rem;
    font-weight: 700;
    margin: 0;
  }

  .badge-pill {
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    background: rgba(245, 158, 11, 0.15);
    color: #f59e0b;
    border: 1px solid rgba(245, 158, 11, 0.3);
    padding: 2px 8px;
    border-radius: 10px;
  }

  .modal-close {
    background: none;
    border: none;
    color: var(--fg-subtle);
    font-size: 1.1rem;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 4px;
  }

  .modal-close:hover {
    color: var(--fg);
    background: var(--canvas-elevated);
  }

  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .error-banner {
    background: rgba(239, 68, 68, 0.12);
    border: 1px solid #ef4444;
    color: #ef4444;
    padding: 10px 14px;
    border-radius: 8px;
    font-size: 0.9rem;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .tabs-nav {
    display: flex;
    gap: 8px;
    border-bottom: 1px solid var(--border);
    padding-bottom: 8px;
  }

  .tab-btn {
    background: none;
    border: none;
    padding: 8px 16px;
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--fg-subtle);
    cursor: pointer;
    border-radius: 6px;
  }

  .tab-btn.active {
    background: var(--primary);
    color: #ffffff;
  }

  .field-label {
    display: block;
    font-size: 0.85rem;
    font-weight: 600;
    margin-bottom: 6px;
    color: var(--fg);
  }

  .field-hint {
    font-size: 0.75rem;
    color: var(--fg-subtle);
    margin-top: 4px;
  }

  .text-input, .select-input {
    width: 100%;
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--canvas);
    color: var(--fg);
    font-family: inherit;
    font-size: 0.9rem;
  }

  .notes-textarea {
    height: 140px;
    resize: vertical;
  }

  .image-dropzone {
    border: 2px dashed var(--border);
    border-radius: 8px;
    padding: 24px;
    text-align: center;
    background: var(--canvas);
    position: relative;
    cursor: pointer;
  }

  .dropzone-empty {
    position: relative;
  }

  .dropzone-icon {
    font-size: 2rem;
    display: block;
    margin-bottom: 8px;
  }

  .file-input {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
    width: 100%;
    height: 100%;
  }

  .image-preview-wrapper {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }

  .preview-img {
    max-height: 140px;
    border-radius: 6px;
    border: 1px solid var(--border);
  }

  .preview-meta {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 0.85rem;
  }

  .btn-text-danger {
    background: none;
    border: none;
    color: #ef4444;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .config-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 16px;
  }

  .range-slider {
    width: 100%;
    accent-color: var(--primary);
  }

  .provider-section {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--canvas);
    overflow: hidden;
  }

  .provider-toggle-btn {
    width: 100%;
    padding: 10px 14px;
    background: none;
    border: none;
    color: var(--fg);
    font-size: 0.85rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    cursor: pointer;
  }

  .provider-settings-panel {
    padding: 14px;
    border-top: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .provider-row {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .sub-label {
    font-size: 0.85rem;
    font-weight: 600;
  }

  .radio-group {
    display: flex;
    gap: 14px;
  }

  .radio-label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.85rem;
    cursor: pointer;
  }

  .key-input-row {
    display: flex;
    gap: 8px;
  }

  .btn-toggle-key {
    padding: 6px 12px;
    background: var(--canvas-elevated);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--fg);
    cursor: pointer;
    font-size: 0.8rem;
  }

  /* Review View */
  .review-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 12px;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border);
  }

  .review-stats {
    font-size: 0.95rem;
  }

  .review-actions {
    display: flex;
    gap: 8px;
  }

  .btn-small {
    padding: 4px 10px;
    font-size: 0.8rem;
    background: var(--canvas);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--fg);
    cursor: pointer;
  }

  .cards-review-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .review-card-item {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px;
    background: var(--canvas);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .review-card-item.disabled {
    opacity: 0.45;
  }

  .card-item-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .card-checkbox-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 700;
    font-size: 0.85rem;
    cursor: pointer;
  }

  .btn-delete-card {
    background: none;
    border: none;
    color: var(--fg-subtle);
    cursor: pointer;
  }

  .btn-delete-card:hover {
    color: #ef4444;
  }

  .card-field-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .field-tag {
    width: 50px;
    font-size: 0.75rem;
    font-weight: 700;
    color: var(--fg-subtle);
    text-transform: uppercase;
  }

  /* Modal Footer */
  .modal-footer {
    padding: 14px 20px;
    background: var(--canvas);
    border-top: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 12px;
  }

  .btn-primary, .btn-secondary {
    padding: 8px 18px;
    font-size: 0.9rem;
    font-weight: 600;
    border-radius: 6px;
    cursor: pointer;
    border: 1px solid transparent;
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .btn-primary {
    background: var(--primary);
    color: #ffffff;
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: var(--canvas-elevated);
    border-color: var(--border);
    color: var(--fg);
  }

  .spinner-small {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255, 255, 255, 0.4);
    border-top-color: #ffffff;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
