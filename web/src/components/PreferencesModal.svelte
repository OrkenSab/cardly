<script lang="ts">
  import { getPreferences, setPreferences } from "@generated/backend";
  import {
    Preferences,
    Preferences_BackupLimits,
    Preferences_Editing,
    Preferences_Reviewing,
    Preferences_Scheduling,
    Preferences_Scheduling_NewReviewMix,
  } from "@generated/anki/config_pb";
  import {
    createBackupNow,
    listBackups,
    restoreBackup,
    type BackupEntry,
  } from "../lib/api";
  import { store, type ThemeMode } from "../lib/store.svelte";

  type PrefTab = "appearance" | "review" | "editing" | "syncing" | "backups";

  let activeTab = $state<PrefTab>("appearance");
  let loading = $state(true);
  let saving = $state(false);
  let rawPrefs = $state<Preferences | null>(null);

  // Appearance (client-side)
  let theme = $state<ThemeMode>(store.theme);
  let minimalistMode = $state(store.minimalistMode);
  let hideTopBar = $state(store.hideTopBar);
  let hideBottomBar = $state(store.hideBottomBar);
  let spacebarRates = $state(store.spacebarRatesCard);
  let key1 = $state(store.answerKeys[1] ?? "1");
  let key2 = $state(store.answerKeys[2] ?? "2");
  let key3 = $state(store.answerKeys[3] ?? "3");
  let key4 = $state(store.answerKeys[4] ?? "4");

  // Scheduling & Reviewing (rslib Preferences)
  let rollover = $state(4);
  let learnAheadMins = $state(20);
  let newReviewMix = $state<Preferences_Scheduling_NewReviewMix>(
    Preferences_Scheduling_NewReviewMix.DISTRIBUTE
  );
  let dayLearnFirst = $state(false);
  let timeLimitMins = $state(0);
  let showRemainingDueCounts = $state(true);
  let showIntervalsOnButtons = $state(true);
  let hideAudioPlayButtons = $state(false);
  let interruptAudioWhenAnswering = $state(true);

  // Editing
  let addingDefaultsToCurrentDeck = $state(true);
  let pasteImagesAsPng = $state(false);
  let pasteStripsFormatting = $state(false);
  let defaultSearchText = $state("");
  let ignoreAccentsInSearch = $state(false);
  let renderLatex = $state(true);

  // Backups
  let backupDaily = $state(8);
  let backupWeekly = $state(4);
  let backupMonthly = $state(6);
  let backupMinMins = $state(30);
  let backups = $state<BackupEntry[]>([]);
  let loadingBackups = $state(false);

  const syncServerUrl = $derived(window.location.origin);

  async function load() {
    loading = true;
    theme = store.theme;
    minimalistMode = store.minimalistMode;
    hideTopBar = store.hideTopBar;
    hideBottomBar = store.hideBottomBar;
    spacebarRates = store.spacebarRatesCard;
    key1 = store.answerKeys[1] ?? "1";
    key2 = store.answerKeys[2] ?? "2";
    key3 = store.answerKeys[3] ?? "3";
    key4 = store.answerKeys[4] ?? "4";
    try {
      const p = await getPreferences({});
      rawPrefs = p;
      if (p.scheduling) {
        rollover = p.scheduling.rollover;
        learnAheadMins = Math.round(p.scheduling.learnAheadSecs / 60);
        newReviewMix = p.scheduling.newReviewMix;
        dayLearnFirst = p.scheduling.dayLearnFirst;
      }
      if (p.reviewing) {
        timeLimitMins = Math.round(p.reviewing.timeLimitSecs / 60);
        showRemainingDueCounts = p.reviewing.showRemainingDueCounts;
        showIntervalsOnButtons = p.reviewing.showIntervalsOnButtons;
        hideAudioPlayButtons = p.reviewing.hideAudioPlayButtons;
        interruptAudioWhenAnswering = p.reviewing.interruptAudioWhenAnswering;
      }
      if (p.editing) {
        addingDefaultsToCurrentDeck = p.editing.addingDefaultsToCurrentDeck;
        pasteImagesAsPng = p.editing.pasteImagesAsPng;
        pasteStripsFormatting = p.editing.pasteStripsFormatting;
        defaultSearchText = p.editing.defaultSearchText;
        ignoreAccentsInSearch = p.editing.ignoreAccentsInSearch;
        renderLatex = p.editing.renderLatex;
      }
      if (p.backups) {
        backupDaily = p.backups.daily;
        backupWeekly = p.backups.weekly;
        backupMonthly = p.backups.monthly;
        backupMinMins = p.backups.minimumIntervalMins;
      }
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to load preferences", "error");
    } finally {
      loading = false;
    }
  }

  async function loadBackupList() {
    loadingBackups = true;
    try {
      backups = await listBackups();
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to list backups", "error");
    } finally {
      loadingBackups = false;
    }
  }

  $effect(() => {
    if (store.preferencesOpen) {
      void load();
      void loadBackupList();
    }
  });

  async function save() {
    saving = true;
    try {
      store.setTheme(theme);
      store.setMinimalist(minimalistMode);
      store.setHideBars(hideTopBar, hideBottomBar);
      store.setAnswerKeys(
        {
          1: (key1 || "1").slice(0, 1),
          2: (key2 || "2").slice(0, 1),
          3: (key3 || "3").slice(0, 1),
          4: (key4 || "4").slice(0, 1),
        },
        spacebarRates
      );

      const next = rawPrefs ? rawPrefs.clone() : new Preferences();
      const sched = next.scheduling ? next.scheduling.clone() : new Preferences_Scheduling();
      sched.rollover = Math.max(0, Math.min(23, Number(rollover) || 0));
      sched.learnAheadSecs = Math.max(0, Number(learnAheadMins) || 0) * 60;
      sched.newReviewMix = newReviewMix;
      sched.dayLearnFirst = dayLearnFirst;
      next.scheduling = sched;

      const rev = next.reviewing ? next.reviewing.clone() : new Preferences_Reviewing();
      rev.timeLimitSecs = Math.max(0, Number(timeLimitMins) || 0) * 60;
      rev.showRemainingDueCounts = showRemainingDueCounts;
      rev.showIntervalsOnButtons = showIntervalsOnButtons;
      rev.hideAudioPlayButtons = hideAudioPlayButtons;
      rev.interruptAudioWhenAnswering = interruptAudioWhenAnswering;
      next.reviewing = rev;

      const ed = next.editing ? next.editing.clone() : new Preferences_Editing();
      ed.addingDefaultsToCurrentDeck = addingDefaultsToCurrentDeck;
      ed.pasteImagesAsPng = pasteImagesAsPng;
      ed.pasteStripsFormatting = pasteStripsFormatting;
      ed.defaultSearchText = defaultSearchText;
      ed.ignoreAccentsInSearch = ignoreAccentsInSearch;
      ed.renderLatex = renderLatex;
      next.editing = ed;

      const bk = next.backups ? next.backups.clone() : new Preferences_BackupLimits();
      bk.daily = Math.max(0, Number(backupDaily) || 0);
      bk.weekly = Math.max(0, Number(backupWeekly) || 0);
      bk.monthly = Math.max(0, Number(backupMonthly) || 0);
      bk.minimumIntervalMins = Math.max(1, Number(backupMinMins) || 30);
      next.backups = bk;

      await setPreferences(next);
      store.bumpOpRevision();
      store.toast("Preferences saved");
      close();
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to save preferences", "error");
    } finally {
      saving = false;
    }
  }

  async function handleCreateBackup() {
    try {
      await createBackupNow();
      store.toast("Backup created");
      await loadBackupList();
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Backup failed", "error");
    }
  }

  async function handleRestoreBackup(name: string) {
    const ok = await store.confirm(
      `Restore backup "${name}"? Your current collection will be replaced with this backup.`,
      "Restore Backup",
      { danger: true, confirmLabel: "Restore" }
    );
    if (!ok) return;
    try {
      await restoreBackup(name);
      store.toast("Backup restored");
      store.view = "decks";
      close();
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Restore failed", "error");
    }
  }

  function formatBytes(n: number): string {
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    return `${(n / (1024 * 1024)).toFixed(2)} MB`;
  }

  function formatDate(secs: number): string {
    return new Date(secs * 1000).toLocaleString();
  }

  function copySyncUrl() {
    void navigator.clipboard?.writeText(syncServerUrl);
    store.toast("Sync server URL copied");
  }

  function close() {
    store.preferencesOpen = false;
  }
</script>

<div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Preferences">
  <div class="modal-window pref-window">
    <div class="modal-header">
      <h3 class="modal-title">Preferences</h3>
      <button class="modal-close" onclick={close}>×</button>
    </div>

    <div class="pref-tabs" role="tablist">
      <button type="button" class="pref-tab" class:active={activeTab === "appearance"} onclick={() => (activeTab = "appearance")}>Appearance</button>
      <button type="button" class="pref-tab" class:active={activeTab === "review"} onclick={() => (activeTab = "review")}>Review</button>
      <button type="button" class="pref-tab" class:active={activeTab === "editing"} onclick={() => (activeTab = "editing")}>Editing</button>
      <button type="button" class="pref-tab" class:active={activeTab === "syncing"} onclick={() => (activeTab = "syncing")}>Syncing</button>
      <button type="button" class="pref-tab" class:active={activeTab === "backups"} onclick={() => (activeTab = "backups")}>Backups</button>
    </div>

    <div class="modal-body pref-body">
      {#if loading}
        <div class="muted">Loading preferences...</div>
      {:else if activeTab === "appearance"}
        <div class="form-row">
          <label for="pref-theme">Theme:</label>
          <select id="pref-theme" class="anki-select" bind:value={theme}>
            <option value="auto">Follow system</option>
            <option value="light">Light</option>
            <option value="dark">Dark</option>
          </select>
        </div>
        <label class="check-row">
          <input type="checkbox" bind:checked={minimalistMode} />
          Minimalist mode (flatter buttons & controls)
        </label>
        <label class="check-row">
          <input type="checkbox" bind:checked={hideTopBar} />
          Hide top bar during review
        </label>
        <label class="check-row">
          <input type="checkbox" bind:checked={hideBottomBar} />
          Hide bottom bar during review
        </label>
      {:else if activeTab === "review"}
        <div class="form-row">
          <label for="pref-rollover">Next day starts at:</label>
          <input id="pref-rollover" type="number" class="anki-input num" min="0" max="23" bind:value={rollover} />
          <span class="unit">hours past midnight</span>
        </div>
        <div class="form-row">
          <label for="pref-ahead">Learn ahead limit:</label>
          <input id="pref-ahead" type="number" class="anki-input num" min="0" max="1440" bind:value={learnAheadMins} />
          <span class="unit">mins</span>
        </div>
        <div class="form-row">
          <label for="pref-timebox">Timebox time limit:</label>
          <input id="pref-timebox" type="number" class="anki-input num" min="0" max="1440" bind:value={timeLimitMins} />
          <span class="unit">mins (0 = disabled)</span>
        </div>
        <div class="form-row">
          <label for="pref-mix">New/review order:</label>
          <select id="pref-mix" class="anki-select" bind:value={newReviewMix}>
            <option value={Preferences_Scheduling_NewReviewMix.DISTRIBUTE}>Mix with reviews</option>
            <option value={Preferences_Scheduling_NewReviewMix.REVIEWS_FIRST}>Show after reviews</option>
            <option value={Preferences_Scheduling_NewReviewMix.NEW_FIRST}>Show before reviews</option>
          </select>
        </div>
        <label class="check-row">
          <input type="checkbox" bind:checked={dayLearnFirst} />
          Show interday learning cards before reviews
        </label>
        <label class="check-row">
          <input type="checkbox" bind:checked={showIntervalsOnButtons} />
          Show next review time above answer buttons
        </label>
        <label class="check-row">
          <input type="checkbox" bind:checked={showRemainingDueCounts} />
          Show remaining card count during review
        </label>
        <label class="check-row">
          <input type="checkbox" bind:checked={hideAudioPlayButtons} />
          Hide audio play buttons on cards
        </label>
        <label class="check-row">
          <input type="checkbox" bind:checked={interruptAudioWhenAnswering} />
          Interrupt current audio when answering
        </label>
        <label class="check-row">
          <input type="checkbox" bind:checked={spacebarRates} />
          Spacebar / Enter answers card as Good
        </label>

        <fieldset class="keys-group">
          <legend>Answer Shortcuts</legend>
          <div class="keys-row">
            <label>Again: <input type="text" class="anki-input key-in" maxlength="1" bind:value={key1} /></label>
            <label>Hard: <input type="text" class="anki-input key-in" maxlength="1" bind:value={key2} /></label>
            <label>Good: <input type="text" class="anki-input key-in" maxlength="1" bind:value={key3} /></label>
            <label>Easy: <input type="text" class="anki-input key-in" maxlength="1" bind:value={key4} /></label>
          </div>
        </fieldset>
      {:else if activeTab === "editing"}
        <label class="check-row">
          <input type="checkbox" bind:checked={addingDefaultsToCurrentDeck} />
          Default deck when adding: Current deck (uncheck to remember by note type)
        </label>
        <label class="check-row">
          <input type="checkbox" bind:checked={pasteStripsFormatting} />
          Paste without Shift strips formatting
        </label>
        <label class="check-row">
          <input type="checkbox" bind:checked={pasteImagesAsPng} />
          Paste clipboard images as PNG
        </label>
        <label class="check-row">
          <input type="checkbox" bind:checked={ignoreAccentsInSearch} />
          Ignore accents in search (slower)
        </label>
        <label class="check-row">
          <input type="checkbox" bind:checked={renderLatex} />
          Render LaTeX tags
        </label>
        <div class="form-row">
          <label for="pref-defsearch">Default browser search:</label>
          <input id="pref-defsearch" type="text" class="anki-input" bind:value={defaultSearchText} placeholder="deck:current" />
        </div>
      {:else if activeTab === "syncing"}
        <div class="sync-card">
          <h4>Self-Hosted Sync Server</h4>
          <p class="muted">
            Cardly includes a built-in sync server compatible with AnkiDroid, AnkiMobile, and Desktop clients.
            Point your mobile or desktop app's <strong>Self-hosted sync server</strong> URL to:
          </p>
          <div class="copy-row">
            <input type="text" class="anki-input mono" readonly value={syncServerUrl} />
            <button type="button" class="anki-btn" onclick={copySyncUrl}>Copy URL</button>
          </div>
          <div class="sync-meta">
            <div><strong>Username:</strong> <code>{store.username}</code></div>
            <div><strong>Password:</strong> your Cardly account password</div>
          </div>
          <p class="muted small">
            Note: Collections are stored on the server in real time. Mobile/desktop sync merges changes seamlessly
            with your web collection and media folder.
          </p>
        </div>
      {:else if activeTab === "backups"}
        <div class="bk-settings">
          <div class="form-row">
            <label for="bk-int">Minimum minutes between backups:</label>
            <input id="bk-int" type="number" class="anki-input num" min="1" bind:value={backupMinMins} />
          </div>
          <div class="bk-counts">
            <label>Daily: <input type="number" class="anki-input num" min="0" bind:value={backupDaily} /></label>
            <label>Weekly: <input type="number" class="anki-input num" min="0" bind:value={backupWeekly} /></label>
            <label>Monthly: <input type="number" class="anki-input num" min="0" bind:value={backupMonthly} /></label>
          </div>
        </div>

        <div class="bk-header">
          <strong>Saved Backups</strong>
          <button type="button" class="anki-btn small primary" onclick={handleCreateBackup}>
            Create Backup Now
          </button>
        </div>
        <div class="bk-list">
          {#if loadingBackups}
            <div class="muted pad">Loading backups...</div>
          {:else if backups.length === 0}
            <div class="muted pad">No backups yet. Click "Create Backup Now" to save a snapshot.</div>
          {:else}
            {#each backups as b (b.name)}
              <div class="bk-row">
                <div class="bk-info">
                  <div class="bk-name">{b.name}</div>
                  <div class="bk-meta">{formatDate(b.modified_secs)} · {formatBytes(b.size)}</div>
                </div>
                <div class="bk-actions">
                  <a
                    class="anki-btn small"
                    href={`/api/backups/${encodeURIComponent(b.name)}`}
                    download={b.name}
                  >
                    Download
                  </a>
                  <button
                    type="button"
                    class="anki-btn small danger"
                    onclick={() => void handleRestoreBackup(b.name)}
                  >
                    Restore
                  </button>
                </div>
              </div>
            {/each}
          {/if}
        </div>
      {/if}
    </div>

    <div class="modal-footer">
      <button class="anki-btn" onclick={close}>Cancel</button>
      <button class="anki-btn primary" disabled={loading || saving} onclick={save}>
        {saving ? "Saving..." : "Close & Save"}
      </button>
    </div>
  </div>
</div>

<style>
  .pref-window {
    width: 600px;
    max-width: 95vw;
    max-height: 86vh;
  }
  .pref-tabs {
    display: flex;
    border-bottom: 1px solid var(--border);
    background: var(--canvas-elevated);
    padding: 0 12px;
    gap: 2px;
    flex-shrink: 0;
  }
  .pref-tab {
    border: none;
    background: transparent;
    color: var(--fg-subtle);
    padding: 9px 14px;
    font-size: 13px;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
  }
  .pref-tab:hover {
    color: var(--fg);
  }
  .pref-tab.active {
    color: var(--accent);
    font-weight: 600;
    border-bottom-color: var(--accent);
  }
  .pref-body {
    min-height: 320px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .form-row {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
  }
  .form-row label {
    min-width: 170px;
    color: var(--fg-subtle);
  }
  .form-row select,
  .form-row input[type="text"] {
    flex: 1;
  }
  .num {
    width: 76px;
  }
  .unit {
    font-size: 12px;
    color: var(--fg-subtle);
  }
  .check-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    cursor: pointer;
  }
  .keys-group {
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    padding: 8px 12px;
    margin-top: 6px;
  }
  .keys-group legend {
    font-size: 12px;
    color: var(--fg-subtle);
    padding: 0 4px;
  }
  .keys-row {
    display: flex;
    gap: 14px;
    flex-wrap: wrap;
    font-size: 13px;
  }
  .key-in {
    width: 38px;
    text-align: center;
    font-family: var(--font-mono);
  }
  .sync-card {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .sync-card h4 {
    margin: 0;
    font-size: 15px;
  }
  .copy-row {
    display: flex;
    gap: 8px;
  }
  .copy-row input {
    flex: 1;
  }
  .mono {
    font-family: var(--font-mono);
  }
  .sync-meta {
    background: var(--canvas-elevated);
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    padding: 10px 12px;
    font-size: 13px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .bk-settings {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 6px;
  }
  .bk-counts {
    display: flex;
    gap: 14px;
    flex-wrap: wrap;
    font-size: 13px;
  }
  .bk-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 4px;
  }
  .bk-list {
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    background: var(--canvas-inset);
    max-height: 200px;
    overflow-y: auto;
  }
  .bk-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border-subtle);
    gap: 10px;
  }
  .bk-row:last-child {
    border-bottom: none;
  }
  .bk-name {
    font-size: 13px;
    font-weight: 500;
  }
  .bk-meta {
    font-size: 11px;
    color: var(--fg-subtle);
  }
  .bk-actions {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }
  .muted {
    color: var(--fg-subtle);
    font-size: 13px;
  }
  .small {
    font-size: 12px;
  }
  .pad {
    padding: 14px;
  }
</style>
