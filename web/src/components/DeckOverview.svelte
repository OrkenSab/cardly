<script lang="ts">
  import {
    congratsInfo,
    deckTree,
    emptyFilteredDeck,
    getDeck,
    getQueuedCards,
    rebuildFilteredDeck,
    renderMarkdown,
    unburyDeck,
  } from "@generated/backend";
  import type { DeckTreeNode } from "@generated/anki/decks_pb";
  import { UnburyDeckRequest_Mode } from "@generated/anki/scheduler_pb";
  import { store } from "../lib/store.svelte";
  import UpstreamFrame from "./UpstreamFrame.svelte";

  let deckName = $state<string>("");
  let isFiltered = $state<boolean>(false);
  let descHtml = $state<string>("");
  let newCount = $state<number>(0);
  let learnCount = $state<number>(0);
  let reviewCount = $state<number>(0);
  let buriedNew = $state<number>(0);
  let buriedLearn = $state<number>(0);
  let buriedReview = $state<number>(0);
  let haveBuried = $state<boolean>(false);
  let haveSchedBuried = $state<boolean>(false);
  let haveUserBuried = $state<boolean>(false);
  let isFinished = $state<boolean>(false);
  let loading = $state<boolean>(true);

  function findNode(nodes: DeckTreeNode[], did: number): DeckTreeNode | null {
    for (const n of nodes) {
      if (Number(n.deckId) === did) return n;
      const found = findNode(n.children, did);
      if (found) return found;
    }
    return null;
  }

  async function loadOverview() {
    loading = true;
    try {
      const did = BigInt(store.currentDeckId);
      const nowSecs = BigInt(Math.floor(Date.now() / 1000));
      const [deck, queued, tree, congrats] = await Promise.all([
        getDeck({ did }, { alertOnError: false }),
        getQueuedCards({ fetchLimit: 0, intradayLearningOnly: false }, { alertOnError: false }),
        deckTree({ now: nowSecs }, { alertOnError: false }),
        congratsInfo({}, { alertOnError: false }),
      ]);

      deckName = deck.name;
      isFiltered = deck.kind.case === "filtered";
      const node = findNode(tree.children, store.currentDeckId);

      // Use queued counts for active deck (mirrors qt/aqt/overview.py)
      newCount = queued.newCount;
      learnCount = queued.learningCount;
      reviewCount = queued.reviewCount;
      const nNew = node ? node.newCount : newCount;
      const nLearn = node ? node.learnCount : learnCount;
      const nRev = node ? node.reviewCount : reviewCount;
      buriedNew = Math.max(0, nNew - newCount);
      buriedLearn = Math.max(0, nLearn - learnCount);
      buriedReview = Math.max(0, nRev - reviewCount);

      haveSchedBuried = congrats.haveSchedBuried;
      haveUserBuried = congrats.haveUserBuried;
      haveBuried = haveSchedBuried || haveUserBuried;
      isFinished = newCount === 0 && learnCount === 0 && reviewCount === 0;

      if (deck.kind.case === "normal") {
        const rawDesc = deck.kind.value.description ?? "";
        if (rawDesc && deck.kind.value.markdownDescription) {
          descHtml = (
            await renderMarkdown({ markdown: rawDesc, sanitize: true }, { alertOnError: false })
          ).val;
        } else {
          descHtml = rawDesc;
        }
      } else {
        descHtml =
          "This is a special deck for studying outside of the normal schedule. Cards will be automatically returned to their original decks after you review them. Deleting this deck from the deck list will return all remaining cards to their original deck.";
      }
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to load deck overview", "error");
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void store.currentDeckId;
    void store.opRevision;
    void loadOverview();
  });

  function startStudy() {
    if (isFinished) {
      store.toast("No cards are due yet.");
      return;
    }
    store.view = "review";
  }

  async function handleRebuild() {
    if (!isFiltered) return;
    const res = await rebuildFilteredDeck({ did: BigInt(store.currentDeckId) });
    store.toast(`Rebuilt filtered deck (${res.count} cards)`);
    await loadOverview();
  }

  async function handleEmpty() {
    if (!isFiltered) return;
    await emptyFilteredDeck({ did: BigInt(store.currentDeckId) });
    store.toast("Emptied filtered deck");
    await loadOverview();
  }

  async function handleUnbury() {
    await unburyDeck({
      deckId: BigInt(store.currentDeckId),
      mode: UnburyDeckRequest_Mode.ALL,
    });
    store.toast("Unburied cards");
    await loadOverview();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (store.hasOpenModal) return;
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    const tag = (e.target as HTMLElement | null)?.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;

    if (e.key === " " || e.key === "Enter") {
      if (!isFinished) {
        e.preventDefault();
        startStudy();
      }
    } else if (e.key === "o") {
      e.preventDefault();
      if (isFiltered) store.openFilteredDeck(store.currentDeckId);
      else store.openDeckOptions(store.currentDeckId);
    } else if (e.key === "c" && !isFiltered) {
      e.preventDefault();
      store.customStudyOpen = true;
    } else if (e.key === "u" && haveBuried) {
      e.preventDefault();
      void handleUnbury();
    } else if (e.key === "r" && isFiltered) {
      e.preventDefault();
      void handleRebuild();
    } else if (e.key === "e" && isFiltered) {
      e.preventDefault();
      void handleEmpty();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<main class="overview-screen">
  {#if loading}
    <div class="center-box"><span class="muted">Loading...</span></div>
  {:else if isFinished}
    <div class="congrats-wrap">
      <UpstreamFrame src="/congrats" title="Deck Finished" />
    </div>
  {:else}
    <div class="center-box">
      <h2 class="deck-heading">{deckName}</h2>

      <div class="overview-card">
        <table class="counts-table">
          <tbody>
            <tr>
              <td>New:</td>
              <td class="num">
                <b class="new-count">{newCount}</b>
                {#if buriedNew > 0}<span class="buried-delta">+{buriedNew}</span>{/if}
              </td>
            </tr>
            <tr>
              <td>Learning:</td>
              <td class="num">
                <b class="learn-count">{learnCount}</b>
                {#if buriedLearn > 0}<span class="buried-delta">+{buriedLearn}</span>{/if}
              </td>
            </tr>
            <tr>
              <td>To Review:</td>
              <td class="num">
                <b class="review-count">{reviewCount}</b>
                {#if buriedReview > 0}<span class="buried-delta">+{buriedReview}</span>{/if}
              </td>
            </tr>
          </tbody>
        </table>

        <div class="study-action">
          <button id="study" class="anki-btn primary study-btn" onclick={startStudy}>
            Study Now
          </button>
        </div>
      </div>

      {#if descHtml}
        <div class="desc-box" class:dyn={isFiltered}>
          {@html descHtml}
        </div>
      {/if}
    </div>
  {/if}

  <footer class="bottom-bar">
    {#if isFiltered}
      <button class="anki-btn" title="Shortcut: O" onclick={() => store.openFilteredDeck(store.currentDeckId)}>
        Options
      </button>
      <button class="anki-btn" title="Shortcut: R" onclick={handleRebuild}>Rebuild</button>
      <button class="anki-btn" title="Shortcut: E" onclick={handleEmpty}>Empty</button>
    {:else}
      <button class="anki-btn" title="Shortcut: O" onclick={() => store.openDeckOptions(store.currentDeckId)}>
        Options
      </button>
      <button class="anki-btn" title="Shortcut: C" onclick={() => (store.customStudyOpen = true)}>
        Custom Study
      </button>
      <button class="anki-btn" onclick={() => store.openDeckDescription(store.currentDeckId)}>
        Description
      </button>
    {/if}
    {#if haveBuried}
      <button class="anki-btn" title="Shortcut: U" onclick={handleUnbury}>Unbury</button>
    {/if}
  </footer>
</main>

<style>
  .overview-screen {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .congrats-wrap {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .center-box {
    flex: 1;
    overflow: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 24px 16px;
    gap: 18px;
  }
  .deck-heading {
    margin: 0;
    font-size: 20px;
    font-weight: 700;
    text-align: center;
    max-width: 100%;
    word-break: break-word;
  }
  .overview-card {
    display: flex;
    align-items: center;
    gap: 32px;
    padding: 22px 28px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--border-radius-medium);
    background: var(--canvas-elevated);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.06);
  }
  .counts-table {
    border-collapse: collapse;
    font-size: 14.5px;
  }
  .counts-table td {
    padding: 4px 10px 4px 0;
  }
  .counts-table td.num {
    text-align: right;
    padding-right: 0;
    font-variant-numeric: tabular-nums;
  }
  .buried-delta {
    margin-left: 4px;
    font-size: 12px;
    color: var(--fg-subtle);
  }
  .study-btn {
    padding: 10px 24px;
    font-size: 15px;
  }
  .desc-box {
    max-width: 600px;
    width: 100%;
    color: var(--fg-subtle);
    font-size: 13.5px;
    line-height: 1.5;
    white-space: pre-wrap;
    text-align: left;
  }
  .desc-box.dyn {
    text-align: center;
  }
  .muted {
    color: var(--fg-subtle);
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
  @media (max-width: 640px) {
    .center-box {
      padding: 16px 12px;
    }
    .overview-card {
      flex-direction: column;
      align-items: stretch;
      gap: 16px;
      padding: 16px;
      width: 100%;
      max-width: 340px;
    }
    .study-btn {
      width: 100%;
      min-height: 44px;
    }
    .bottom-bar {
      padding: 8px 12px;
      gap: 6px;
      flex-wrap: wrap;
    }
    .bottom-bar .anki-btn {
      flex: 1;
      min-width: 80px;
    }
  }
</style>
