<script lang="ts">
  import { onMount } from "svelte";
  import { store } from "../lib/store.svelte";
  import Icon from "./Icon.svelte";

  const isFlat = $derived(store.view === "review");

  let inputFocused = $state(false);

  function isTextInput(el: EventTarget | null): boolean {
    if (!(el instanceof HTMLElement)) return false;
    const tag = el.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") {
      const type = (el as HTMLInputElement).type;
      return !["button", "submit", "reset", "checkbox", "radio", "file"].includes(type);
    }
    return el.isContentEditable;
  }

  onMount(() => {
    const onFocusIn = (e: FocusEvent) => {
      if (isTextInput(e.target)) inputFocused = true;
    };
    const onFocusOut = () => {
      // Defer check until focus settles on next element
      requestAnimationFrame(() => {
        inputFocused = isTextInput(document.activeElement);
      });
    };
    document.addEventListener("focusin", onFocusIn, true);
    document.addEventListener("focusout", onFocusOut, true);
    return () => {
      document.removeEventListener("focusin", onFocusIn, true);
      document.removeEventListener("focusout", onFocusOut, true);
    };
  });

  // Hide mobile bottom tab bar in Reviewer, Add/Editor & full-screen modals, or while typing
  const hideMobileBar = $derived(
    store.view === "review" || store.hasOpenModal || inputFocused
  );
</script>

<header
  class="top-bar"
  class:flat={isFlat}
  class:hidden={store.view === "review" && store.hideTopBar}
  class:mobile-hidden={hideMobileBar}
>
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
      <span class="tab-icon"><Icon name="layers" size={18} /></span>
      <span class="tab-label">Decks</span>
    </button>
    <button
      class="hitem"
      title="Shortcut: A"
      onclick={() => {
        store.addCardsOpen = true;
      }}
    >
      <span class="tab-icon"><Icon name="plus" size={18} /></span>
      <span class="tab-label">Add</span>
    </button>
    <button
      class="hitem ai-toolbar-btn"
      title="AI Card Generator (Ctrl+Shift+G)"
      onclick={() => {
        store.aiGeneratorOpen = true;
      }}
    >
      <span class="ai-icon"><Icon name="sparkles" size={15} /></span>
      <span class="tab-label">AI</span>
    </button>
    <button
      class="hitem"
      title="Shortcut: B"
      onclick={() => {
        store.openBrowser("");
      }}
    >
      <span class="tab-icon"><Icon name="search" size={18} /></span>
      <span class="tab-label">Browse</span>
    </button>
    <button
      class="hitem"
      title="Shortcut: T"
      onclick={() => {
        store.statsOpen = true;
      }}
    >
      <span class="tab-icon"><Icon name="bar-chart" size={18} /></span>
      <span class="tab-label">Stats</span>
    </button>
    <button
      class="hitem"
      title="Shortcut: Y — Sync info & refresh"
      onclick={() => {
        store.syncInfoOpen = true;
      }}
    >
      <span class="tab-icon"><Icon name="refresh-cw" size={18} /></span>
      <span class="tab-label">Sync</span>
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
        <Icon name="user" size={13} />
        <span class="user-text">{store.username}</span>
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
    gap: 5px;
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
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
  }
  .tab-icon {
    display: none;
  }
  .ai-icon {
    display: inline-flex;
    align-items: center;
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
      order: 10;
      display: flex;
      width: 100%;
      padding: 0;
      padding-bottom: var(--safe-bottom);
      padding-left: var(--safe-left);
      padding-right: var(--safe-right);
      background: var(--canvas-elevated);
      border-top: 1px solid var(--border-subtle);
      z-index: 150;
    }
    .top-bar.mobile-hidden {
      display: none;
    }
    .left-tray,
    .right-tray {
      display: none;
    }
    .toolbar {
      width: 100%;
      border-radius: 0;
      border: none;
      box-shadow: none;
      background: transparent;
      display: grid;
      grid-template-columns: repeat(6, minmax(0, 1fr));
    }
    .hitem {
      flex-direction: column;
      gap: 2px;
      min-height: 50px;
      padding: 6px 2px;
      font-size: 10.5px;
      font-weight: 600;
      border: none;
      color: var(--fg-subtle);
      min-width: 0;
    }
    .hitem.active {
      color: var(--button-primary-bg);
    }
    .hitem:first-child,
    .hitem:last-child {
      padding-left: 2px;
      padding-right: 2px;
    }
    .tab-icon,
    .ai-icon {
      display: inline-flex;
      align-items: center;
      justify-content: center;
    }
    .ai-icon :global(svg) {
      width: 18px;
      height: 18px;
    }
    .tab-label {
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
      max-width: 100%;
    }
  }
</style>
