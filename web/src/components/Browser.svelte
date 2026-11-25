<script lang="ts">
  import {
    addNoteTags,
    allBrowserColumns,
    buildSearchString,
    buryOrSuspendCards,
    cardsOfNote,
    clearUnusedTags,
    deckTree,
    getCard,
    getConfigJson,
    getDeckNames,
    getNote,
    getNotetypeNames,
    getSingleNotetypeOfNotes,
    gradeNow,
    joinSearchNodes,
    removeNoteTags,
    removeNotes,
    restoreBuriedAndSuspendedCards,
    scheduleCardsAsNew,
    searchCards,
    searchNotes,
    setActiveBrowserColumns,
    setConfigJson,
    setDeck,
    setDueDate,
    setFlag,
    sortCards,
    tagTree,
  } from "@generated/backend";
  import type { DeckNameId, DeckTreeNode } from "@generated/anki/decks_pb";
  import type { NotetypeNameId } from "@generated/anki/notetypes_pb";
  import {
    BuryOrSuspendCardsRequest_Mode,
    CardAnswer_Rating,
  } from "@generated/anki/scheduler_pb";
  import {
    BrowserColumns_Column,
    BrowserRow_Color,
    SearchNode,
    SearchNode_CardState,
    SearchNode_Flag,
    SearchNode_Group_Joiner,
    SearchNode_Rating,
    SortOrder,
  } from "@generated/anki/search_pb";
  import type { TagTreeNode } from "@generated/anki/tags_pb";
  import { fetchBrowserRowsBatch, type BrowserRowJson } from "../lib/api";
  import { store } from "../lib/store.svelte";
  import Icon from "./Icon.svelte";
  import FindAndReplaceModal from "./FindAndReplaceModal.svelte";
  import FindDuplicatesModal from "./FindDuplicatesModal.svelte";
  import UpstreamEditor from "./UpstreamEditor.svelte";

  const DEFAULT_COLUMNS = ["noteFld", "template", "cardDue", "deck", "noteTags"];
  const ROW_HEIGHT = 28;
  const FLAG_LABELS = [
    { idx: 1, name: "Red", color: "var(--flag-1)", enumVal: SearchNode_Flag.RED },
    { idx: 2, name: "Orange", color: "var(--flag-2)", enumVal: SearchNode_Flag.ORANGE },
    { idx: 3, name: "Green", color: "var(--flag-3)", enumVal: SearchNode_Flag.GREEN },
    { idx: 4, name: "Blue", color: "var(--flag-4)", enumVal: SearchNode_Flag.BLUE },
    { idx: 5, name: "Pink", color: "var(--flag-5)", enumVal: SearchNode_Flag.PINK },
    { idx: 6, name: "Turquoise", color: "var(--flag-6)", enumVal: SearchNode_Flag.TURQUOISE },
    { idx: 7, name: "Purple", color: "var(--flag-7)", enumVal: SearchNode_Flag.PURPLE },
  ];

  // Mode & layout
  let notesMode = $state(false);
  let verticalSplit = $state(false);
  let sidebarVisible = $state(typeof window !== "undefined" ? window.innerWidth > 768 : true);

  // Search & results
  let searchText = $state(store.browser.initialSearch || "");
  let searchHistory = $state<string[]>([]);
  let savedSearches = $state<Record<string, string>>({});
  let resultIds = $state<number[]>([]);
  let rowCache = $state<Map<number, BrowserRowJson>>(new Map());
  let searching = $state(false);
  let searchError = $state("");

  // Columns & sorting
  let allColumns = $state<BrowserColumns_Column[]>([]);
  let activeColumnKeys = $state<string[]>([...DEFAULT_COLUMNS]);
  let sortColumn = $state<string>("noteFld");
  let sortReverse = $state<boolean>(false);
  let columnPickerOpen = $state(false);

  // Selection & active editor note
  let selectedIds = $state<Set<number>>(new Set());
  let lastClickedIdx = $state<number>(-1);
  let activeNoteId = $state<number | null>(null);
  let activeCardId = $state<number | null>(null);

  // Sidebar data
  let decks = $state<DeckTreeNode[]>([]);
  let flatDeckNames = $state<DeckNameId[]>([]);
  let notetypes = $state<NotetypeNameId[]>([]);
  let tags = $state<TagTreeNode[]>([]);
  let openSections = $state<Record<string, boolean>>({
    saved: true,
    today: true,
    flags: false,
    states: true,
    decks: true,
    notetypes: false,
    tags: true,
  });

  // Virtual scroll state
  let tableWrapEl = $state<HTMLDivElement | null>(null);
  let scrollTop = $state(0);
  let viewportHeight = $state(420);

  // Menus & sub-modals
  let rowMenu = $state<{ x: number; y: number } | null>(null);
  let findReplaceOpen = $state(false);
  let findDupesOpen = $state(false);
  let changeDeckOpen = $state(false);
  let changeDeckTargetId = $state<number>(1);
  let actionsMenuOpen = $state<"notes" | "cards" | null>(null);

  const visibleColumns = $derived(
    activeColumnKeys
      .map((k) => allColumns.find((c) => c.key === k))
      .filter((c): c is BrowserColumns_Column => Boolean(c))
  );

  const totalHeight = $derived(resultIds.length * ROW_HEIGHT);
  const startIdx = $derived(Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - 8));
  const endIdx = $derived(
    Math.min(resultIds.length, Math.ceil((scrollTop + viewportHeight) / ROW_HEIGHT) + 12)
  );
  const visibleSlice = $derived(
    resultIds.slice(startIdx, endIdx).map((id, i) => ({
      id,
      index: startIdx + i,
      row: rowCache.get(id),
    }))
  );

  let batchLoading = new Set<number>();

  async function ensureVisibleRowsLoaded() {
    const missing: number[] = [];
    for (let i = startIdx; i < endIdx; i++) {
      const id = resultIds[i];
      if (id !== undefined && !rowCache.has(id) && !batchLoading.has(id)) {
        missing.push(id);
      }
    }
    if (missing.length === 0) return;
    const chunk = missing.slice(0, 120);
    for (const id of chunk) batchLoading.add(id);
    try {
      const fetched = await fetchBrowserRowsBatch(chunk);
      const next = new Map(rowCache);
      for (const r of fetched) {
        next.set(r.id, r);
      }
      rowCache = next;
    } catch {
      // ignore transient errors
    } finally {
      for (const id of chunk) batchLoading.delete(id);
    }
  }

  async function loadSidebarAndColumns() {
    try {
      const [colsResp, treeResp, deckNamesResp, ntResp, tagResp, savedResp] = await Promise.all([
        allBrowserColumns({}, { alertOnError: false }),
        deckTree({ now: BigInt(0) }, { alertOnError: false }),
        getDeckNames({ skipEmptyDefault: false, includeFiltered: true }, { alertOnError: false }),
        getNotetypeNames({}, { alertOnError: false }),
        tagTree({}, { alertOnError: false }),
        getConfigJson({ val: "savedFilters" }, { alertOnError: false }).catch(() => ({
          json: new Uint8Array(0),
        })),
      ]);
      allColumns = colsResp.columns;
      decks = treeResp.children;
      flatDeckNames = deckNamesResp.entries;
      notetypes = ntResp.entries;
      tags = tagResp.children;

      if (savedResp.json.length > 0) {
        try {
          const parsed = JSON.parse(new TextDecoder().decode(savedResp.json));
          if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
            savedSearches = parsed;
          }
        } catch {
          // ignore
        }
      }

      const savedCols = localStorage.getItem("cardly-browser-cols");
      if (savedCols) {
        try {
          const parsed = JSON.parse(savedCols);
          if (Array.isArray(parsed) && parsed.length > 0) {
            activeColumnKeys = parsed;
          }
        } catch {
          // ignore
        }
      }
      await setActiveBrowserColumns({ vals: activeColumnKeys }, { alertOnError: false });
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to initialize Browser", "error");
    }
  }

  async function runSearch(query = searchText) {
    searching = true;
    searchError = "";
    try {
      await setActiveBrowserColumns({ vals: activeColumnKeys }, { alertOnError: false });
      const order = new SortOrder({
        value: sortColumn
          ? { case: "builtin", value: { column: sortColumn, reverse: sortReverse } }
          : { case: "none", value: {} },
      });

      const resp = notesMode
        ? await searchNotes({ search: query, order }, { alertOnError: false })
        : await searchCards({ search: query, order }, { alertOnError: false });

      resultIds = resp.ids.map((id) => Number(id));
      rowCache = new Map();
      batchLoading.clear();

      if (query.trim() && !searchHistory.includes(query.trim())) {
        searchHistory = [query.trim(), ...searchHistory].slice(0, 20);
      }

      // Preserve or select first item
      if (store.browser.initialCardId && resultIds.includes(store.browser.initialCardId)) {
        selectSingle(store.browser.initialCardId, resultIds.indexOf(store.browser.initialCardId));
      } else if (resultIds.length > 0 && selectedIds.size === 0) {
        selectSingle(resultIds[0], 0);
      } else {
        // Keep only ids that still exist
        const keep = new Set([...selectedIds].filter((id) => resultIds.includes(id)));
        selectedIds = keep;
        if (keep.size === 0) {
          activeNoteId = null;
          activeCardId = null;
        }
      }
      await ensureVisibleRowsLoaded();
    } catch (err) {
      searchError = err instanceof Error ? err.message : "Invalid search query";
      resultIds = [];
      rowCache = new Map();
    } finally {
      searching = false;
    }
  }

  async function selectSingle(id: number, idx: number) {
    selectedIds = new Set([id]);
    lastClickedIdx = idx;
    await loadEditorForId(id);
  }

  async function loadEditorForId(id: number) {
    if (notesMode) {
      activeNoteId = id;
      activeCardId = null;
      store.activeEditorNoteId = id;
    } else {
      activeCardId = id;
      try {
        const card = await getCard({ cid: BigInt(id) }, { alertOnError: false });
        activeNoteId = Number(card.noteId);
        store.activeEditorNoteId = activeNoteId;
      } catch {
        activeNoteId = null;
      }
    }
  }

  function handleRowClick(id: number, idx: number, e: MouseEvent) {
    rowMenu = null;
    if (e.shiftKey && lastClickedIdx !== -1) {
      const lo = Math.min(lastClickedIdx, idx);
      const hi = Math.max(lastClickedIdx, idx);
      const next = new Set(e.ctrlKey || e.metaKey ? selectedIds : []);
      for (let i = lo; i <= hi; i++) {
        next.add(resultIds[i]);
      }
      selectedIds = next;
      void loadEditorForId(id);
    } else if (e.ctrlKey || e.metaKey) {
      const next = new Set(selectedIds);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      selectedIds = next;
      lastClickedIdx = idx;
      if (next.size === 1) {
        void loadEditorForId([...next][0]);
      } else {
        activeNoteId = null;
      }
    } else {
      void selectSingle(id, idx);
    }
  }

  function handleRowContextMenu(id: number, idx: number, e: MouseEvent) {
    e.preventDefault();
    if (!selectedIds.has(id)) {
      void selectSingle(id, idx);
    }
    rowMenu = {
      x: Math.min(e.clientX, window.innerWidth - 230),
      y: Math.min(e.clientY, window.innerHeight - 360),
    };
  }

  async function applySidebarFilter(node: SearchNode, e: MouseEvent) {
    try {
      let nextQuery: string;
      const effectiveNode = e.altKey
        ? new SearchNode({ filter: { case: "negated", value: node } })
        : node;

      if ((e.ctrlKey || e.metaKey || e.shiftKey) && searchText.trim()) {
        const joiner = e.shiftKey ? SearchNode_Group_Joiner.OR : SearchNode_Group_Joiner.AND;
        const existing = new SearchNode({
          filter: { case: "parsableText", value: searchText.trim() },
        });
        nextQuery = (
          await joinSearchNodes({
            joiner,
            existingNode: existing,
            additionalNode: effectiveNode,
          })
        ).val;
      } else {
        nextQuery = (await buildSearchString(effectiveNode)).val;
      }
      searchText = nextQuery;
      selectedIds = new Set();
      await runSearch(nextQuery);
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Filter failed", "error");
    }
  }

  async function handleSaveSearch() {
    const q = searchText.trim();
    if (!q) return;
    const name = await store.prompt("Name for saved search:", "", "Save Current Search");
    if (!name || !name.trim()) return;
    const next = { ...savedSearches, [name.trim()]: q };
    savedSearches = next;
    await setConfigJson({
      key: "savedFilters",
      valueJson: new TextEncoder().encode(JSON.stringify(next)),
      undoable: false,
    });
    store.toast(`Saved search "${name.trim()}"`);
  }

  async function deleteSavedSearch(name: string, e: MouseEvent) {
    e.stopPropagation();
    const next = { ...savedSearches };
    delete next[name];
    savedSearches = next;
    await setConfigJson({
      key: "savedFilters",
      valueJson: new TextEncoder().encode(JSON.stringify(next)),
      undoable: false,
    });
  }

  async function toggleColumn(key: string) {
    let next: string[];
    if (activeColumnKeys.includes(key)) {
      if (activeColumnKeys.length <= 1) return;
      next = activeColumnKeys.filter((k) => k !== key);
    } else {
      next = [...activeColumnKeys, key];
    }
    activeColumnKeys = next;
    localStorage.setItem("cardly-browser-cols", JSON.stringify(next));
    await setActiveBrowserColumns({ vals: next });
    rowCache = new Map();
    await ensureVisibleRowsLoaded();
  }

  async function toggleSort(colKey: string) {
    if (sortColumn === colKey) {
      sortReverse = !sortReverse;
    } else {
      sortColumn = colKey;
      sortReverse = false;
    }
    await runSearch();
  }

  // ============================================================================
  // Helper to resolve selected Card IDs & Note IDs in both Cards and Notes mode
  // ============================================================================

  async function getSelectedCardAndNoteIds(): Promise<{ cardIds: number[]; noteIds: number[] }> {
    const sel = [...selectedIds];
    if (sel.length === 0) return { cardIds: [], noteIds: [] };
    if (notesMode) {
      const cardIds: number[] = [];
      for (const nid of sel) {
        const res = await cardsOfNote({ nid: BigInt(nid) }, { alertOnError: false });
        for (const cid of res.cids) cardIds.push(Number(cid));
      }
      return { cardIds, noteIds: sel };
    } else {
      const noteSet = new Set<number>();
      for (const cid of sel) {
        const c = await getCard({ cid: BigInt(cid) }, { alertOnError: false });
        noteSet.add(Number(c.noteId));
      }
      return { cardIds: sel, noteIds: [...noteSet] };
    }
  }

  // ============================================================================
  // Bulk Actions
  // ============================================================================

  async function bulkToggleSuspend() {
    rowMenu = null;
    actionsMenuOpen = null;
    const { cardIds } = await getSelectedCardAndNoteIds();
    if (cardIds.length === 0) return;
    // Check if first card is suspended -> unsuspend all, else suspend all (matches Anki)
    const first = await getCard({ cid: BigInt(cardIds[0]) });
    if (first.queue === -1) {
      await restoreBuriedAndSuspendedCards({ cids: cardIds.map((id) => BigInt(id)) });
      store.toast(`Unsuspended ${cardIds.length} card(s)`);
    } else {
      await buryOrSuspendCards({
        cardIds: cardIds.map((id) => BigInt(id)),
        noteIds: [],
        mode: BuryOrSuspendCardsRequest_Mode.SUSPEND,
      });
      store.toast(`Suspended ${cardIds.length} card(s)`);
    }
    await runSearch();
  }

  async function bulkToggleMark() {
    rowMenu = null;
    actionsMenuOpen = null;
    const { noteIds } = await getSelectedCardAndNoteIds();
    if (noteIds.length === 0) return;
    const first = await getNote({ nid: BigInt(noteIds[0]) });
    const isMarked = first.tags.some((t) => t.toLowerCase() === "marked");
    if (isMarked) {
      await removeNoteTags({ noteIds: noteIds.map((id) => BigInt(id)), tags: "marked" });
      store.toast(`Unmarked ${noteIds.length} note(s)`);
    } else {
      await addNoteTags({ noteIds: noteIds.map((id) => BigInt(id)), tags: "marked" });
      store.toast(`Marked ${noteIds.length} note(s)`);
    }
    await runSearch();
  }

  async function bulkSetFlag(flag: number) {
    rowMenu = null;
    actionsMenuOpen = null;
    const { cardIds } = await getSelectedCardAndNoteIds();
    if (cardIds.length === 0) return;
    await setFlag({ cardIds: cardIds.map((id) => BigInt(id)), flag });
    store.toast(flag === 0 ? "Cleared flags" : `Set flag on ${cardIds.length} card(s)`);
    await runSearch();
  }

  async function bulkAddTags() {
    rowMenu = null;
    actionsMenuOpen = null;
    const { noteIds } = await getSelectedCardAndNoteIds();
    if (noteIds.length === 0) return;
    const tagsStr = await store.prompt("Enter tags to add (space-separated):", "", "Add Tags");
    if (!tagsStr || !tagsStr.trim()) return;
    await addNoteTags({ noteIds: noteIds.map((id) => BigInt(id)), tags: tagsStr.trim() });
    store.toast(`Added tags to ${noteIds.length} note(s)`);
    await loadSidebarAndColumns();
    await runSearch();
  }

  async function bulkRemoveTags() {
    rowMenu = null;
    actionsMenuOpen = null;
    const { noteIds } = await getSelectedCardAndNoteIds();
    if (noteIds.length === 0) return;
    const tagsStr = await store.prompt("Enter tags to remove (space-separated):", "", "Remove Tags");
    if (!tagsStr || !tagsStr.trim()) return;
    await removeNoteTags({ noteIds: noteIds.map((id) => BigInt(id)), tags: tagsStr.trim() });
    store.toast(`Removed tags from ${noteIds.length} note(s)`);
    await loadSidebarAndColumns();
    await runSearch();
  }

  async function bulkClearUnusedTags() {
    actionsMenuOpen = null;
    const res = await clearUnusedTags({});
    store.toast(`Cleared ${res.count} unused tag(s)`);
    await loadSidebarAndColumns();
  }

  async function bulkDelete() {
    rowMenu = null;
    actionsMenuOpen = null;
    const { noteIds } = await getSelectedCardAndNoteIds();
    if (noteIds.length === 0) return;
    const ok = await store.confirm(
      `Delete ${noteIds.length} selected note(s) and all their cards?`,
      "Delete Notes",
      { danger: true, confirmLabel: "Delete" }
    );
    if (!ok) return;
    await removeNotes({ noteIds: noteIds.map((id) => BigInt(id)), cardIds: [] });
    selectedIds = new Set();
    activeNoteId = null;
    store.toast(`Deleted ${noteIds.length} note(s)`);
    await runSearch();
  }

  async function bulkChangeDeckConfirm() {
    const { cardIds } = await getSelectedCardAndNoteIds();
    changeDeckOpen = false;
    if (cardIds.length === 0) return;
    await setDeck({
      cardIds: cardIds.map((id) => BigInt(id)),
      deckId: BigInt(changeDeckTargetId),
    });
    store.toast(`Moved ${cardIds.length} card(s)`);
    await runSearch();
  }

  async function bulkSetDueDate() {
    rowMenu = null;
    actionsMenuOpen = null;
    const { cardIds } = await getSelectedCardAndNoteIds();
    if (cardIds.length === 0) return;
    const days = await store.prompt(
      "Show in how many days? (0 = today, 1-5 = range, 3! = update interval)",
      "0",
      "Set Due Date"
    );
    if (!days || !days.trim()) return;
    await setDueDate({ cardIds: cardIds.map((id) => BigInt(id)), days: days.trim() });
    store.toast(`Updated due date for ${cardIds.length} card(s)`);
    await runSearch();
  }

  async function bulkForget() {
    rowMenu = null;
    actionsMenuOpen = null;
    const { cardIds } = await getSelectedCardAndNoteIds();
    if (cardIds.length === 0) return;
    const ok = await store.confirm(`Reset ${cardIds.length} card(s) to New?`, "Forget Cards");
    if (!ok) return;
    await scheduleCardsAsNew({
      cardIds: cardIds.map((id) => BigInt(id)),
      log: true,
      restorePosition: true,
      resetCounts: false,
    });
    store.toast(`Reset ${cardIds.length} card(s) to New`);
    await runSearch();
  }

  async function bulkReposition() {
    rowMenu = null;
    actionsMenuOpen = null;
    const { cardIds } = await getSelectedCardAndNoteIds();
    if (cardIds.length === 0) return;
    const startStr = await store.prompt("Start position for New cards:", "0", "Reposition New Cards");
    if (startStr === null) return;
    const startPos = Math.max(0, parseInt(startStr, 10) || 0);
    await sortCards({
      cardIds: cardIds.map((id) => BigInt(id)),
      startingFrom: startPos,
      stepSize: 1,
      randomize: false,
      shiftExisting: true,
    });
    store.toast(`Repositioned ${cardIds.length} card(s)`);
    await runSearch();
  }

  async function bulkGradeNow(rating: CardAnswer_Rating, label: string) {
    rowMenu = null;
    actionsMenuOpen = null;
    const { cardIds } = await getSelectedCardAndNoteIds();
    if (cardIds.length === 0) return;
    await gradeNow({ cardIds: cardIds.map((id) => BigInt(id)), rating });
    store.toast(`Graded ${cardIds.length} card(s) as ${label}`);
    await runSearch();
  }

  async function bulkChangeNotetype() {
    rowMenu = null;
    actionsMenuOpen = null;
    const { noteIds } = await getSelectedCardAndNoteIds();
    if (noteIds.length === 0) return;
    try {
      const ntid = (
        await getSingleNotetypeOfNotes({ noteIds: noteIds.map((id) => BigInt(id)) })
      ).ntid;
      store.openChangeNotetype(noteIds, Number(ntid));
    } catch (err) {
      store.toast(
        err instanceof Error ? err.message : "Select notes of a single note type",
        "warning"
      );
    }
  }

  async function openFindReplace() {
    rowMenu = null;
    actionsMenuOpen = null;
    const { noteIds } = await getSelectedCardAndNoteIds();
    if (noteIds.length === 0) {
      store.toast("Select one or more cards/notes first", "warning");
      return;
    }
    findReplaceNoteIds = noteIds;
    findReplaceOpen = true;
  }

  let findReplaceNoteIds = $state<number[]>([]);

  async function openPreview() {
    const { cardIds } = await getSelectedCardAndNoteIds();
    if (cardIds.length === 0 && resultIds.length > 0 && !notesMode) {
      store.openPreviewer(resultIds.slice(0, 200), 0);
      return;
    }
    if (cardIds.length > 0) {
      store.openPreviewer(cardIds, 0);
    }
  }

  async function openCardInfo() {
    rowMenu = null;
    const { cardIds } = await getSelectedCardAndNoteIds();
    if (cardIds.length > 0) store.openCardInfo(cardIds[0]);
  }

  async function openExportSelected() {
    rowMenu = null;
    actionsMenuOpen = null;
    const { noteIds } = await getSelectedCardAndNoteIds();
    if (noteIds.length === 0) return;
    store.openExport({ noteIds });
  }

  function rowColorClass(color: number): string {
    switch (color) {
      case BrowserRow_Color.MARKED:
        return "row-marked";
      case BrowserRow_Color.SUSPENDED:
        return "row-suspended";
      case BrowserRow_Color.BURIED:
        return "row-buried";
      case BrowserRow_Color.FLAG_RED:
        return "row-flag-1";
      case BrowserRow_Color.FLAG_ORANGE:
        return "row-flag-2";
      case BrowserRow_Color.FLAG_GREEN:
        return "row-flag-3";
      case BrowserRow_Color.FLAG_BLUE:
        return "row-flag-4";
      case BrowserRow_Color.FLAG_PINK:
        return "row-flag-5";
      case BrowserRow_Color.FLAG_TURQUOISE:
        return "row-flag-6";
      case BrowserRow_Color.FLAG_PURPLE:
        return "row-flag-7";
      default:
        return "";
    }
  }

  function flattenDecks(nodes: DeckTreeNode[], out: Array<{ node: DeckTreeNode; fullName: string }> = []) {
    for (const n of nodes) {
      out.push({ node: n, fullName: n.name });
      if (n.children.length > 0) flattenDecks(n.children, out);
    }
    return out;
  }

  function flattenTags(
    nodes: TagTreeNode[],
    prefix = "",
    out: Array<{ name: string; fullTag: string; level: number }> = []
  ) {
    for (const n of nodes) {
      const fullTag = prefix ? `${prefix}::${n.name}` : n.name;
      out.push({ name: n.name, fullTag, level: n.level });
      if (n.children.length > 0) flattenTags(n.children, fullTag, out);
    }
    return out;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (findReplaceOpen || findDupesOpen || changeDeckOpen || store.dialog) return;
    const tag = (e.target as HTMLElement | null)?.tagName;
    const inInput = tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
    const mod = e.ctrlKey || e.metaKey;

    if (e.key === "Escape") {
      if (rowMenu || columnPickerOpen || actionsMenuOpen) {
        rowMenu = null;
        columnPickerOpen = false;
        actionsMenuOpen = null;
      } else {
        closeBrowser();
      }
      return;
    }

    if (inInput) return;

    if (mod && e.key.toLowerCase() === "a") {
      e.preventDefault();
      selectedIds = new Set(resultIds);
    } else if (mod && e.key.toLowerCase() === "j") {
      e.preventDefault();
      void bulkToggleSuspend();
    } else if (mod && e.key.toLowerCase() === "k") {
      e.preventDefault();
      void bulkToggleMark();
    } else if (mod && e.key.toLowerCase() === "d" && !e.shiftKey) {
      e.preventDefault();
      if (selectedIds.size > 0) changeDeckOpen = true;
    } else if (mod && e.shiftKey && e.key.toLowerCase() === "a") {
      e.preventDefault();
      void bulkAddTags();
    } else if (mod && e.shiftKey && e.key.toLowerCase() === "d") {
      e.preventDefault();
      void bulkSetDueDate();
    } else if (mod && e.altKey && e.key.toLowerCase() === "f") {
      e.preventDefault();
      void openFindReplace();
    } else if (mod && !e.shiftKey && e.key >= "1" && e.key <= "7") {
      e.preventDefault();
      void bulkSetFlag(parseInt(e.key, 10));
    } else if (e.key === "Delete") {
      e.preventDefault();
      void bulkDelete();
    }
  }

  function closeBrowser() {
    store.browser = { open: false, initialSearch: "" };
  }

  $effect(() => {
    void loadSidebarAndColumns().then(() => runSearch(store.browser.initialSearch || ""));
  });

  $effect(() => {
    void startIdx;
    void endIdx;
    void resultIds;
    void ensureVisibleRowsLoaded();
  });

  $effect(() => {
    if (!tableWrapEl) return;
    const ro = new ResizeObserver((entries) => {
      for (const entry of entries) {
        viewportHeight = entry.contentRect.height;
      }
    });
    ro.observe(tableWrapEl);
    return () => ro.disconnect();
  });
</script>

<svelte:window
  onkeydown={handleKeydown}
  onclick={() => {
    rowMenu = null;
    actionsMenuOpen = null;
    columnPickerOpen = false;
  }}
/>

<div class="modal-backdrop browser-backdrop" role="dialog" aria-modal="true">
  <div class="modal-window browser-window">
    <!-- Top Browser Header & Action Bar -->
    <div class="browser-topbar">
      <div class="browser-menus">
        <span class="browser-title">
          Browse ({resultIds.length} {notesMode ? "notes" : "cards"}{selectedIds.size > 0
            ? `, ${selectedIds.size} selected`
            : ""})
        </span>

        <div class="menu-wrap">
          <button
            class="mini-btn"
            onclick={(e) => {
              e.stopPropagation();
              actionsMenuOpen = actionsMenuOpen === "notes" ? null : "notes";
            }}
          >
            Notes ▾
          </button>
          {#if actionsMenuOpen === "notes"}
            <div class="context-menu dropdown" onclick={(e) => e.stopPropagation()} role="menu">
              <button class="context-menu-item" onclick={bulkAddTags}>
                <span>Add Tags...</span><span class="shortcut">Ctrl+Shift+A</span>
              </button>
              <button class="context-menu-item" onclick={bulkRemoveTags}>
                <span>Remove Tags...</span>
              </button>
              <button class="context-menu-item" onclick={bulkClearUnusedTags}>
                <span>Clear Unused Tags</span>
              </button>
              <div class="context-menu-sep"></div>
              <button class="context-menu-item" onclick={bulkToggleMark}>
                <span>Toggle Mark</span><span class="shortcut">Ctrl+K</span>
              </button>
              <button class="context-menu-item" onclick={bulkChangeNotetype}>
                <span>Change Note Type...</span>
              </button>
              <button class="context-menu-item" onclick={openFindReplace}>
                <span>Find and Replace...</span><span class="shortcut">Ctrl+Alt+F</span>
              </button>
              <button
                class="context-menu-item"
                onclick={() => {
                  actionsMenuOpen = null;
                  findDupesOpen = true;
                }}
              >
                <span>Find Duplicates...</span>
              </button>
              <button class="context-menu-item" onclick={openExportSelected}>
                <span>Export Notes...</span>
              </button>
              <div class="context-menu-sep"></div>
              <button class="context-menu-item" onclick={bulkDelete}>
                <span style="color: var(--accent-danger)">Delete</span>
                <span class="shortcut">Del</span>
              </button>
            </div>
          {/if}
        </div>

        <div class="menu-wrap">
          <button
            class="mini-btn"
            onclick={(e) => {
              e.stopPropagation();
              actionsMenuOpen = actionsMenuOpen === "cards" ? null : "cards";
            }}
          >
            Cards ▾
          </button>
          {#if actionsMenuOpen === "cards"}
            <div class="context-menu dropdown" onclick={(e) => e.stopPropagation()} role="menu">
              <div class="ctx-flags" style="padding: 6px 12px">
                <span>Flag:</span>
                <button class="flag-chip" title="Clear Flag" onclick={() => bulkSetFlag(0)}>×</button>
                {#each FLAG_LABELS as fl}
                  <button
                    class="flag-chip"
                    style="background: {fl.color}"
                    title={fl.name}
                    onclick={() => bulkSetFlag(fl.idx)}
                  ></button>
                {/each}
              </div>
              <div class="context-menu-sep"></div>
              <button
                class="context-menu-item"
                onclick={() => {
                  actionsMenuOpen = null;
                  if (selectedIds.size > 0) changeDeckOpen = true;
                }}
              >
                <span>Change Deck...</span><span class="shortcut">Ctrl+D</span>
              </button>
              <button class="context-menu-item" onclick={bulkToggleSuspend}>
                <span>Toggle Suspend</span><span class="shortcut">Ctrl+J</span>
              </button>
              <button class="context-menu-item" onclick={bulkForget}>
                <span>Forget...</span>
              </button>
              <button class="context-menu-item" onclick={bulkSetDueDate}>
                <span>Set Due Date...</span><span class="shortcut">Ctrl+Shift+D</span>
              </button>
              <button class="context-menu-item" onclick={bulkReposition}>
                <span>Reposition...</span>
              </button>
              <div class="context-menu-sep"></div>
              <button
                class="context-menu-item"
                onclick={() => bulkGradeNow(CardAnswer_Rating.GOOD, "Good")}
              >
                <span>Grade Now: Good</span>
              </button>
              <button
                class="context-menu-item"
                onclick={() => bulkGradeNow(CardAnswer_Rating.AGAIN, "Again")}
              >
                <span>Grade Now: Again</span>
              </button>
              <div class="context-menu-sep"></div>
              <button class="context-menu-item" onclick={openCardInfo}>
                <span>Info...</span>
              </button>
            </div>
          {/if}
        </div>

        <button class="mini-btn" onclick={() => (sidebarVisible = !sidebarVisible)}>
          {sidebarVisible ? "Hide Sidebar" : "Show Sidebar"}
        </button>
        <button class="mini-btn" onclick={() => (verticalSplit = !verticalSplit)}>
          {verticalSplit ? "Horizontal Split" : "Vertical Split"}
        </button>
      </div>

      <button class="modal-close" onclick={closeBrowser} title="Close Browser (Esc)">×</button>
    </div>

    <!-- Main 3-Pane Body -->
    <div class="browser-body">
      {#if sidebarVisible}
        <aside class="sidebar">
          <div class="sidebar-group">
            <button
              class="sb-heading"
              onclick={() => (openSections.saved = !openSections.saved)}
            >
              {openSections.saved ? "▾" : "▸"} Saved Searches
            </button>
            {#if openSections.saved}
              {#each Object.entries(savedSearches) as [name, query]}
                <div class="sb-item-row">
                  <button
                    class="sb-item"
                    onclick={() => {
                      searchText = query;
                      void runSearch(query);
                    }}
                  >
                    <Icon name="star" size={13} /> {name}
                  </button>
                  <button
                    class="sb-del"
                    title="Remove saved search"
                    onclick={(e) => deleteSavedSearch(name, e)}
                  >
                    ×
                  </button>
                </div>
              {:else}
                <div class="sb-empty">None saved</div>
              {/each}
            {/if}
          </div>

          <div class="sidebar-group">
            <button
              class="sb-heading"
              onclick={() => (openSections.today = !openSections.today)}
            >
              {openSections.today ? "▾" : "▸"} Today
            </button>
            {#if openSections.today}
              <button
                class="sb-item"
                onclick={(e) =>
                  applySidebarFilter(new SearchNode({ filter: { case: "addedInDays", value: 1 } }), e)}
              >
                Added
              </button>
              <button
                class="sb-item"
                onclick={(e) =>
                  applySidebarFilter(
                    new SearchNode({
                      filter: { case: "rated", value: { days: 1, rating: SearchNode_Rating.ANY } },
                    }),
                    e
                  )}
              >
                Studied
              </button>
              <button
                class="sb-item"
                onclick={(e) =>
                  applySidebarFilter(
                    new SearchNode({
                      filter: { case: "rated", value: { days: 1, rating: SearchNode_Rating.AGAIN } },
                    }),
                    e
                  )}
              >
                Again
              </button>
              <button
                class="sb-item"
                onclick={(e) =>
                  applySidebarFilter(new SearchNode({ filter: { case: "editedInDays", value: 1 } }), e)}
              >
                Edited
              </button>
            {/if}
          </div>

          <div class="sidebar-group">
            <button
              class="sb-heading"
              onclick={() => (openSections.flags = !openSections.flags)}
            >
              {openSections.flags ? "▾" : "▸"} Flags
            </button>
            {#if openSections.flags}
              {#each FLAG_LABELS as fl}
                <button
                  class="sb-item"
                  onclick={(e) =>
                    applySidebarFilter(
                      new SearchNode({ filter: { case: "flag", value: fl.enumVal } }),
                      e
                    )}
                >
                  <span class="sb-dot" style="background: {fl.color}"></span>
                  {fl.name}
                </button>
              {/each}
            {/if}
          </div>

          <div class="sidebar-group">
            <button
              class="sb-heading"
              onclick={() => (openSections.states = !openSections.states)}
            >
              {openSections.states ? "▾" : "▸"} Card State
            </button>
            {#if openSections.states}
              {#each [
                { label: "New", state: SearchNode_CardState.NEW },
                { label: "Learning", state: SearchNode_CardState.LEARN },
                { label: "Review", state: SearchNode_CardState.REVIEW },
                { label: "Due", state: SearchNode_CardState.DUE },
                { label: "Suspended", state: SearchNode_CardState.SUSPENDED },
                { label: "Buried", state: SearchNode_CardState.BURIED },
              ] as st}
                <button
                  class="sb-item"
                  onclick={(e) =>
                    applySidebarFilter(
                      new SearchNode({ filter: { case: "cardState", value: st.state } }),
                      e
                    )}
                >
                  {st.label}
                </button>
              {/each}
            {/if}
          </div>

          <div class="sidebar-group">
            <button
              class="sb-heading"
              onclick={() => (openSections.decks = !openSections.decks)}
            >
              {openSections.decks ? "▾" : "▸"} Decks
            </button>
            {#if openSections.decks}
              {#each flattenDecks(decks) as { node, fullName }}
                <button
                  class="sb-item"
                  style="padding-left: {12 + (node.level - 1) * 14}px"
                  onclick={(e) =>
                    applySidebarFilter(new SearchNode({ filter: { case: "deck", value: fullName } }), e)}
                >
                  {fullName.split("::").pop()}
                </button>
              {/each}
            {/if}
          </div>

          <div class="sidebar-group">
            <button
              class="sb-heading"
              onclick={() => (openSections.notetypes = !openSections.notetypes)}
            >
              {openSections.notetypes ? "▾" : "▸"} Note Types
            </button>
            {#if openSections.notetypes}
              {#each notetypes as nt}
                <button
                  class="sb-item"
                  onclick={(e) =>
                    applySidebarFilter(new SearchNode({ filter: { case: "note", value: nt.name } }), e)}
                >
                  {nt.name}
                </button>
              {/each}
            {/if}
          </div>

          <div class="sidebar-group">
            <button
              class="sb-heading"
              onclick={() => (openSections.tags = !openSections.tags)}
            >
              {openSections.tags ? "▾" : "▸"} Tags
            </button>
            {#if openSections.tags}
              {#each flattenTags(tags) as t}
                <button
                  class="sb-item"
                  style="padding-left: {12 + Math.max(0, t.level - 1) * 14}px"
                  onclick={(e) =>
                    applySidebarFilter(new SearchNode({ filter: { case: "tag", value: t.fullTag } }), e)}
                >
                  #{t.name}
                </button>
              {:else}
                <div class="sb-empty">No tags</div>
              {/each}
            {/if}
          </div>
        </aside>
      {/if}

      <div class="right-area" class:vertical={verticalSplit}>
        <!-- Search + Table Pane -->
        <section class="table-pane">
          <form
            class="search-bar"
            onsubmit={(e) => {
              e.preventDefault();
              void runSearch();
            }}
          >
            <button
              type="button"
              class="mode-toggle"
              class:notes={notesMode}
              title="Toggle between Cards and Notes mode"
              onclick={() => {
                notesMode = !notesMode;
                selectedIds = new Set();
                void runSearch();
              }}
            >
              {notesMode ? "Notes" : "Cards"}
            </button>
            <input
              class="anki-input search-input"
              type="text"
              bind:value={searchText}
              list="cardly-search-history"
              placeholder='Search cards/notes (e.g. deck:Default tag:vocab is:due)'
            />
            <datalist id="cardly-search-history">
              {#each searchHistory as h}
                <option value={h}></option>
              {/each}
            </datalist>
            <button type="submit" class="anki-btn small">Search</button>
            <button type="button" class="anki-btn small" title="Save current search" aria-label="Save current search" onclick={handleSaveSearch}>
              <Icon name="star" size={14} />
            </button>
            <div class="menu-wrap">
              <button
                type="button"
                class="anki-btn small"
                onclick={(e) => {
                  e.stopPropagation();
                  columnPickerOpen = !columnPickerOpen;
                }}
              >
                Columns ▾
              </button>
              {#if columnPickerOpen}
                <div
                  class="context-menu col-picker"
                  onclick={(e) => e.stopPropagation()}
                  role="menu"
                >
                  {#each allColumns as col}
                    <label class="col-opt">
                      <input
                        type="checkbox"
                        checked={activeColumnKeys.includes(col.key)}
                        onchange={() => toggleColumn(col.key)}
                      />
                      <span>{notesMode ? col.notesModeLabel : col.cardsModeLabel}</span>
                    </label>
                  {/each}
                </div>
              {/if}
            </div>
            <button type="button" class="anki-btn small" onclick={openPreview}>Preview</button>
          </form>

          {#if searchError}
            <div class="search-err">{searchError}</div>
          {/if}

          <!-- Column Headers -->
          <div class="grid-header" style="grid-template-columns: repeat({visibleColumns.length}, minmax(90px, 1fr))">
            {#each visibleColumns as col}
              <button class="th-btn" onclick={() => toggleSort(col.key)}>
                <span>{notesMode ? col.notesModeLabel : col.cardsModeLabel}</span>
                {#if sortColumn === col.key}
                  <span class="sort-arrow">{sortReverse ? "▼" : "▲"}</span>
                {/if}
              </button>
            {/each}
          </div>

          <!-- Virtualized Rows Container -->
          <div
            class="grid-scroll"
            bind:this={tableWrapEl}
            onscroll={(e) => {
              scrollTop = (e.currentTarget as HTMLDivElement).scrollTop;
            }}
          >
            {#if searching}
              <div class="empty-table">Searching...</div>
            {:else if resultIds.length === 0}
              <div class="empty-table">No matching {notesMode ? "notes" : "cards"}.</div>
            {:else}
              <div class="virtual-spacer" style="height: {totalHeight}px">
                {#each visibleSlice as item (item.id)}
                  <!-- svelte-ignore a11y_click_events_have_key_events -->
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <div
                    class="grid-row {item.row ? rowColorClass(item.row.color) : ''}"
                    class:selected={selectedIds.has(item.id)}
                    style="top: {item.index * ROW_HEIGHT}px; height: {ROW_HEIGHT}px; grid-template-columns: repeat({visibleColumns.length}, minmax(90px, 1fr))"
                    onclick={(e) => handleRowClick(item.id, item.index, e)}
                    oncontextmenu={(e) => handleRowContextMenu(item.id, item.index, e)}
                  >
                    {#if item.row}
                      {#each item.row.cells as cell}
                        <div class="grid-cell" dir={cell.is_rtl ? "rtl" : "auto"}>{cell.text}</div>
                      {/each}
                    {:else}
                      <div class="grid-cell muted">…</div>
                    {/if}
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        </section>

        <!-- Embedded Upstream Note Editor Pane -->
        <section class="editor-pane">
          <UpstreamEditor
            mode="browser"
            noteId={activeNoteId}
            reviewerCardId={activeCardId}
            onSaved={() => {
              rowCache = new Map();
              void ensureVisibleRowsLoaded();
            }}
          />
        </section>
      </div>
    </div>
  </div>
</div>

<!-- Row Context Menu -->
{#if rowMenu}
  <div
    class="context-menu"
    style="left: {rowMenu.x}px; top: {rowMenu.y}px"
    onclick={(e) => e.stopPropagation()}
    role="menu"
  >
    <button
      class="context-menu-item"
      onclick={() => {
        rowMenu = null;
        changeDeckOpen = true;
      }}
    >
      <span>Change Deck...</span><span class="shortcut">Ctrl+D</span>
    </button>
    <button class="context-menu-item" onclick={bulkToggleSuspend}>
      <span>Toggle Suspend</span><span class="shortcut">Ctrl+J</span>
    </button>
    <button class="context-menu-item" onclick={bulkToggleMark}>
      <span>Toggle Mark</span><span class="shortcut">Ctrl+K</span>
    </button>
    <div class="context-menu-sep"></div>
    <div class="ctx-flags">
      <span>Flag:</span>
      <button class="flag-chip" onclick={() => bulkSetFlag(0)}>×</button>
      {#each FLAG_LABELS as fl}
        <button
          class="flag-chip"
          style="background: {fl.color}"
          title={fl.name}
          onclick={() => bulkSetFlag(fl.idx)}
        ></button>
      {/each}
    </div>
    <div class="context-menu-sep"></div>
    <button class="context-menu-item" onclick={bulkAddTags}><span>Add Tags...</span></button>
    <button class="context-menu-item" onclick={bulkRemoveTags}><span>Remove Tags...</span></button>
    <button class="context-menu-item" onclick={bulkChangeNotetype}><span>Change Note Type...</span></button>
    <button class="context-menu-item" onclick={openFindReplace}><span>Find and Replace...</span></button>
    <div class="context-menu-sep"></div>
    <button class="context-menu-item" onclick={bulkSetDueDate}><span>Set Due Date...</span></button>
    <button class="context-menu-item" onclick={bulkForget}><span>Forget...</span></button>
    <button class="context-menu-item" onclick={bulkReposition}><span>Reposition...</span></button>
    <button class="context-menu-item" onclick={openCardInfo}><span>Card Info...</span></button>
    <button class="context-menu-item" onclick={openExportSelected}><span>Export Notes...</span></button>
    <div class="context-menu-sep"></div>
    <button class="context-menu-item" onclick={bulkDelete}>
      <span style="color: var(--accent-danger)">Delete</span>
    </button>
  </div>
{/if}

<!-- Change Deck Sub-modal -->
{#if changeDeckOpen}
  <div class="modal-backdrop" style="z-index: 1600" role="dialog" aria-modal="true">
    <div class="modal-window" style="width: 380px">
      <div class="modal-header">
        <h3 class="modal-title">Change Deck</h3>
        <button class="modal-close" onclick={() => (changeDeckOpen = false)}>×</button>
      </div>
      <div class="modal-body">
        <label style="display:flex; flex-direction:column; gap:8px; font-size:13.5px;">
          <span>Move {selectedIds.size} selected item(s) to deck:</span>
          <select class="anki-select" bind:value={changeDeckTargetId}>
            {#each flatDeckNames as d}
              <option value={Number(d.id)}>{d.name}</option>
            {/each}
          </select>
        </label>
      </div>
      <div class="modal-footer">
        <button class="anki-btn" onclick={() => (changeDeckOpen = false)}>Cancel</button>
        <button class="anki-btn primary" onclick={bulkChangeDeckConfirm}>Move Cards</button>
      </div>
    </div>
  </div>
{/if}

{#if findReplaceOpen}
  <FindAndReplaceModal
    noteIds={findReplaceNoteIds}
    onClose={() => (findReplaceOpen = false)}
    onDone={() => void runSearch()}
  />
{/if}

{#if findDupesOpen}
  <FindDuplicatesModal
    onClose={() => (findDupesOpen = false)}
    onOpenSearch={(q) => {
      searchText = q;
      void runSearch(q);
    }}
  />
{/if}

<style>
  .browser-backdrop {
    padding: 12px;
  }
  .browser-window {
    width: 96vw;
    height: 92vh;
    max-width: 1500px;
  }
  .browser-topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 12px;
    background: var(--canvas-elevated);
    border-bottom: 1px solid var(--border-subtle);
    gap: 10px;
    flex-shrink: 0;
  }
  .browser-menus {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
  .browser-title {
    font-weight: 600;
    font-size: 13.5px;
    margin-right: 8px;
  }
  .mini-btn {
    border: 1px solid var(--border-subtle);
    background: var(--button-bg);
    color: var(--fg);
    border-radius: 5px;
    padding: 3px 9px;
    font-size: 12.5px;
    font-family: inherit;
    cursor: pointer;
  }
  .mini-btn:hover {
    border-color: var(--border);
  }
  .menu-wrap {
    position: relative;
  }
  .dropdown {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    z-index: 2100;
  }
  .col-picker {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    max-height: 320px;
    overflow: auto;
    min-width: 200px;
    z-index: 2100;
  }
  .col-opt {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    font-size: 12.5px;
    cursor: pointer;
  }
  .col-opt:hover {
    background: var(--border-subtle);
    border-radius: 4px;
  }
  .browser-body {
    flex: 1;
    display: flex;
    min-height: 0;
    overflow: hidden;
  }
  .sidebar {
    width: 225px;
    flex-shrink: 0;
    overflow: auto;
    border-right: 1px solid var(--border-subtle);
    background: var(--canvas);
    padding: 6px 4px;
    user-select: none;
  }
  .sidebar-group {
    margin-bottom: 6px;
  }
  .sb-heading {
    width: 100%;
    text-align: left;
    border: none;
    background: transparent;
    color: var(--fg-subtle);
    font-size: 12px;
    font-weight: 700;
    padding: 4px 6px;
    cursor: pointer;
    font-family: inherit;
  }
  .sb-item-row {
    display: flex;
    align-items: center;
  }
  .sb-item {
    width: 100%;
    text-align: left;
    border: none;
    background: transparent;
    color: var(--fg);
    font-size: 12.5px;
    padding: 4px 12px;
    border-radius: 4px;
    cursor: pointer;
    font-family: inherit;
    display: flex;
    align-items: center;
    gap: 6px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sb-item:hover {
    background: var(--border-subtle);
  }
  .sb-del {
    border: none;
    background: transparent;
    color: var(--fg-subtle);
    cursor: pointer;
    padding: 2px 6px;
  }
  .sb-empty {
    font-size: 12px;
    color: var(--fg-faint);
    padding: 2px 14px;
  }
  .sb-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .right-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }
  .right-area.vertical {
    flex-direction: row;
  }
  .table-pane {
    flex: 1.15;
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
    border-bottom: 1px solid var(--border);
  }
  .right-area.vertical .table-pane {
    border-bottom: none;
    border-right: 1px solid var(--border);
  }
  .editor-pane {
    flex: 1;
    display: flex;
    min-height: 0;
    min-width: 0;
  }
  .search-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 10px;
    background: var(--canvas-elevated);
    border-bottom: 1px solid var(--border-subtle);
    flex-shrink: 0;
  }
  .mode-toggle {
    border: 1px solid var(--accent-card);
    background: rgba(96, 165, 250, 0.16);
    color: var(--fg);
    border-radius: 5px;
    padding: 4px 10px;
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
    font-family: inherit;
  }
  .mode-toggle.notes {
    border-color: var(--accent-note);
    background: rgba(34, 197, 94, 0.16);
  }
  .search-input {
    flex: 1;
    min-width: 120px;
  }
  .search-err {
    padding: 6px 12px;
    font-size: 12.5px;
    color: var(--accent-danger);
    background: rgba(239, 68, 68, 0.1);
  }
  .grid-header {
    display: grid;
    background: var(--canvas-elevated);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    user-select: none;
  }
  .th-btn {
    border: none;
    border-right: 1px solid var(--border-subtle);
    background: transparent;
    color: var(--fg);
    font-size: 12px;
    font-weight: 600;
    padding: 5px 8px;
    text-align: left;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-family: inherit;
    overflow: hidden;
  }
  .th-btn:hover {
    background: var(--border-subtle);
  }
  .sort-arrow {
    font-size: 10px;
    color: var(--fg-subtle);
  }
  .grid-scroll {
    flex: 1;
    overflow: auto;
    position: relative;
    background: var(--canvas-inset);
    user-select: none;
  }
  .virtual-spacer {
    position: relative;
    width: 100%;
  }
  .grid-row {
    position: absolute;
    left: 0;
    right: 0;
    display: grid;
    align-items: center;
    border-bottom: 1px solid var(--border-subtle);
    font-size: 12.5px;
    cursor: default;
  }
  .grid-row:hover {
    background: rgba(128, 128, 128, 0.08);
  }
  .grid-row.row-marked {
    background: rgba(99, 102, 241, 0.18);
  }
  .grid-row.row-suspended {
    background: rgba(250, 204, 21, 0.22);
  }
  .grid-row.row-buried {
    background: rgba(245, 158, 11, 0.18);
  }
  .grid-row.row-flag-1 {
    background: rgba(239, 68, 68, 0.18);
  }
  .grid-row.row-flag-2 {
    background: rgba(251, 146, 60, 0.2);
  }
  .grid-row.row-flag-3 {
    background: rgba(74, 222, 128, 0.18);
  }
  .grid-row.row-flag-4 {
    background: rgba(59, 130, 246, 0.18);
  }
  .grid-row.row-flag-5 {
    background: rgba(232, 121, 249, 0.18);
  }
  .grid-row.row-flag-6 {
    background: rgba(45, 212, 191, 0.18);
  }
  .grid-row.row-flag-7 {
    background: rgba(168, 85, 247, 0.18);
  }
  .grid-row.selected {
    background: var(--highlight-bg) !important;
    color: var(--highlight-fg);
  }
  .grid-cell {
    padding: 0 8px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .grid-cell.muted {
    color: var(--fg-faint);
  }
  .empty-table {
    padding: 28px;
    text-align: center;
    color: var(--fg-subtle);
    font-size: 13.5px;
  }
  .ctx-flags {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 4px 8px;
    font-size: 12px;
    color: var(--fg-subtle);
  }
  .flag-chip {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 1px solid var(--border);
    background: var(--canvas-inset);
    color: var(--fg);
    cursor: pointer;
    padding: 0;
    font-size: 10px;
  }
  @media (max-width: 768px) {
    .browser-window {
      width: 100%;
      max-width: 100%;
      height: 100dvh;
      max-height: 100dvh;
    }
    .browser-topbar {
      padding: calc(6px + var(--safe-top)) calc(10px + var(--safe-right)) 6px calc(10px + var(--safe-left));
    }
    .browser-body {
      min-width: 0;
      max-width: 100%;
      overflow: hidden;
    }
    .sidebar {
      position: absolute;
      top: calc(44px + var(--safe-top));
      bottom: 0;
      left: 0;
      width: min(280px, 85vw);
      background: var(--canvas-overlay);
      box-shadow: 4px 0 16px rgba(0, 0, 0, 0.25);
      z-index: 50;
    }
    .right-area {
      flex-direction: column !important;
      min-width: 0;
      max-width: 100%;
    }
    .table-pane {
      flex: 1;
      min-height: 220px;
      min-width: 0;
      max-width: 100%;
      overflow: auto;
    }
    .editor-pane {
      flex: 1;
      min-height: 220px;
      min-width: 0;
      max-width: 100%;
    }
    .search-bar {
      padding: 6px 8px;
      flex-wrap: wrap;
      min-width: 0;
    }
    .search-input {
      order: 3;
      width: 100%;
      min-width: 0;
    }
  }
</style>
