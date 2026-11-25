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
      <table class="deck-table" cellspacing="0" cellpadding="3">
        <thead>
          <tr>
            <th class="col-deck">Deck</th>
            <th class="col-count">New</th>
            <th class="col-count">Learn</th>
            <th class="col-count">Due</th>
            <th class="col-opts"></th>
          </tr>
        </thead>
        <tbody>
          <tr
            class="top-level-drag-row"
            class:drag-hover={dragOverId === "top"}
            ondragover={(e) => {
              e.preventDefault();
              dragOverId = "top";
            }}
            ondragleave={() => dragOverId === "top" && (dragOverId = null)}
            ondrop={(e) => handleDrop("top", e)}
          >
            <td colspan="5"></td>
          </tr>

          {#if loading && rows.length === 0}
            <tr>
              <td colspan="5" class="loading-cell">Loading decks...</td>
            </tr>
          {:else}
            {#each rows as { node, hasChildren } (Number(node.deckId))}
              {@const did = Number(node.deckId)}
              <tr
                class="deck-row"
                class:current={did === currentDeckId}
                class:drag-hover={dragOverId === did}
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
                <td class="decktd">
                  <span class="indent" style="padding-left: {(node.level - 1) * 20}px"></span>
                  {#if hasChildren}
                    <button
                      class="collapse-btn"
                      aria-label={node.collapsed ? "Expand subdecks" : "Collapse subdecks"}
                      onclick={(e) => toggleCollapse(node, e)}
                    >
                      {node.collapsed ? "+" : "−"}
                    </button>
                  {:else}
                    <span class="collapse-spacer"></span>
                  {/if}
                  <button
                    class="deck-link"
                    class:filtered={node.filtered}
                    onclick={(e) => openDeck(did, e)}
                  >
                    {shortName(node.name)}
                  </button>
                </td>
                <td class="col-count" data-col="new">
                  <span class={node.newCount > 0 ? "new-count" : "zero-count"}>{node.newCount}</span>
                </td>
                <td class="col-count" data-col="learn">
                  <span class={node.learnCount > 0 ? "learn-count" : "zero-count"}>{node.learnCount}</span>
                </td>
                <td class="col-count" data-col="due">
                  <span class={node.reviewCount > 0 ? "review-count" : "zero-count"}>{node.reviewCount}</span>
                </td>
                <td class="col-opts">
                  <button
                    class="gear-btn"
                    title="Deck options menu"
                    aria-label={`Options for ${node.name}`}
                    onclick={(e) => openGearMenu(node, e)}
                  >
                    ⚙
                  </button>
                </td>
              </tr>
            {/each}
          {/if}
        </tbody>
      </table>
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
        <a href="https://github.com/OrkenSab/cardly" target="_blank" rel="noopener noreferrer">Source Code</a>
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
  }
  .deck-scroll {
    flex: 1;
    overflow: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 24px 16px 16px;
  }
  .error-banner {
    background: rgba(239, 68, 68, 0.12);
    border: 1px solid var(--accent-danger);
    color: var(--accent-danger);
    padding: 8px 14px;
    border-radius: var(--border-radius);
    margin-bottom: 14px;
    font-size: 13.5px;
  }
  .deck-table-wrap {
    border: 1px solid var(--border-subtle);
    border-radius: var(--border-radius-medium);
    background: var(--canvas-glass);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.06);
    padding: 14px 16px;
    min-width: min(560px, 94vw);
    max-width: 92vw;
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
    border-collapse: separate;
    border-spacing: 0;
    font-size: 14.5px;
  }
  th {
    border-bottom: 1px solid var(--border-subtle);
    padding: 4px 12px 7px;
    font-weight: 600;
    font-size: 13.5px;
    color: var(--fg);
  }
  .col-deck {
    text-align: left;
  }
  .col-count {
    min-width: 4em;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .col-opts {
    width: 2.2em;
    text-align: center;
  }
  .top-level-drag-row td {
    height: 8px;
    padding: 0;
    border-bottom: 2px solid transparent;
  }
  .deck-row td {
    padding: 5px 12px;
    white-space: nowrap;
  }
  .deck-row.current td,
  .deck-row:hover td {
    background: var(--border-subtle);
  }
  .deck-row.current td:first-child,
  .deck-row:hover td:first-child {
    border-top-left-radius: var(--border-radius-medium);
    border-bottom-left-radius: var(--border-radius-medium);
  }
  .deck-row.current td:last-child,
  .deck-row:hover td:last-child {
    border-top-right-radius: var(--border-radius-medium);
    border-bottom-right-radius: var(--border-radius-medium);
  }
  .deck-row.drag-hover td,
  .top-level-drag-row.drag-hover td {
    border-bottom: 2px solid var(--border-focus);
  }
  .decktd {
    min-width: 15em;
    max-width: calc(100vw - 300px);
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .collapse-btn {
    border: none;
    background: transparent;
    color: var(--fg);
    cursor: pointer;
    width: 1.2em;
    padding: 0;
    font-weight: 600;
    font-size: 14px;
    text-align: center;
  }
  .collapse-spacer {
    display: inline-block;
    width: 1.2em;
  }
  .deck-link {
    border: none;
    background: transparent;
    color: var(--fg);
    font-family: inherit;
    font-size: inherit;
    padding: 0 4px;
    cursor: pointer;
    text-align: left;
  }
  .deck-link:hover {
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
    font-size: 15px;
    padding: 0 4px;
    opacity: 0.45;
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
      padding: 0;
    }
    .deck-table {
      font-size: 13.5px;
    }
    .decktd {
      min-width: 0;
      max-width: calc(100vw - 160px);
    }
    th {
      padding: 4px 6px 6px;
      font-size: 12px;
    }
    .deck-row td {
      padding: 9px 6px;
    }
    .col-count {
      min-width: 2.2em;
    }
    .gear-btn {
      opacity: 0.85;
      padding: 4px 6px;
      font-size: 16px;
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
