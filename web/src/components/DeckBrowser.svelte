<script lang="ts">
  import {
    addOrUpdateDeckLegacy,
    deckTree,
    getCurrentDeck,
    newDeckLegacy,
    removeDecks,
    renameDeck,
    reparentDecks,
    setCurrentDeck,
    setDeckCollapsed,
    studiedToday,
  } from "@generated/backend";
  import { SetDeckCollapsedRequest_Scope } from "@generated/anki/decks_pb";
  import type { DeckTreeNode } from "@generated/anki/decks_pb";
  import { store } from "../lib/store.svelte";
  import Icon from "./Icon.svelte";

  interface FlatDeckRow {
    node: DeckTreeNode;
    hasChildren: boolean;
  }

  let rows = $state<FlatDeckRow[]>([]);
  let currentDeckId = $state<number>(1);
  let studiedText = $state<string>("");
  let loading = $state<boolean>(true);
  let error = $state<string>("");

  // Gear context menu state
  let menuDeck = $state<DeckTreeNode | null>(null);
  let menuX = $state<number>(0);
  let menuY = $state<number>(0);

  // Drag & drop state
  let draggingId = $state<number | null>(null);
  let dragOverId = $state<number | "top" | null>(null);

  function flatten(nodes: DeckTreeNode[], out: FlatDeckRow[] = []): FlatDeckRow[] {
    for (const node of nodes) {
      const hasChildren = node.children.length > 0;
      out.push({ node, hasChildren });
      if (hasChildren && !node.collapsed) {
        flatten(node.children, out);
      }
    }
    return out;
  }

  async function loadDecks() {
    try {
      const nowSecs = BigInt(Math.floor(Date.now() / 1000));
      const [tree, current, studied] = await Promise.all([
        deckTree({ now: nowSecs }, { alertOnError: false }),
        getCurrentDeck({}, { alertOnError: false }),
        studiedToday({}, { alertOnError: false }),
      ]);
      rows = flatten(tree.children);
      currentDeckId = Number(current.id);
      store.currentDeckId = currentDeckId;
      studiedText = studied.val;
      error = "";
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to load decks";
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void store.opRevision;
    void loadDecks();
  });

  async function openDeck(deckId: number, e?: MouseEvent) {
    if (e?.shiftKey) {
      await setCurrentDeck({ did: BigInt(deckId) });
      currentDeckId = deckId;
      store.currentDeckId = deckId;
      return;
    }
    await setCurrentDeck({ did: BigInt(deckId) });
    store.currentDeckId = deckId;
    store.view = "overview";
  }

  async function toggleCollapse(node: DeckTreeNode, e: MouseEvent) {
    e.stopPropagation();
    await setDeckCollapsed({
      deckId: node.deckId,
      collapsed: !node.collapsed,
      scope: SetDeckCollapsedRequest_Scope.REVIEWER,
    });
    await loadDecks();
  }

  async function handleCreateDeck() {
    const name = await store.prompt("Name for deck:", "", "Create Deck", "e.g. Languages::Japanese");
    if (!name || !name.trim()) return;
    try {
      const blankJson = (await newDeckLegacy({ val: false })).json;
      const deckObj = JSON.parse(new TextDecoder().decode(blankJson));
      deckObj.name = name.trim();
      await addOrUpdateDeckLegacy({
        deck: new TextEncoder().encode(JSON.stringify(deckObj)),
        preserveUsnAndMtime: false,
      });
      store.toast(`Created deck "${name.trim()}"`);
      await loadDecks();
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to create deck", "error");
    }
  }

  function openGearMenu(node: DeckTreeNode, e: MouseEvent) {
    e.stopPropagation();
    menuDeck = node;
    menuX = Math.min(e.clientX, window.innerWidth - 210);
    menuY = Math.min(e.clientY + 6, window.innerHeight - 190);
  }

  function closeGearMenu() {
    menuDeck = null;
  }

  async function handleRename(node: DeckTreeNode) {
    closeGearMenu();
    const nextName = await store.prompt("New deck name:", node.name, "Rename Deck");
    if (!nextName || !nextName.trim() || nextName.trim() === node.name) return;
    try {
      await renameDeck({ deckId: node.deckId, newName: nextName.trim() });
      store.toast("Deck renamed");
      await loadDecks();
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Rename failed", "error");
    }
  }

  async function handleDelete(node: DeckTreeNode) {
    closeGearMenu();
    const ok = await store.confirm(
      `Delete deck "${node.name}" and its ${node.totalIncludingChildren} card(s)?`,
      "Delete Deck",
      { danger: true, confirmLabel: "Delete" }
    );
    if (!ok) return;
    try {
      await removeDecks({ dids: [node.deckId] });
      store.toast(`Deleted "${node.name}"`);
      await loadDecks();
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Delete failed", "error");
    }
  }

  function handleDragStart(deckId: number, e: DragEvent) {
    draggingId = deckId;
    e.dataTransfer?.setData("text/plain", String(deckId));
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
  }

  async function handleDrop(targetDeckId: number | "top", e: DragEvent) {
    e.preventDefault();
    const src = draggingId ?? Number(e.dataTransfer?.getData("text/plain"));
    draggingId = null;
    dragOverId = null;
    if (!src) return;
    const newParent = targetDeckId === "top" ? 0 : targetDeckId;
    if (src === newParent) return;
    try {
      await reparentDecks({
        deckIds: [BigInt(src)],
        newParent: BigInt(newParent),
      });
      await loadDecks();
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Could not move deck", "error");
    }
  }

  function shortName(fullName: string): string {
    const parts = fullName.split("::");
    return parts[parts.length - 1];
  }
</script>

<svelte:window onclick={closeGearMenu} />

<main class="deck-browser">
  <div class="deck-scroll">
    {#if error}
      <div class="error-banner">{error}</div>
    {/if}

    <div class="deck-table-wrap">
      <div class="deck-table" role="table" aria-label="Decks">
        <div class="deck-thead" role="rowgroup">
          <div class="deck-header-row" role="row">
            <div class="col-deck" role="columnheader">Deck</div>
            <div class="col-counts-head" role="presentation">
              <div class="col-count" role="columnheader">New</div>
              <div class="col-count" role="columnheader">Learn</div>
              <div class="col-count" role="columnheader">Due</div>
            </div>
            <div class="col-opts" role="columnheader" aria-label="Options"></div>
          </div>
        </div>

        <div class="deck-tbody" role="rowgroup">
          <div
            class="top-level-drag-row"
            class:drag-hover={dragOverId === "top"}
            ondragover={(e) => {
              e.preventDefault();
              dragOverId = "top";
            }}
            ondragleave={() => dragOverId === "top" && (dragOverId = null)}
            ondrop={(e) => handleDrop("top", e)}
          ></div>

          {#if loading && rows.length === 0}
            <div class="loading-cell">Loading decks...</div>
          {:else}
            {#each rows as { node, hasChildren } (Number(node.deckId))}
              {@const did = Number(node.deckId)}
              <div
                class="deck-row"
                class:current={did === currentDeckId}
                class:drag-hover={dragOverId === did}
                role="row"
                draggable="true"
                ondragstart={(e) => handleDragStart(did, e)}
                ondragend={() => {
                  draggingId = null;
                  dragOverId = null;
                }}
                ondragover={(e) => {
                  e.preventDefault();
                  if (draggingId !== did) dragOverId = did;
                }}
                ondragleave={() => dragOverId === did && (dragOverId = null)}
                ondrop={(e) => handleDrop(did, e)}
              >
                <span class="indent" style="width: {(node.level - 1) * 16}px"></span>
                {#if hasChildren}
                  <button
                    type="button"
                    class="collapse-btn"
                    aria-label={node.collapsed ? "Expand group" : "Collapse group"}
                    onclick={(e) => toggleCollapse(node, e)}
                  >
                    {node.collapsed ? "+" : "−"}
                  </button>
                {:else}
                  <span class="collapse-spacer"></span>
                {/if}
                <button
                  type="button"
                  class="deck-link"
                  class:filtered={node.filtered}
                  onclick={(e) => openDeck(did, e)}
                >
                  <span class="deck-name-text">{shortName(node.name)}</span>
                  <span class="deck-counts">
                    <span class="col-count" data-col="new">
                      <span class={node.newCount > 0 ? "new-count" : "zero-count"}>{node.newCount}</span>
                    </span>
                    <span class="col-count" data-col="learn">
                      <span class={node.learnCount > 0 ? "learn-count" : "zero-count"}>{node.learnCount}</span>
                    </span>
                    <span class="col-count" data-col="due">
                      <span class={node.reviewCount > 0 ? "review-count" : "zero-count"}>{node.reviewCount}</span>
                    </span>
                  </span>
                </button>
                <div class="col-opts">
                  <button
                    type="button"
                    class="gear-btn"
                    title="Deck options menu"
                    aria-label={`Options for ${node.name}`}
                    onclick={(e) => openGearMenu(node, e)}
                  >
                    <Icon name="settings" size={16} />
                  </button>
                </div>
              </div>
            {/each}
          {/if}
        </div>
      </div>
    </div>

    {#if studiedText}
      <div class="studied-today">{@html studiedText}</div>
    {/if}
  </div>

  <footer class="bottom-bar">
    <button class="anki-btn" onclick={handleCreateDeck}>Create Deck</button>
    <button class="anki-btn" onclick={() => store.openImport("pick")}>Import File</button>
    {#if store.isDemoUser}
      <div class="demo-footer-info">
        <span>Demo resets every {Math.max(1, Math.round(store.demoResetIntervalMinutes / 60))}h • </span>
        <a href="https://github.com/cardly-anki/cardly" target="_blank" rel="noopener noreferrer">Source Code</a>
        <span> • Not affiliated with Anki / Ankitects</span>
      </div>
    {/if}
  </footer>
</main>

{#if menuDeck}
  {@const target = menuDeck}
  <div class="context-menu" style="left: {menuX}px; top: {menuY}px" role="menu">
    <button
      class="context-menu-item"
      onclick={(e) => {
        e.stopPropagation();
        const node = target;
        void handleRename(node);
      }}
    >
      <span>Rename</span>
    </button>
    <button
      class="context-menu-item"
      onclick={(e) => {
        e.stopPropagation();
        const did = Number(target.deckId);
        const isFiltered = target.filtered;
        closeGearMenu();
        if (isFiltered) {
          store.openFilteredDeck(did);
        } else {
          store.openDeckOptions(did);
        }
      }}
    >
      <span>Options</span>
    </button>
    <button
      class="context-menu-item"
      onclick={(e) => {
        e.stopPropagation();
        const did = Number(target.deckId);
        closeGearMenu();
        store.openExport({ deckId: did });
      }}
    >
      <span>Export</span>
    </button>
    <div class="context-menu-sep"></div>
    <button
      class="context-menu-item"
      onclick={(e) => {
        e.stopPropagation();
        const node = target;
        void handleDelete(node);
      }}
    >
      <span style="color: var(--accent-danger)">Delete</span>
    </button>
  </div>
{/if}

<style>
  .deck-browser {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
  }
  .deck-scroll {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 24px 16px 16px;
    min-width: 0;
  }
  .error-banner {
    background: rgba(239, 68, 68, 0.12);
    border: 1px solid var(--accent-danger);
    color: var(--accent-danger);
    padding: 8px 14px;
    border-radius: var(--border-radius);
    margin-bottom: 14px;
    font-size: 13.5px;
    max-width: 100%;
  }
  .deck-table-wrap {
    border: 1px solid var(--border-subtle);
    border-radius: var(--border-radius-medium);
    background: var(--canvas-glass);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.06);
    padding: 14px 16px;
    width: min(600px, 100%);
    max-width: 100%;
    min-width: 0;
    transition: box-shadow var(--transition) ease-in-out;
  }
  .deck-table-wrap:hover {
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  }
  :global(body.minimalist) .deck-table-wrap {
    border: none;
    box-shadow: none;
    background: transparent;
  }
  .deck-table {
    width: 100%;
    min-width: 0;
    font-size: 14.5px;
  }
  .deck-header-row {
    display: flex;
    align-items: center;
    border-bottom: 1px solid var(--border-subtle);
    padding: 4px 4px 7px 12px;
    font-weight: 600;
    font-size: 13.5px;
    color: var(--fg);
    gap: 8px;
  }
  .col-deck {
    flex: 1;
    min-width: 0;
    text-align: left;
  }
  .col-counts-head,
  .deck-counts {
    display: flex;
    align-items: center;
    flex-shrink: 0;
    gap: 4px;
  }
  .col-count {
    width: 3.6em;
    text-align: right;
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }
  .col-opts {
    width: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .top-level-drag-row {
    height: 8px;
    border-bottom: 2px solid transparent;
  }
  .deck-row {
    display: flex;
    align-items: center;
    padding: 2px 4px 2px 6px;
    border-radius: var(--border-radius-medium);
    min-width: 0;
  }
  .deck-row.current,
  .deck-row:hover {
    background: var(--border-subtle);
  }
  .deck-row.drag-hover,
  .top-level-drag-row.drag-hover {
    border-bottom: 2px solid var(--border-focus);
  }
  .indent {
    flex-shrink: 0;
  }
  .collapse-btn {
    border: none;
    background: transparent;
    color: var(--fg);
    cursor: pointer;
    width: 24px;
    height: 28px;
    padding: 0;
    font-weight: 600;
    font-size: 14px;
    text-align: center;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .collapse-spacer {
    display: inline-block;
    width: 24px;
    flex-shrink: 0;
  }
  .deck-link {
    flex: 1;
    min-width: 0;
    min-height: 32px;
    border: none;
    background: transparent;
    color: var(--fg);
    font-family: inherit;
    font-size: inherit;
    padding: 4px 6px;
    cursor: pointer;
    text-align: left;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .deck-name-text {
    flex: 1;
    min-width: 0;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    word-break: break-word;
    line-height: 1.3;
  }
  .deck-link:hover .deck-name-text {
    text-decoration: underline;
  }
  .deck-link.filtered {
    color: var(--fg-link);
  }
  .gear-btn {
    border: none;
    background: transparent;
    color: var(--fg-subtle);
    cursor: pointer;
    width: 30px;
    height: 30px;
    padding: 0;
    opacity: 0.55;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
  }
  .deck-row:hover .gear-btn,
  .deck-row.current .gear-btn,
  .gear-btn:focus {
    opacity: 1;
  }
  .loading-cell {
    text-align: center;
    padding: 20px;
    color: var(--fg-subtle);
  }
  .studied-today {
    margin-top: 24px;
    font-size: 13.5px;
    color: var(--fg-subtle);
    text-align: center;
  }
  .bottom-bar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 10px 16px;
    border-top: 1px solid var(--border-subtle);
    background: var(--canvas);
  }
  .demo-footer-info {
    margin-left: 12px;
    font-size: 11.5px;
    color: var(--fg-subtle);
  }
  .demo-footer-info a {
    color: var(--accent-info, #0075ca);
    text-decoration: underline;
  }
  @media (max-width: 640px) {
    .deck-scroll {
      padding: 10px 8px 16px;
    }
    .deck-table-wrap {
      padding: 6px 4px;
    }
    .deck-table {
      font-size: 13.5px;
    }
    .deck-header-row {
      padding: 4px 2px 6px 8px;
      font-size: 12px;
      gap: 4px;
    }
    .deck-row {
      padding: 2px 0 2px 4px;
    }
    .collapse-btn {
      width: 32px;
      min-height: 44px;
    }
    .collapse-spacer {
      width: 12px;
    }
    .deck-link {
      min-height: 44px;
      padding: 8px 4px;
      gap: 6px;
    }
    .col-count {
      width: 2.3em;
      font-size: 12.5px;
    }
    .col-opts {
      width: 44px;
    }
    .gear-btn {
      opacity: 0.85;
      min-width: 44px;
      min-height: 44px;
    }
    .bottom-bar {
      padding: 8px 12px;
      gap: 6px;
      flex-wrap: wrap;
    }
    .bottom-bar .anki-btn {
      flex: 1;
      min-width: 120px;
    }
  }
</style>
