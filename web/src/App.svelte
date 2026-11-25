<script lang="ts">
  import { onMount } from "svelte";
  import {
    checkAuth,
    installFetchInterceptor,
    performRedo,
    performUndo,
  } from "./lib/api";
  import { installUpstreamBridge } from "./lib/bridge";
  import { store } from "./lib/store.svelte";

  import AudioRecorderModal from "./components/AudioRecorderModal.svelte";
  import AiCardGeneratorModal from "./components/AiCardGeneratorModal.svelte";
  import Browser from "./components/Browser.svelte";
  import CardPreviewerModal from "./components/CardPreviewerModal.svelte";
  import CardTemplatesModal from "./components/CardTemplatesModal.svelte";
  import CustomStudyModal from "./components/CustomStudyModal.svelte";
  import DeckBrowser from "./components/DeckBrowser.svelte";
  import DeckDescriptionModal from "./components/DeckDescriptionModal.svelte";
  import DeckOverview from "./components/DeckOverview.svelte";
  import DialogOverlay from "./components/DialogOverlay.svelte";
  import ExportModal from "./components/ExportModal.svelte";
  import FieldsModal from "./components/FieldsModal.svelte";
  import FilteredDeckModal from "./components/FilteredDeckModal.svelte";
  import Icon from "./components/Icon.svelte";
  import ImportModal from "./components/ImportModal.svelte";
  import LoginView from "./components/LoginView.svelte";
  import MaintenanceModals from "./components/MaintenanceModals.svelte";
  import ManageNotetypesModal from "./components/ManageNotetypesModal.svelte";
  import MenuBar from "./components/MenuBar.svelte";
  import PreferencesModal from "./components/PreferencesModal.svelte";
  import Reviewer from "./components/Reviewer.svelte";
  import StudyDeckModal from "./components/StudyDeckModal.svelte";
  import TopToolbar from "./components/TopToolbar.svelte";
  import UpstreamEditor from "./components/UpstreamEditor.svelte";
  import UpstreamFrame from "./components/UpstreamFrame.svelte";

  let demoBannerDismissed = $state(
    typeof sessionStorage !== "undefined" && sessionStorage.getItem("cardly_demo_banner_dismissed") === "1"
  );

  function dismissDemoBanner() {
    demoBannerDismissed = true;
    try {
      sessionStorage.setItem("cardly_demo_banner_dismissed", "1");
    } catch {
      // ignore storage errors
    }
  }

  onMount(() => {
    installFetchInterceptor();
    installUpstreamBridge();
    store.initTheme();
    void checkAuth();
  });

  function isEditableTarget(target: EventTarget | null): boolean {
    if (!(target instanceof HTMLElement)) return false;
    const tag = target.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
    if (target.isContentEditable) return true;
    return false;
  }

  function handleGlobalKeydown(ev: KeyboardEvent) {
    if (store.authStatus !== "authenticated") return;

    // Escape closes topmost modal (except when DialogOverlay handles it)
    if (ev.key === "Escape") {
      if (store.dialog) return;
      if (store.aiGeneratorOpen) {
        store.aiGeneratorOpen = false;
        return;
      }
      if (store.audioRecorder.open) {
        store.audioRecorder.resolve?.(null);
        return;
      }
      if (store.previewer.open) {
        store.previewer = { open: false, cardIds: [], index: 0 };
        return;
      }
      if (store.cardInfo.open) {
        store.cardInfo = { open: false, cardId: 0 };
        return;
      }
      if (store.fieldsModal.open) {
        store.fieldsModal = { open: false, notetypeId: 0 };
        return;
      }
      if (store.cardTemplates.open) {
        store.cardTemplates = { open: false, notetypeId: 0 };
        return;
      }
      if (store.changeNotetype.open) {
        store.changeNotetype = { open: false, noteIds: [], notetypeId: 0 };
        return;
      }
      if (store.manageNotetypesOpen) {
        store.manageNotetypesOpen = false;
        return;
      }
      if (store.deckOptions.open) {
        store.deckOptions = { open: false, deckId: store.deckOptions.deckId };
        return;
      }
      if (store.statsOpen) {
        store.statsOpen = false;
        return;
      }
      if (store.preferencesOpen) {
        store.preferencesOpen = false;
        return;
      }
      if (store.importModal.open) {
        store.importModal = { open: false, mode: "pick" };
        return;
      }
      if (store.exportModal.open) {
        store.exportModal = { open: false };
        return;
      }
      if (store.customStudyOpen) {
        store.customStudyOpen = false;
        return;
      }
      if (store.filteredDeck.open) {
        store.filteredDeck = { open: false };
        return;
      }
      if (store.deckDescription.open) {
        store.deckDescription = { open: false, deckId: 0 };
        return;
      }
      if (store.studyDeckOpen) {
        store.studyDeckOpen = false;
        return;
      }
      if (store.imageOcclusion.open) {
        store.imageOcclusion = { open: false, pathOrNoteId: "" };
        return;
      }
      if (store.checkDbOpen || store.checkMediaOpen || store.emptyCardsOpen || store.syncInfoOpen || store.aboutOpen) {
        store.checkDbOpen = false;
        store.checkMediaOpen = false;
        store.emptyCardsOpen = false;
        store.syncInfoOpen = false;
        store.aboutOpen = false;
        return;
      }
      if (store.editCurrent.open) {
        store.editCurrent = { open: false, noteId: 0 };
        store.bumpOpRevision();
        return;
      }
      if (store.addCardsOpen) {
        store.addCardsOpen = false;
        store.bumpOpRevision();
        return;
      }
      if (store.browser.open) {
        store.browser = { open: false, initialSearch: "" };
        return;
      }
      return;
    }

    const mod = ev.ctrlKey || ev.metaKey;
    const key = ev.key.toLowerCase();

    // Global modifier shortcuts (matching Desktop Anki)
    if (mod && !ev.altKey) {
      if (key === "z" && !ev.shiftKey && !isEditableTarget(ev.target) && !store.hasOpenModal) {
        ev.preventDefault();
        void performUndo();
        return;
      }
      if ((key === "z" && ev.shiftKey) || (key === "y" && !ev.shiftKey)) {
        if (!isEditableTarget(ev.target) && !store.hasOpenModal) {
          ev.preventDefault();
          void performRedo();
          return;
        }
      }
      if (key === "p" && !ev.shiftKey) {
        ev.preventDefault();
        store.preferencesOpen = true;
        return;
      }
      if (key === "i" && ev.shiftKey) {
        ev.preventDefault();
        store.openImport("pick");
        return;
      }
      if (key === "e" && !ev.shiftKey && !isEditableTarget(ev.target)) {
        ev.preventDefault();
        store.openExport();
        return;
      }
      if (key === "n" && ev.shiftKey) {
        ev.preventDefault();
        store.manageNotetypesOpen = true;
        return;
      }
      if (key === "g" && ev.shiftKey) {
        ev.preventDefault();
        store.aiGeneratorOpen = true;
        return;
      }
    }

    // Single-key main window shortcuts (only when no modal is open & not typing in an input)
    if (store.hasOpenModal || mod || ev.altKey || isEditableTarget(ev.target)) {
      return;
    }

    if (key === "d" && !ev.shiftKey) {
      ev.preventDefault();
      store.view = "decks";
    } else if (key === "a" && !ev.shiftKey) {
      ev.preventDefault();
      store.addCardsOpen = true;
    } else if (key === "b" && !ev.shiftKey) {
      ev.preventDefault();
      store.openBrowser("");
    } else if (key === "t" && !ev.shiftKey) {
      ev.preventDefault();
      store.statsOpen = true;
    } else if (key === "y" && !ev.shiftKey) {
      ev.preventDefault();
      store.syncInfoOpen = true;
    } else if (key === "s" && !ev.shiftKey && store.view !== "review") {
      ev.preventDefault();
      if (store.view === "decks") {
        store.studyDeckOpen = true;
      } else if (store.view === "overview") {
        store.view = "review";
      }
    } else if (key === "f" && !ev.shiftKey && store.view !== "review") {
      ev.preventDefault();
      store.openFilteredDeck(0);
    }
  }
</script>

<svelte:window onkeydown={handleGlobalKeydown} />

{#if store.authStatus === "loading"}
  <div class="boot-screen">
    <div class="boot-spinner"></div>
    <span>Loading Cardly...</span>
  </div>
{:else if store.authStatus === "unauthenticated"}
  <LoginView />
{:else}
  <div class="app-shell">
    {#if store.isDemoUser && !demoBannerDismissed}
      <div
        class="demo-banner-top"
        class:needs-safe-top={store.view === "review" && store.hideTopBar}
        role="region"
        aria-label="Demo notice"
      >
        <span class="demo-banner-text">
          <Icon name="zap" size={14} />
          <strong>Demo Mode</strong>: data resets every {Math.max(1, Math.round(store.demoResetIntervalMinutes / 60))}h.
        </span>
        <div class="demo-banner-links">
          <a href="https://github.com/cardly-anki/cardly" target="_blank" rel="noopener noreferrer">Source Code</a>
          <span class="banner-sep">•</span>
          <span class="banner-disclaimer">Not affiliated with Anki / Ankitects</span>
          <button
            type="button"
            class="demo-banner-close"
            title="Dismiss demo notice"
            aria-label="Dismiss demo notice"
            onclick={dismissDemoBanner}
          >
            <Icon name="x" size={14} />
          </button>
        </div>
      </div>
    {/if}
    {#if !(store.view === "review" && store.hideTopBar)}
      <MenuBar demoBannerVisible={store.isDemoUser && !demoBannerDismissed} />
      <TopToolbar />
    {/if}

    <main class="main-area">
      {#if store.view === "decks"}
        <DeckBrowser />
      {:else if store.view === "overview"}
        <DeckOverview />
      {:else if store.view === "review"}
        <Reviewer />
      {/if}
    </main>
  </div>

  <!-- Add Cards Modal (hosts real upstream ts/routes/editor?mode=add) -->
  {#if store.addCardsOpen}
    <div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Add">
      <div class="modal-window add-window">
        <div class="modal-header">
          <h3 class="modal-title">Add</h3>
          <button
            class="modal-close"
            onclick={() => {
              store.addCardsOpen = false;
              store.bumpOpRevision();
            }}
          >×</button>
        </div>
        <div class="modal-body flush">
          <UpstreamEditor
            mode="add"
            deckId={store.currentDeckId}
            onSaved={() => store.bumpOpRevision()}
          />
        </div>
      </div>
    </div>
  {/if}

  <!-- Edit Current Note Modal (hosts real upstream ts/routes/editor?mode=current) -->
  {#if store.editCurrent.open && store.editCurrent.noteId}
    <div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Edit Current">
      <div class="modal-window add-window">
        <div class="modal-header">
          <h3 class="modal-title">Edit Current</h3>
          <button
            class="modal-close"
            onclick={() => {
              store.editCurrent = { open: false, noteId: 0 };
              store.bumpOpRevision();
            }}
          >×</button>
        </div>
        <div class="modal-body flush">
          <UpstreamEditor
            mode="current"
            noteId={store.editCurrent.noteId}
            reviewerCardId={store.editCurrent.cardId ?? null}
            onSaved={() => store.bumpOpRevision()}
          />
        </div>
      </div>
    </div>
  {/if}

  <!-- Card & Note Browser -->
  {#if store.browser.open}
    <Browser />
  {/if}

  <!-- Statistics (real upstream ts/routes/graphs) -->
  {#if store.statsOpen}
    <div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Statistics">
      <div class="modal-window stats-window">
        <div class="modal-header">
          <h3 class="modal-title">Statistics</h3>
          <button class="modal-close" onclick={() => (store.statsOpen = false)}>×</button>
        </div>
        <div class="modal-body flush">
          <UpstreamFrame src="/graphs" title="Statistics" />
        </div>
      </div>
    </div>
  {/if}

  <!-- Deck Options including FSRS (real upstream ts/routes/deck-options/[deckId]) -->
  {#if store.deckOptions.open}
    <div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Deck Options">
      <div class="modal-window deck-opts-window">
        <div class="modal-header">
          <h3 class="modal-title">Deck Options</h3>
          <button
            class="modal-close"
            onclick={() => {
              store.deckOptions = { open: false, deckId: store.deckOptions.deckId };
              store.bumpOpRevision();
            }}
          >×</button>
        </div>
        <div class="modal-body flush">
          <UpstreamFrame
            src={`/deck-options/${store.deckOptions.deckId}`}
            title="Deck Options"
          />
        </div>
      </div>
    </div>
  {/if}

  <!-- Card Info (real upstream ts/routes/card-info/[cardId]) -->
  {#if store.cardInfo.open && store.cardInfo.cardId}
    <div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Card Info">
      <div class="modal-window card-info-window">
        <div class="modal-header">
          <h3 class="modal-title">{store.cardInfo.title ?? "Card Info"}</h3>
          <button
            class="modal-close"
            onclick={() => (store.cardInfo = { open: false, cardId: 0 })}
          >×</button>
        </div>
        <div class="modal-body flush">
          <UpstreamFrame
            src={`/card-info/${store.cardInfo.cardId}`}
            title="Card Info"
          />
        </div>
      </div>
    </div>
  {/if}

  <!-- Change Note Type (real upstream ts/routes/change-notetype/[notetypeId]) -->
  {#if store.changeNotetype.open && store.changeNotetype.notetypeId}
    <div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Change Note Type">
      <div class="modal-window change-nt-window">
        <div class="modal-header">
          <h3 class="modal-title">
            Change Note Type ({store.changeNotetype.noteIds.length} note{store.changeNotetype.noteIds.length === 1 ? "" : "s"})
          </h3>
          <button
            class="modal-close"
            onclick={() => (store.changeNotetype = { open: false, noteIds: [], notetypeId: 0 })}
          >×</button>
        </div>
        <div class="modal-body flush">
          <UpstreamFrame
            src={`/change-notetype/${store.changeNotetype.notetypeId}`}
            title="Change Note Type"
          />
        </div>
      </div>
    </div>
  {/if}

  <!-- Image Occlusion (real upstream ts/routes/image-occlusion/[pathOrNoteId]) -->
  {#if store.imageOcclusion.open && store.imageOcclusion.pathOrNoteId}
    <div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Image Occlusion">
      <div class="modal-window stats-window">
        <div class="modal-header">
          <h3 class="modal-title">Image Occlusion</h3>
          <button
            class="modal-close"
            onclick={() => {
              store.imageOcclusion = { open: false, pathOrNoteId: "" };
              store.bumpOpRevision();
            }}
          >×</button>
        </div>
        <div class="modal-body flush">
          <UpstreamFrame
            src={`/image-occlusion/${encodeURIComponent(store.imageOcclusion.pathOrNoteId)}`}
            title="Image Occlusion"
          />
        </div>
      </div>
    </div>
  {/if}

  {#if store.manageNotetypesOpen}
    <ManageNotetypesModal />
  {/if}

  {#if store.fieldsModal.open}
    <FieldsModal />
  {/if}

  {#if store.cardTemplates.open}
    <CardTemplatesModal />
  {/if}

  {#if store.importModal.open}
    <ImportModal />
  {/if}

  {#if store.exportModal.open}
    <ExportModal />
  {/if}

  {#if store.preferencesOpen}
    <PreferencesModal />
  {/if}

  {#if store.customStudyOpen}
    <CustomStudyModal />
  {/if}

  {#if store.filteredDeck.open}
    <FilteredDeckModal />
  {/if}

  {#if store.deckDescription.open}
    <DeckDescriptionModal />
  {/if}

  {#if store.studyDeckOpen}
    <StudyDeckModal />
  {/if}

  {#if store.previewer.open}
    <CardPreviewerModal />
  {/if}

  {#if store.audioRecorder.open}
    <AudioRecorderModal />
  {/if}

  {#if store.aiGeneratorOpen}
    <AiCardGeneratorModal />
  {/if}

  <MaintenanceModals />
{/if}

<DialogOverlay />

<style>
  .boot-screen {
    height: 100dvh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    background: var(--canvas);
    color: var(--fg-subtle);
    font-size: 14px;
  }
  .boot-spinner {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: 2.5px solid var(--border);
    border-top-color: var(--accent);
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .app-shell {
    height: 100dvh;
    display: flex;
    flex-direction: column;
    background: var(--canvas);
    color: var(--fg);
    overflow: hidden;
    min-width: 0;
    max-width: 100%;
  }
  .main-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
    overflow: hidden;
  }
  .add-window {
    width: min(740px, 94vw);
    height: min(680px, 88vh);
  }
  .stats-window {
    width: min(1040px, 95vw);
    height: min(800px, 90vh);
  }
  .deck-opts-window {
    width: min(900px, 95vw);
    height: min(780px, 90vh);
  }
  .card-info-window {
    width: min(680px, 94vw);
    height: min(640px, 88vh);
  }
  .change-nt-window {
    width: min(720px, 94vw);
    height: min(620px, 86vh);
  }
  .demo-banner-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: calc(6px + var(--safe-top)) calc(14px + var(--safe-right)) 6px calc(14px + var(--safe-left));
    background: #1e293b;
    color: #f1f5f9;
    font-size: 12px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.12);
    z-index: 100;
    flex-shrink: 0;
  }
  .demo-banner-text {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
  }
  .demo-banner-top a {
    color: #38bdf8;
    text-decoration: underline;
    font-weight: 500;
    white-space: nowrap;
  }
  .demo-banner-top a:hover {
    color: #7dd3fc;
  }
  .demo-banner-links {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    color: #94a3b8;
    flex-shrink: 0;
  }
  .demo-banner-close {
    border: none;
    background: transparent;
    color: #94a3b8;
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .demo-banner-close:hover {
    color: #f1f5f9;
    background: rgba(255, 255, 255, 0.1);
  }
  @media (max-width: 640px) {
    .demo-banner-top {
      padding: calc(5px + var(--safe-top)) calc(10px + var(--safe-right)) 5px calc(10px + var(--safe-left));
      font-size: 11.5px;
    }
    .banner-disclaimer {
      display: none;
    }
    .banner-sep {
      display: none;
    }
  }
  @media (max-width: 640px) {
    .add-window,
    .stats-window,
    .deck-opts-window,
    .card-info-window,
    .change-nt-window {
      width: 100% !important;
      max-width: 100% !important;
      height: 100dvh !important;
      max-height: 100dvh !important;
      border-radius: 0 !important;
    }
  }
</style>
