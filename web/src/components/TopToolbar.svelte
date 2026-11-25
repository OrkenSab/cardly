<script lang="ts">
  import { store } from "../lib/store.svelte";

  const isFlat = $derived(store.view === "review");
</script>

<header class="top-bar" class:flat={isFlat} class:hidden={store.view === "review" && store.hideTopBar}>
  <div class="left-tray"></div>
  <div class="toolbar" role="navigation" aria-label="Main views">
    <button
      class="hitem"
      class:active={store.view === "decks" && !store.hasOpenModal}
      title="Shortcut: D"
      onclick={() => {
        store.view = "decks";
      }}
    >
      Decks
    </button>
    <button
      class="hitem"
      title="Shortcut: A"
      onclick={() => {
        store.addCardsOpen = true;
      }}
    >
      Add
    </button>
    <button
      class="hitem ai-toolbar-btn"
      title="AI Card Generator (Ctrl+Shift+G)"
      onclick={() => {
        store.aiGeneratorOpen = true;
      }}
    >
      ✨ AI
    </button>
    <button
      class="hitem"
      title="Shortcut: B"
      onclick={() => {
        store.openBrowser("");
      }}
    >
      Browse
    </button>
    <button
      class="hitem"
      title="Shortcut: T"
      onclick={() => {
        store.statsOpen = true;
      }}
    >
      Stats
    </button>
    <button
      class="hitem"
      title="Shortcut: Y — Sync info & refresh"
      onclick={() => {
        store.syncInfoOpen = true;
      }}
    >
      Sync
    </button>
  </div>
  <div class="right-tray">
    {#if store.isDemoUser}
      <div class="demo-pill" title="Cardly Demo Mode: sample collection resets automatically every {Math.max(1, Math.round(store.demoResetIntervalMinutes / 60))} hour{Math.round(store.demoResetIntervalMinutes / 60) > 1 ? 's' : ''}">
        <span class="demo-dot"></span>
        <span class="demo-text">Demo Guest</span>
      </div>
    {:else if store.username}
      <div class="user-pill" title="Signed in as {store.username}">
        <span class="user-text">👤 {store.username}</span>
      </div>
    {/if}
  </div>
</header>

<style>
  .right-tray {
    display: flex;
    justify-content: flex-end;
    align-items: center;
  }
  .demo-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 10px;
    margin-right: 12px;
    margin-top: 4px;
    font-size: 11px;
    font-weight: 600;
    color: var(--accent-info, #0075ca);
    background: rgba(0, 117, 202, 0.1);
    border: 1px solid rgba(0, 117, 202, 0.25);
    border-radius: 999px;
    user-select: none;
  }
  .demo-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent-info, #0075ca);
    box-shadow: 0 0 6px var(--accent-info, #0075ca);
  }
  .user-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 10px;
    margin-right: 12px;
    margin-top: 4px;
    font-size: 11px;
    font-weight: 600;
    color: var(--fg-subtle);
    background: var(--canvas-subtle, rgba(125, 125, 125, 0.08));
    border: 1px solid var(--border-subtle);
    border-radius: 999px;
    user-select: none;
  }
  @media (max-width: 640px) {
    .demo-text,
    .user-text {
      display: none;
    }
    .demo-pill,
    .user-pill {
      padding: 4px 6px;
      margin-right: 6px;
    }
  }
  .top-bar {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: start;
    flex-shrink: 0;
    user-select: none;
    padding-bottom: 4px;
  }
  .top-bar.hidden {
    display: none;
  }
  .toolbar {
    display: inline-flex;
    align-items: stretch;
    white-space: nowrap;
    overflow: hidden;
    background: var(--canvas-elevated);
    border-bottom-left-radius: var(--border-radius-medium);
    border-bottom-right-radius: var(--border-radius-medium);
    border: 1px solid var(--border-subtle);
    border-top: none;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.08);
    transition: box-shadow var(--transition) ease-in-out;
  }
  .top-bar.flat .toolbar {
    background: var(--canvas-glass);
    backdrop-filter: blur(var(--blur));
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
  }
  .toolbar:hover {
    box-shadow: 0 3px 10px rgba(0, 0, 0, 0.12);
  }
  .hitem {
    font-family: inherit;
    font-size: 14px;
    font-weight: 700;
    padding: 6px 14px;
    color: var(--fg);
    background: transparent;
    border: 1px solid transparent;
    cursor: pointer;
    line-height: 1.3;
  }
  .hitem:first-child {
    padding-left: 18px;
  }
  .hitem:last-child {
    padding-right: 18px;
  }
  .hitem:hover {
    border-color: var(--border-subtle);
    background: var(--canvas-glass);
  }
  .hitem:active {
    background: var(--canvas-inset);
  }
  :global(body.minimalist) .toolbar {
    border-radius: 0;
    border: none;
    border-bottom: 1px solid var(--border-subtle);
    box-shadow: none;
    width: 100%;
    justify-content: center;
  }
  :global(body.minimalist) .top-bar {
    grid-template-columns: 0 1fr 0;
  }
  @media (max-width: 640px) {
    .top-bar {
      grid-template-columns: 0 1fr 0;
      width: 100%;
    }
    .toolbar {
      width: 100%;
      border-radius: 0;
      border-left: none;
      border-right: none;
      display: flex;
      justify-content: space-around;
    }
    .hitem {
      padding: 8px 4px;
      font-size: 13px;
      flex: 1;
      text-align: center;
    }
    .hitem:first-child {
      padding-left: 4px;
    }
    .hitem:last-child {
      padding-right: 4px;
    }
  }
</style>
