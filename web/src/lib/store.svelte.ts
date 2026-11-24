// Cardly Reactive Application Store (Svelte 5 Runes)

export type MainView = "decks" | "overview" | "review";
export type ThemeMode = "auto" | "light" | "dark";

export interface ToastItem {
  id: number;
  text: string;
  kind: "info" | "warning" | "error";
}

export interface DialogRequest {
  kind: "confirm" | "alert" | "prompt";
  title: string;
  text: string;
  defaultValue?: string;
  placeholder?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
  resolve: (value: any) => void;
}

export interface BrowserModalState {
  open: boolean;
  initialSearch: string;
  initialCardId?: number;
}

export interface DeckOptionsModalState {
  open: boolean;
  deckId: number;
}

export interface CardInfoModalState {
  open: boolean;
  cardId: number;
  title?: string;
}

export interface ChangeNotetypeModalState {
  open: boolean;
  noteIds: number[];
  notetypeId: number;
}

export interface ImportModalState {
  open: boolean;
  mode: "pick" | "apkg" | "csv";
  uploadId?: string;
  filename?: string;
}

export interface ExportModalState {
  open: boolean;
  deckId?: number;
  noteIds?: number[];
}

export interface FieldsModalState {
  open: boolean;
  notetypeId: number;
}

export interface CardTemplatesModalState {
  open: boolean;
  notetypeId: number;
  noteId?: number;
  ord?: number;
}

export interface EditCurrentModalState {
  open: boolean;
  noteId: number;
  cardId?: number;
}

export interface PreviewerModalState {
  open: boolean;
  cardIds: number[];
  index: number;
}

export interface ImageOcclusionModalState {
  open: boolean;
  pathOrNoteId: string;
}

export interface FilteredDeckModalState {
  open: boolean;
  deckId?: number; // 0 or undefined = create new
}

export interface DeckDescriptionModalState {
  open: boolean;
  deckId: number;
}

export interface AudioRecorderState {
  open: boolean;
  resolve?: (filename: string | null) => void;
}

export interface ProgressState {
  visible: boolean;
  label: string;
  value?: number;
  max?: number;
  cancellable?: boolean;
}

class CardlyStore {
  // Auth
  authStatus = $state<"loading" | "authenticated" | "unauthenticated">("loading");
  username = $state<string>("");
  apiToken = $state<string>("");
  demoMode = $state<boolean>(false);
  isDemoUser = $state<boolean>(false);
  demoResetIntervalMinutes = $state<number>(120);
  demoResetsInSeconds = $state<number | null>(null);

  // Navigation
  view = $state<MainView>("decks");
  currentDeckId = $state<number>(1);

  // Visual preferences
  theme = $state<ThemeMode>("auto");
  systemDark = $state<boolean>(false);
  minimalistMode = $state<boolean>(false);
  hideTopBar = $state<boolean>(false);
  hideBottomBar = $state<boolean>(false);
  answerKeys = $state<Record<number, string>>({ 1: "1", 2: "2", 3: "3", 4: "4" });
  spacebarRatesCard = $state<boolean>(true);

  // Reactivity & Undo
  opRevision = $state<number>(0);
  undoLabel = $state<string>("");
  redoLabel = $state<string>("");

  // Active modals & overlays
  addCardsOpen = $state<boolean>(false);
  aiGeneratorOpen = $state<boolean>(false);
  editCurrent = $state<EditCurrentModalState>({ open: false, noteId: 0 });
  browser = $state<BrowserModalState>({ open: false, initialSearch: "" });
  statsOpen = $state<boolean>(false);
  deckOptions = $state<DeckOptionsModalState>({ open: false, deckId: 1 });
  cardInfo = $state<CardInfoModalState>({ open: false, cardId: 0 });
  changeNotetype = $state<ChangeNotetypeModalState>({
    open: false,
    noteIds: [],
    notetypeId: 0,
  });
  manageNotetypesOpen = $state<boolean>(false);
  fieldsModal = $state<FieldsModalState>({ open: false, notetypeId: 0 });
  cardTemplates = $state<CardTemplatesModalState>({ open: false, notetypeId: 0 });
  importModal = $state<ImportModalState>({ open: false, mode: "pick" });
  exportModal = $state<ExportModalState>({ open: false });
  preferencesOpen = $state<boolean>(false);
  customStudyOpen = $state<boolean>(false);
  filteredDeck = $state<FilteredDeckModalState>({ open: false });
  deckDescription = $state<DeckDescriptionModalState>({ open: false, deckId: 0 });
  studyDeckOpen = $state<boolean>(false);
  previewer = $state<PreviewerModalState>({ open: false, cardIds: [], index: 0 });
  imageOcclusion = $state<ImageOcclusionModalState>({ open: false, pathOrNoteId: "" });
  checkDbOpen = $state<boolean>(false);
  checkMediaOpen = $state<boolean>(false);
  emptyCardsOpen = $state<boolean>(false);
  aboutOpen = $state<boolean>(false);
  syncInfoOpen = $state<boolean>(false);
  audioRecorder = $state<AudioRecorderState>({ open: false });
  progress = $state<ProgressState>({ visible: false, label: "" });

  // Editor active notetype context (for Fields / Cards buttons inside ts/routes/editor)
  activeEditorNotetypeId = $state<number>(0);
  activeEditorNoteId = $state<number>(0);

  // Toasts & Dialogs
  toasts = $state<ToastItem[]>([]);
  dialog = $state<DialogRequest | null>(null);
  private toastSeq = 0;

  // Network & PWA
  isOnline = $state<boolean>(typeof navigator !== "undefined" ? navigator.onLine : true);
  deferredInstallPrompt = $state<any>(null);
  isStandalone = $state<boolean>(
    typeof window !== "undefined" &&
      (window.matchMedia("(display-mode: standalone)").matches || (window.navigator as any).standalone === true)
  );
  lastGlobalError = $state<string | null>(null);

  get canInstall(): boolean {
    return this.deferredInstallPrompt !== null && !this.isStandalone;
  }

  setInstallPrompt(ev: any) {
    this.deferredInstallPrompt = ev;
  }

  async installCardly(): Promise<boolean> {
    if (!this.deferredInstallPrompt) return false;
    try {
      this.deferredInstallPrompt.prompt();
      const choice = await this.deferredInstallPrompt.userChoice;
      if (choice.outcome === "accepted") {
        this.deferredInstallPrompt = null;
        this.toast("Cardly installed successfully!");
        return true;
      }
    } catch (err) {
      console.warn("PWA install error:", err);
    }
    return false;
  }

  setOnline(online: boolean) {
    this.isOnline = online;
    if (online) {
      this.toast("Connection restored", "info");
      this.bumpOpRevision();
    } else {
      this.toast("Network disconnected — offline mode", "warning");
    }
  }

  setGlobalError(msg: string | null) {
    this.lastGlobalError = msg;
  }

  get isDark(): boolean {
    if (this.theme === "dark") return true;
    if (this.theme === "light") return false;
    return this.systemDark;
  }

  get hasOpenModal(): boolean {
    return (
      this.addCardsOpen ||
      this.aiGeneratorOpen ||
      this.editCurrent.open ||
      this.browser.open ||
      this.statsOpen ||
      this.deckOptions.open ||
      this.cardInfo.open ||
      this.changeNotetype.open ||
      this.manageNotetypesOpen ||
      this.fieldsModal.open ||
      this.cardTemplates.open ||
      this.importModal.open ||
      this.exportModal.open ||
      this.preferencesOpen ||
      this.customStudyOpen ||
      this.filteredDeck.open ||
      this.deckDescription.open ||
      this.studyDeckOpen ||
      this.previewer.open ||
      this.imageOcclusion.open ||
      this.checkDbOpen ||
      this.checkMediaOpen ||
      this.emptyCardsOpen ||
      this.aboutOpen ||
      this.syncInfoOpen ||
      this.audioRecorder.open ||
      this.dialog !== null
    );
  }

  initTheme() {
    const saved = localStorage.getItem("cardly-theme") as ThemeMode | null;
    if (saved === "light" || saved === "dark" || saved === "auto") {
      this.theme = saved;
    }
    this.minimalistMode = localStorage.getItem("cardly-minimalist") === "1";
    this.hideTopBar = localStorage.getItem("cardly-hide-top") === "1";
    this.hideBottomBar = localStorage.getItem("cardly-hide-bottom") === "1";
    const savedKeys = localStorage.getItem("cardly-answer-keys");
    if (savedKeys) {
      try {
        this.answerKeys = JSON.parse(savedKeys);
      } catch {
        // ignore
      }
    }
    const savedSpace = localStorage.getItem("cardly-spacebar-rates");
    if (savedSpace !== null) {
      this.spacebarRatesCard = savedSpace !== "0";
    }

    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    this.systemDark = mq.matches;
    mq.addEventListener("change", (ev) => {
      this.systemDark = ev.matches;
      this.applyThemeClasses();
    });
    this.applyThemeClasses();
  }

  setTheme(mode: ThemeMode) {
    this.theme = mode;
    localStorage.setItem("cardly-theme", mode);
    this.applyThemeClasses();
  }

  setMinimalist(val: boolean) {
    this.minimalistMode = val;
    localStorage.setItem("cardly-minimalist", val ? "1" : "0");
    this.applyThemeClasses();
  }

  setHideBars(top: boolean, bottom: boolean) {
    this.hideTopBar = top;
    this.hideBottomBar = bottom;
    localStorage.setItem("cardly-hide-top", top ? "1" : "0");
    localStorage.setItem("cardly-hide-bottom", bottom ? "1" : "0");
  }

  setAnswerKeys(keys: Record<number, string>, spacebarRates: boolean) {
    this.answerKeys = keys;
    this.spacebarRatesCard = spacebarRates;
    localStorage.setItem("cardly-answer-keys", JSON.stringify(keys));
    localStorage.setItem("cardly-spacebar-rates", spacebarRates ? "1" : "0");
  }

  applyThemeClasses() {
    const dark = this.isDark;
    const root = document.documentElement;
    root.classList.toggle("night-mode", dark);
    root.classList.toggle("nightMode", dark);
    document.body.classList.toggle("night-mode", dark);
    document.body.classList.toggle("nightMode", dark);
    document.body.classList.toggle("minimalist", this.minimalistMode);
    document.body.classList.toggle("fancy", !this.minimalistMode);
    // Sync theme into any embedded same-origin iframes (upstream ts/ pages & reviewer)
    for (const iframe of Array.from(document.querySelectorAll("iframe"))) {
      try {
        const doc = iframe.contentDocument;
        if (!doc) continue;
        doc.documentElement.classList.toggle("night-mode", dark);
        doc.documentElement.classList.toggle("nightMode", dark);
        if (doc.body) {
          doc.body.classList.toggle("night-mode", dark);
          doc.body.classList.toggle("nightMode", dark);
        }
      } catch {
        // cross-origin or not ready
      }
    }
  }

  bumpOpRevision() {
    this.opRevision += 1;
  }

  toast(text: string, kind: "info" | "warning" | "error" = "info", durationMs = 2600) {
    if (!text) return;
    const id = ++this.toastSeq;
    this.toasts = [...this.toasts, { id, text, kind }];
    setTimeout(() => {
      this.toasts = this.toasts.filter((t) => t.id !== id);
    }, durationMs);
  }

  confirm(text: string, title = "Cardly", opts?: { danger?: boolean; confirmLabel?: string }): Promise<boolean> {
    return new Promise((resolve) => {
      this.dialog = {
        kind: "confirm",
        title,
        text,
        danger: opts?.danger,
        confirmLabel: opts?.confirmLabel ?? "OK",
        cancelLabel: "Cancel",
        resolve: (v: boolean) => {
          this.dialog = null;
          resolve(v);
        },
      };
    });
  }

  alert(text: string, title = "Cardly"): Promise<void> {
    return new Promise((resolve) => {
      this.dialog = {
        kind: "alert",
        title,
        text,
        confirmLabel: "OK",
        resolve: () => {
          this.dialog = null;
          resolve();
        },
      };
    });
  }

  prompt(
    text: string,
    defaultValue = "",
    title = "Cardly",
    placeholder = ""
  ): Promise<string | null> {
    return new Promise((resolve) => {
      this.dialog = {
        kind: "prompt",
        title,
        text,
        defaultValue,
        placeholder,
        confirmLabel: "OK",
        cancelLabel: "Cancel",
        resolve: (v: string | null) => {
          this.dialog = null;
          resolve(v);
        },
      };
    });
  }

  openBrowser(initialSearch = "", initialCardId?: number) {
    this.browser = { open: true, initialSearch, initialCardId };
  }

  openDeckOptions(deckId: number) {
    this.deckOptions = { open: true, deckId };
  }

  openCardInfo(cardId: number, title?: string) {
    this.cardInfo = { open: true, cardId, title };
  }

  openEditCurrent(noteId: number, cardId?: number) {
    this.activeEditorNoteId = noteId;
    this.editCurrent = { open: true, noteId, cardId };
  }

  openChangeNotetype(noteIds: number[], notetypeId: number) {
    this.changeNotetype = { open: true, noteIds, notetypeId };
  }

  openFields(notetypeId: number) {
    this.fieldsModal = { open: true, notetypeId };
  }

  openCardTemplates(notetypeId: number, noteId?: number, ord = 0) {
    this.cardTemplates = { open: true, notetypeId, noteId, ord };
  }

  openImport(mode: "pick" | "apkg" | "csv" = "pick", uploadId?: string, filename?: string) {
    this.importModal = { open: true, mode, uploadId, filename };
  }

  openExport(opts?: { deckId?: number; noteIds?: number[] }) {
    this.exportModal = { open: true, deckId: opts?.deckId, noteIds: opts?.noteIds };
  }

  openFilteredDeck(deckId = 0) {
    this.filteredDeck = { open: true, deckId };
  }

  openDeckDescription(deckId: number) {
    this.deckDescription = { open: true, deckId };
  }

  openPreviewer(cardIds: number[], index = 0) {
    if (cardIds.length === 0) return;
    this.previewer = { open: true, cardIds, index };
  }

  openImageOcclusion(pathOrNoteId: string) {
    this.imageOcclusion = { open: true, pathOrNoteId };
  }

  requestAudioRecording(): Promise<string | null> {
    return new Promise((resolve) => {
      this.audioRecorder = {
        open: true,
        resolve: (fname) => {
          this.audioRecorder = { open: false };
          resolve(fname);
        },
      };
    });
  }
}

export const store = new CardlyStore();
