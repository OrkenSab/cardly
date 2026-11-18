<script lang="ts">
  import { createBackupNow, logout, performRedo, performUndo } from "../lib/api";
  import { store } from "../lib/store.svelte";

  type MenuId = "file" | "edit" | "tools" | "help" | null;
  let openMenu = $state<MenuId>(null);

  function toggleMenu(id: MenuId, e: MouseEvent) {
    e.stopPropagation();
    openMenu = openMenu === id ? null : id;
  }

  function closeMenus() {
    openMenu = null;
  }

  function run(fn: () => void) {
    openMenu = null;
    fn();
  }

  async function handleCreateBackup() {
    openMenu = null;
    try {
      await createBackupNow();
      store.toast("Backup created");
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Backup failed", "error");
    }
  }
</script>

<svelte:window onclick={closeMenus} />

<nav class="menubar" aria-label="Application menu">
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

  <div class="menubar-right">
    <button
      class="theme-toggle"
      title={store.isDark ? "Switch to light mode" : "Switch to dark mode"}
      onclick={() => store.setTheme(store.isDark ? "light" : "dark")}
    >
      {store.isDark ? "☀" : "☾"}
    </button>
    <span class="user-pill" title="Signed in account">{store.username}</span>
  </div>
</nav>

<style>
  .menubar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 28px;
    padding: 0 8px;
    background: var(--canvas-elevated);
    border-bottom: 1px solid var(--border-subtle);
    font-size: 12.5px;
    user-select: none;
    flex-shrink: 0;
    z-index: 200;
  }
  .menubar-left,
  .menubar-right {
    display: flex;
    align-items: center;
    gap: 2px;
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
    font-size: 13px;
    padding: 2px 6px;
    border-radius: 4px;
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
</style>
