<script lang="ts">
  import { createBackupNow, logout, performRedo, performUndo } from "../lib/api";
  import { store } from "../lib/store.svelte";
  import Icon from "./Icon.svelte";

  interface Props {
    demoBannerVisible?: boolean;
  }

  let { demoBannerVisible = false }: Props = $props();

  type MenuId = "file" | "edit" | "tools" | "help" | null;
  let openMenu = $state<MenuId>(null);
  let mobileSheetOpen = $state(false);

  const mobileTitle = $derived(
    store.view === "decks" ? "Cardly" : store.view === "overview" ? "Deck Overview" : "Study"
  );

  function toggleMenu(id: MenuId, e: MouseEvent) {
    e.stopPropagation();
    openMenu = openMenu === id ? null : id;
  }

  function closeMenus() {
    openMenu = null;
  }

  function run(fn: () => void) {
    openMenu = null;
    mobileSheetOpen = false;
    fn();
  }

  async function handleCreateBackup() {
    openMenu = null;
    mobileSheetOpen = false;
    try {
      await createBackupNow();
      store.toast("Backup created");
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Backup failed", "error");
    }
  }
</script>

<svelte:window onclick={closeMenus} />

<nav class="menubar" class:has-banner={demoBannerVisible} aria-label="Application menu">
  <div class="menubar-left">
    <div class="menu-root">
      <button
        class="menu-trigger"
        class:active={openMenu === "file"}
        onclick={(e) => toggleMenu("file", e)}
        onmouseenter={() => openMenu && (openMenu = "file")}
      >
        File
      </button>
      {#if openMenu === "file"}
        <div class="menu-dropdown" role="menu">
          <button class="context-menu-item" role="menuitem" onclick={() => run(() => store.openImport("pick"))}>
            <span>Import...</span><span class="shortcut">Ctrl+Shift+I</span>
          </button>
          <button class="context-menu-item" role="menuitem" onclick={() => run(() => store.openExport())}>
            <span>Export...</span><span class="shortcut">Ctrl+E</span>
          </button>
          <div class="context-menu-sep"></div>
          <button class="context-menu-item" role="menuitem" onclick={handleCreateBackup}>
            <span>Create Backup</span>
          </button>
          <button class="context-menu-item" role="menuitem" onclick={() => run(() => (store.preferencesOpen = true))}>
            <span>Preferences...</span><span class="shortcut">Ctrl+P</span>
          </button>
          <div class="context-menu-sep"></div>
          <button class="context-menu-item" role="menuitem" onclick={() => run(() => void logout())}>
            <span>Sign Out ({store.username})</span>
          </button>
        </div>
      {/if}
    </div>

    <div class="menu-root">
      <button
        class="menu-trigger"
        class:active={openMenu === "edit"}
        onclick={(e) => toggleMenu("edit", e)}
        onmouseenter={() => openMenu && (openMenu = "edit")}
      >
        Edit
      </button>
      {#if openMenu === "edit"}
        <div class="menu-dropdown" role="menu">
          <button
            class="context-menu-item"
            role="menuitem"
            disabled={!store.undoLabel}
            onclick={() => run(() => void performUndo())}
          >
            <span>{store.undoLabel ? `Undo ${store.undoLabel}` : "Undo"}</span>
            <span class="shortcut">Ctrl+Z</span>
          </button>
          <button
            class="context-menu-item"
            role="menuitem"
            disabled={!store.redoLabel}
            onclick={() => run(() => void performRedo())}
          >
            <span>{store.redoLabel ? `Redo ${store.redoLabel}` : "Redo"}</span>
            <span class="shortcut">Ctrl+Shift+Z</span>
          </button>
        </div>
      {/if}
    </div>

    <div class="menu-root">
      <button
        class="menu-trigger"
        class:active={openMenu === "tools"}
        onclick={(e) => toggleMenu("tools", e)}
        onmouseenter={() => openMenu && (openMenu = "tools")}
      >
        Tools
      </button>
      {#if openMenu === "tools"}
        <div class="menu-dropdown" role="menu">
          <button class="context-menu-item" role="menuitem" onclick={() => run(() => (store.aiGeneratorOpen = true))}>
            <span>AI Card Generator...</span><span class="shortcut">Ctrl+Shift+G</span>
          </button>
          <div class="context-menu-sep"></div>
          <button class="context-menu-item" role="menuitem" onclick={() => run(() => (store.studyDeckOpen = true))}>
            <span>Study Deck...</span><span class="shortcut">S</span>
          </button>
          <button class="context-menu-item" role="menuitem" onclick={() => run(() => store.openFilteredDeck(0))}>
            <span>Create Filtered Deck...</span><span class="shortcut">F</span>
          </button>
          <div class="context-menu-sep"></div>
          <button class="context-menu-item" role="menuitem" onclick={() => run(() => (store.checkDbOpen = true))}>
            <span>Check Database</span>
          </button>
          <button class="context-menu-item" role="menuitem" onclick={() => run(() => (store.checkMediaOpen = true))}>
            <span>Check Media...</span>
          </button>
          <button class="context-menu-item" role="menuitem" onclick={() => run(() => (store.emptyCardsOpen = true))}>
            <span>Empty Cards...</span>
          </button>
          <div class="context-menu-sep"></div>
          <button
            class="context-menu-item"
            role="menuitem"
            onclick={() => run(() => (store.manageNotetypesOpen = true))}
          >
            <span>Manage Note Types...</span><span class="shortcut">Ctrl+Shift+N</span>
          </button>
          <button class="context-menu-item" role="menuitem" onclick={() => run(() => (store.preferencesOpen = true))}>
            <span>Preferences...</span><span class="shortcut">Ctrl+P</span>
          </button>
        </div>
      {/if}
    </div>

    <div class="menu-root">
      <button
        class="menu-trigger"
        class:active={openMenu === "help"}
        onclick={(e) => toggleMenu("help", e)}
        onmouseenter={() => openMenu && (openMenu = "help")}
      >
        Help
      </button>
      {#if openMenu === "help"}
        <div class="menu-dropdown" role="menu">
          {#if store.canInstall}
            <button class="context-menu-item" role="menuitem" onclick={() => run(() => store.installCardly())}>
              <span>Install Cardly App...</span>
            </button>
            <div class="context-menu-sep"></div>
          {/if}
          <button class="context-menu-item" role="menuitem" onclick={() => run(() => (store.syncInfoOpen = true))}>
            <span>Mobile &amp; Desktop Sync Setup...</span>
          </button>
          <button class="context-menu-item" role="menuitem" onclick={() => run(() => (store.aboutOpen = true))}>
            <span>About Cardly</span>
          </button>
        </div>
      {/if}
    </div>
  </div>

  <!-- Compact mobile header title (visible <= 640px) -->
  <div class="mobile-header-left">
    <span class="mobile-title">{mobileTitle}</span>
  </div>

  <div class="menubar-right">
    {#if !store.isOnline}
      <span class="offline-pill" title="Network disconnected">Offline</span>
    {/if}
    {#if store.canInstall}
      <button
        class="install-pill-btn"
        title="Install Cardly as a desktop/mobile app"
        onclick={() => store.installCardly()}
      >
        Install App
      </button>
    {/if}
    <button
      class="theme-toggle"
      title={store.isDark ? "Switch to light mode" : "Switch to dark mode"}
      aria-label={store.isDark ? "Switch to light mode" : "Switch to dark mode"}
      onclick={() => store.setTheme(store.isDark ? "light" : "dark")}
    >
      <Icon name={store.isDark ? "sun" : "moon"} size={15} />
    </button>
    <span class="user-pill" title="Signed in account">{store.username}</span>
    <button
      type="button"
      class="mobile-menu-btn"
      title="Menu"
      aria-label="Open menu"
      aria-expanded={mobileSheetOpen}
      onclick={(e) => {
        e.stopPropagation();
        mobileSheetOpen = !mobileSheetOpen;
      }}
    >
      <Icon name="menu" size={18} />
    </button>
  </div>
</nav>

{#if mobileSheetOpen}
  <div
    class="mobile-sheet-backdrop"
    role="presentation"
    onclick={() => (mobileSheetOpen = false)}
  >
    <div
      class="mobile-sheet"
      role="dialog"
      aria-modal="true"
      aria-label="Application menu"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.key === "Escape" && (mobileSheetOpen = false)}
      tabindex="-1"
    >
      <div class="mobile-sheet-header">
        <div class="mobile-sheet-account">
          <Icon name="user" size={16} />
          <span>{store.username || "Account"}</span>
        </div>
        <button
          type="button"
          class="mobile-sheet-close"
          aria-label="Close menu"
          onclick={() => (mobileSheetOpen = false)}
        >
          <Icon name="x" size={18} />
        </button>
      </div>

      <div class="mobile-sheet-body">
        <div class="sheet-section-label">Actions</div>
        <button class="sheet-item" onclick={() => run(() => store.openImport("pick"))}>Import...</button>
        <button class="sheet-item" onclick={() => run(() => store.openExport())}>Export...</button>
        <button class="sheet-item" onclick={handleCreateBackup}>Create Backup</button>
        <button class="sheet-item" onclick={() => run(() => (store.preferencesOpen = true))}>Preferences...</button>

        <div class="sheet-sep"></div>
        <div class="sheet-section-label">Edit</div>
        <button class="sheet-item" disabled={!store.undoLabel} onclick={() => run(() => void performUndo())}>
          {store.undoLabel ? `Undo ${store.undoLabel}` : "Undo"}
        </button>
        <button class="sheet-item" disabled={!store.redoLabel} onclick={() => run(() => void performRedo())}>
          {store.redoLabel ? `Redo ${store.redoLabel}` : "Redo"}
        </button>

        <div class="sheet-sep"></div>
        <div class="sheet-section-label">Tools</div>
        <button class="sheet-item" onclick={() => run(() => (store.aiGeneratorOpen = true))}>AI Card Generator...</button>
        <button class="sheet-item" onclick={() => run(() => (store.studyDeckOpen = true))}>Study Deck...</button>
        <button class="sheet-item" onclick={() => run(() => store.openFilteredDeck(0))}>Create Filtered Deck...</button>
        <button class="sheet-item" onclick={() => run(() => (store.checkDbOpen = true))}>Check Database</button>
        <button class="sheet-item" onclick={() => run(() => (store.checkMediaOpen = true))}>Check Media...</button>
        <button class="sheet-item" onclick={() => run(() => (store.emptyCardsOpen = true))}>Empty Cards...</button>
        <button class="sheet-item" onclick={() => run(() => (store.manageNotetypesOpen = true))}>Manage Note Types...</button>

        <div class="sheet-sep"></div>
        <div class="sheet-section-label">Help &amp; Appearance</div>
        <button
          class="sheet-item"
          onclick={() => store.setTheme(store.isDark ? "light" : "dark")}
        >
          <span>{store.isDark ? "Switch to Light Mode" : "Switch to Dark Mode"}</span>
          <Icon name={store.isDark ? "sun" : "moon"} size={16} />
        </button>
        {#if store.canInstall}
          <button class="sheet-item" onclick={() => run(() => store.installCardly())}>Install Cardly App...</button>
        {/if}
        <button class="sheet-item" onclick={() => run(() => (store.syncInfoOpen = true))}>Mobile &amp; Desktop Sync Setup...</button>
        <button class="sheet-item" onclick={() => run(() => (store.aboutOpen = true))}>About Cardly</button>
        <a
          class="sheet-item"
          href="https://github.com/OrkenSab/cardly"
          target="_blank"
          rel="noopener noreferrer"
          onclick={() => (mobileSheetOpen = false)}
        >
          <span>Source Code (AGPL-3.0)</span>
          <Icon name="external-link" size={15} />
        </a>

        <div class="sheet-sep"></div>
        <button class="sheet-item danger" onclick={() => run(() => void logout())}>
          Sign Out ({store.username})
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .menubar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: calc(28px + var(--safe-top));
    padding: var(--safe-top) calc(8px + var(--safe-right)) 0 calc(8px + var(--safe-left));
    background: var(--canvas-elevated);
    border-bottom: 1px solid var(--border-subtle);
    font-size: 12.5px;
    user-select: none;
    flex-shrink: 0;
    z-index: 200;
  }
  .menubar.has-banner {
    min-height: 28px;
    padding-top: 0;
  }
  .menubar-left,
  .menubar-right {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .mobile-header-left,
  .mobile-menu-btn {
    display: none;
  }
  .menu-root {
    position: relative;
  }
  .menu-trigger {
    border: none;
    background: transparent;
    color: var(--fg);
    padding: 3px 8px;
    border-radius: 4px;
    font-family: inherit;
    font-size: 12.5px;
    cursor: pointer;
  }
  .menu-trigger:hover,
  .menu-trigger.active {
    background: var(--border-subtle);
  }
  .menu-dropdown {
    position: absolute;
    top: calc(100% + 2px);
    left: 0;
    min-width: 220px;
    background: var(--canvas-overlay);
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 4px;
    box-shadow: 0 10px 26px rgba(0, 0, 0, 0.22);
    z-index: 300;
  }
  .theme-toggle {
    border: none;
    background: transparent;
    color: var(--fg-subtle);
    cursor: pointer;
    padding: 4px 6px;
    border-radius: 4px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .theme-toggle:hover {
    background: var(--border-subtle);
    color: var(--fg);
  }
  .user-pill {
    color: var(--fg-subtle);
    font-size: 12px;
    padding: 0 6px;
  }
  .offline-pill {
    background: var(--accent-danger);
    color: #ffffff;
    font-size: 11px;
    font-weight: 600;
    padding: 1px 6px;
    border-radius: 4px;
    margin-right: 4px;
  }
  .install-pill-btn {
    background: var(--button-primary-bg);
    color: #ffffff;
    border: none;
    font-size: 11.5px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 4px;
    cursor: pointer;
    margin-right: 4px;
  }
  .install-pill-btn:hover {
    filter: brightness(1.1);
  }

  /* Mobile Menu Sheet */
  .mobile-sheet-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.48);
    z-index: 2500;
    display: flex;
    align-items: flex-end;
    justify-content: center;
  }
  .mobile-sheet {
    width: 100%;
    max-width: 540px;
    max-height: 85dvh;
    background: var(--canvas-overlay);
    border-top-left-radius: 16px;
    border-top-right-radius: 16px;
    border-top: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 -8px 30px rgba(0, 0, 0, 0.3);
  }
  .mobile-sheet-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border-subtle);
    background: var(--canvas-elevated);
    flex-shrink: 0;
  }
  .mobile-sheet-account {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    font-weight: 600;
    color: var(--fg);
  }
  .mobile-sheet-close {
    border: none;
    background: transparent;
    color: var(--fg-subtle);
    min-width: 44px;
    min-height: 44px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 8px;
    cursor: pointer;
  }
  .mobile-sheet-body {
    overflow-y: auto;
    padding: 8px 10px calc(12px + var(--safe-bottom));
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .sheet-section-label {
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--fg-subtle);
    padding: 8px 10px 4px;
  }
  .sheet-item {
    width: 100%;
    min-height: 44px;
    padding: 10px 12px;
    border: none;
    background: transparent;
    color: var(--fg);
    font-family: inherit;
    font-size: 15px;
    text-align: left;
    border-radius: 8px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: space-between;
    text-decoration: none;
  }
  .sheet-item:hover:not(:disabled),
  .sheet-item:active:not(:disabled) {
    background: var(--border-subtle);
  }
  .sheet-item:disabled {
    color: var(--fg-disabled);
    cursor: default;
  }
  .sheet-item.danger {
    color: var(--accent-danger);
  }
  .sheet-sep {
    height: 1px;
    background: var(--border-subtle);
    margin: 6px 8px;
  }

  @media (max-width: 640px) {
    .menubar {
      min-height: calc(44px + var(--safe-top));
      padding: var(--safe-top) calc(8px + var(--safe-right)) 0 calc(14px + var(--safe-left));
    }
    .menubar.has-banner {
      min-height: 44px;
      padding-top: 0;
    }
    .menubar-left,
    .user-pill {
      display: none;
    }
    .mobile-header-left {
      display: flex;
      align-items: center;
      min-width: 0;
    }
    .mobile-title {
      font-size: 16px;
      font-weight: 700;
      color: var(--fg);
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    .theme-toggle,
    .mobile-menu-btn {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-width: 44px;
      min-height: 44px;
      border: none;
      background: transparent;
      color: var(--fg);
      border-radius: 8px;
      cursor: pointer;
    }
  }
</style>
