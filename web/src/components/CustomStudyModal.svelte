<script lang="ts">
  import { customStudy, customStudyDefaults } from "@generated/backend";
  import {
    CustomStudyDefaultsResponse,
    CustomStudyRequest_Cram_CramKind,
  } from "@generated/anki/scheduler_pb";
  import { store } from "../lib/store.svelte";

  type Mode = "new" | "review" | "forgot" | "ahead" | "preview" | "cram";

  let loading = $state(true);
  let submitting = $state(false);
  let defaults = $state<CustomStudyDefaultsResponse | null>(null);
  let mode = $state<Mode>("new");

  let newDelta = $state(20);
  let reviewDelta = $state(50);
  let forgotDays = $state(1);
  let aheadDays = $state(1);
  let previewDays = $state(1);
  let cramLimit = $state(100);
  let cramKind = $state<CustomStudyRequest_Cram_CramKind>(CustomStudyRequest_Cram_CramKind.DUE);
  let includeTags = $state<string[]>([]);
  let excludeTags = $state<string[]>([]);

  async function loadDefaults() {
    loading = true;
    try {
      defaults = await customStudyDefaults({ deckId: BigInt(store.currentDeckId) });
      newDelta = Math.max(1, defaults.extendNew || 20);
      reviewDelta = Math.max(1, defaults.extendReview || 50);
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to load custom study options", "error");
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (store.customStudyOpen) {
      void loadDefaults();
    }
  });

  function toggleTag(list: "include" | "exclude", tag: string) {
    if (list === "include") {
      includeTags = includeTags.includes(tag)
        ? includeTags.filter((t) => t !== tag)
        : [...includeTags, tag];
      excludeTags = excludeTags.filter((t) => t !== tag);
    } else {
      excludeTags = excludeTags.includes(tag)
        ? excludeTags.filter((t) => t !== tag)
        : [...excludeTags, tag];
      includeTags = includeTags.filter((t) => t !== tag);
    }
  }

  async function submit() {
    submitting = true;
    try {
      const deckId = BigInt(store.currentDeckId);
      let value:
        | { case: "newLimitDelta"; value: number }
        | { case: "reviewLimitDelta"; value: number }
        | { case: "forgotDays"; value: number }
        | { case: "reviewAheadDays"; value: number }
        | { case: "previewDays"; value: number }
        | {
            case: "cram";
            value: {
              kind: CustomStudyRequest_Cram_CramKind;
              cardLimit: number;
              tagsToInclude: string[];
              tagsToExclude: string[];
            };
          };

      if (mode === "new") {
        value = { case: "newLimitDelta", value: newDelta };
      } else if (mode === "review") {
        value = { case: "reviewLimitDelta", value: reviewDelta };
      } else if (mode === "forgot") {
        value = { case: "forgotDays", value: forgotDays };
      } else if (mode === "ahead") {
        value = { case: "reviewAheadDays", value: aheadDays };
      } else if (mode === "preview") {
        value = { case: "previewDays", value: previewDays };
      } else {
        value = {
          case: "cram",
          value: {
            kind: cramKind,
            cardLimit: cramLimit,
            tagsToInclude: includeTags,
            tagsToExclude: excludeTags,
          },
        };
      }

      await customStudy({ deckId, value });
      store.bumpOpRevision();
      store.customStudyOpen = false;
      if (mode === "forgot" || mode === "ahead" || mode === "preview" || mode === "cram") {
        store.toast("Custom Study session created");
        store.view = "overview";
      } else {
        store.toast("Today's study limit increased");
      }
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Custom study failed", "error");
    } finally {
      submitting = false;
    }
  }

  function close() {
    store.customStudyOpen = false;
  }
</script>

<div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Custom Study">
  <div class="modal-window cs-window">
    <div class="modal-header">
      <h3 class="modal-title">Custom Study</h3>
      <button class="modal-close" onclick={close}>×</button>
    </div>
    <div class="modal-body">
      {#if loading}
        <div class="muted">Loading custom study options...</div>
      {:else}
        <div class="option-list">
          <label class="radio-row">
            <input type="radio" name="cs-mode" value="new" bind:group={mode} />
            <span>Increase today's new card limit</span>
          </label>
          {#if mode === "new"}
            <div class="sub-box">
              <div class="inline-field">
                <span>Increase today's new card limit by</span>
                <input type="number" class="anki-input num" min="1" max="9999" bind:value={newDelta} />
                <span>cards</span>
              </div>
              {#if defaults}
                <div class="muted small">
                  Available new cards: {defaults.availableNew}
                  {#if defaults.availableNewInChildren > 0}
                    (+ {defaults.availableNewInChildren} in subdecks)
                  {/if}
                </div>
              {/if}
            </div>
          {/if}

          <label class="radio-row">
            <input type="radio" name="cs-mode" value="review" bind:group={mode} />
            <span>Increase today's review card limit</span>
          </label>
          {#if mode === "review"}
            <div class="sub-box">
              <div class="inline-field">
                <span>Increase today's review limit by</span>
                <input type="number" class="anki-input num" min="1" max="9999" bind:value={reviewDelta} />
                <span>cards</span>
              </div>
              {#if defaults}
                <div class="muted small">
                  Available review cards: {defaults.availableReview}
                  {#if defaults.availableReviewInChildren > 0}
                    (+ {defaults.availableReviewInChildren} in subdecks)
                  {/if}
                </div>
              {/if}
            </div>
          {/if}

          <label class="radio-row">
            <input type="radio" name="cs-mode" value="forgot" bind:group={mode} />
            <span>Review forgotten cards</span>
          </label>
          {#if mode === "forgot"}
            <div class="sub-box inline-field">
              <span>Review cards forgotten in last</span>
              <input type="number" class="anki-input num" min="1" max="999" bind:value={forgotDays} />
              <span>day(s)</span>
            </div>
          {/if}

          <label class="radio-row">
            <input type="radio" name="cs-mode" value="ahead" bind:group={mode} />
            <span>Review ahead</span>
          </label>
          {#if mode === "ahead"}
            <div class="sub-box inline-field">
              <span>Review ahead by</span>
              <input type="number" class="anki-input num" min="1" max="999" bind:value={aheadDays} />
              <span>day(s)</span>
            </div>
          {/if}

          <label class="radio-row">
            <input type="radio" name="cs-mode" value="preview" bind:group={mode} />
            <span>Preview new cards</span>
          </label>
          {#if mode === "preview"}
            <div class="sub-box inline-field">
              <span>Preview new cards added in the last</span>
              <input type="number" class="anki-input num" min="1" max="999" bind:value={previewDays} />
              <span>day(s)</span>
            </div>
          {/if}

          <label class="radio-row">
            <input type="radio" name="cs-mode" value="cram" bind:group={mode} />
            <span>Study by card state or tag</span>
          </label>
          {#if mode === "cram"}
            <div class="sub-box cram-box">
              <div class="inline-field">
                <span>Select</span>
                <input type="number" class="anki-input num" min="1" max="9999" bind:value={cramLimit} />
                <span>cards from the deck:</span>
              </div>
              <div class="cram-kinds">
                <label><input type="radio" value={CustomStudyRequest_Cram_CramKind.NEW} bind:group={cramKind} /> New cards only</label>
                <label><input type="radio" value={CustomStudyRequest_Cram_CramKind.DUE} bind:group={cramKind} /> Due cards only</label>
                <label><input type="radio" value={CustomStudyRequest_Cram_CramKind.REVIEW} bind:group={cramKind} /> All review cards in random order (don't reschedule)</label>
                <label><input type="radio" value={CustomStudyRequest_Cram_CramKind.ALL} bind:group={cramKind} /> All cards in random order (don't reschedule)</label>
              </div>
              {#if defaults && defaults.tags.length > 0}
                <div class="tags-section">
                  <div class="muted small">Filter tags (click + to require, − to exclude):</div>
                  <div class="tag-chips">
                    {#each defaults.tags as t}
                      <div class="tag-chip">
                        <span class="tag-name">{t.name}</span>
                        <button
                          type="button"
                          class="chip-btn"
                          class:active-inc={includeTags.includes(t.name)}
                          onclick={() => toggleTag("include", t.name)}
                          title="Require tag"
                        >+</button>
                        <button
                          type="button"
                          class="chip-btn"
                          class:active-exc={excludeTags.includes(t.name)}
                          onclick={() => toggleTag("exclude", t.name)}
                          title="Exclude tag"
                        >−</button>
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {/if}
    </div>
    <div class="modal-footer">
      <button class="anki-btn" onclick={close}>Cancel</button>
      <button class="anki-btn primary" disabled={loading || submitting} onclick={submit}>
        {submitting ? "Working..." : "OK"}
      </button>
    </div>
  </div>
</div>

<style>
  .cs-window {
    width: 520px;
    max-width: 94vw;
  }
  .option-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .radio-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    cursor: pointer;
    padding: 4px 0;
  }
  .sub-box {
    margin: 2px 0 8px 24px;
    padding: 10px 12px;
    background: var(--canvas-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: var(--border-radius);
  }
  .inline-field {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    font-size: 13px;
  }
  .num {
    width: 78px;
  }
  .cram-box {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .cram-kinds {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
  }
  .cram-kinds label {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
  }
  .tags-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-height: 140px;
    overflow-y: auto;
  }
  .tag-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .tag-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-radius: 12px;
    font-size: 12px;
    background: var(--canvas);
  }
  .chip-btn {
    border: none;
    background: transparent;
    color: var(--fg-subtle);
    cursor: pointer;
    font-weight: 700;
    padding: 0 3px;
    border-radius: 4px;
  }
  .chip-btn.active-inc {
    background: var(--stat-review);
    color: #fff;
  }
  .chip-btn.active-exc {
    background: var(--danger);
    color: #fff;
  }
  .muted {
    color: var(--fg-subtle);
  }
  .small {
    font-size: 12px;
    margin-top: 6px;
  }
</style>
