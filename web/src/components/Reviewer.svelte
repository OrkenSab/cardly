<script lang="ts">
  import {
    addNoteTags,
    answerCard,
    buryOrSuspendCards,
    compareAnswer,
    describeNextStates,
    extractAvTags,
    extractClozeForTyping,
    getDeck,
    getDeckConfig,
    getNote,
    getNotetypeLegacy,
    getPreferences,
    getQueuedCards,
    removeNoteTags,
    removeNotes,
    renderExistingCard,
    scheduleCardsAsNew,
    setDueDate,
    setFlag as setCardFlagRpc,
    stateIsLeech,
  } from "@generated/backend";
  import type { AVTag, RenderCardResponse } from "@generated/anki/card_rendering_pb";
  import type { Card } from "@generated/anki/cards_pb";
  import type { DeckConfig_Config } from "@generated/anki/deck_config_pb";
  import type { Note } from "@generated/anki/notes_pb";
  import {
    BuryOrSuspendCardsRequest_Mode,
    CardAnswer_Rating,
    QueuedCards_Queue,
  } from "@generated/anki/scheduler_pb";
  import type { QueuedCards_QueuedCard, SchedulingStates } from "@generated/anki/scheduler_pb";
  import { performUndo } from "../lib/api";
  import { PluginSlot, pluginManager } from "../lib/plugins";
  import { nodesToHtml } from "../lib/renderCard";
  import { store } from "../lib/store.svelte";
  import Icon from "./Icon.svelte";

  const FLAG_NAMES = ["No Flag", "Red", "Orange", "Green", "Blue", "Pink", "Turquoise", "Purple"];
  const FLAG_COLORS = [
    "transparent",
    "var(--flag-1)",
    "var(--flag-2)",
    "var(--flag-3)",
    "var(--flag-4)",
    "var(--flag-5)",
    "var(--flag-6)",
    "var(--flag-7)",
  ];

  // Reviewer state
  let phase = $state<"loading" | "question" | "answer" | "transition">("loading");
  let topQueued = $state<QueuedCards_QueuedCard | null>(null);
  let card = $state<Card | null>(null);
  let previousCardId = $state<number>(0);
  let note = $state<Note | null>(null);
  let rendered = $state<RenderCardResponse | null>(null);
  let questionHtml = $state<string>("");
  let answerHtml = $state<string>("");
  let questionAvTags = $state<AVTag[]>([]);
  let answerAvTags = $state<AVTag[]>([]);
  let deckConf = $state<DeckConfig_Config | null>(null);
  let newCount = $state(0);
  let learnCount = $state(0);
  let reviewCount = $state(0);
  let activeQueueIdx = $state(0); // 0=new, 1=learn, 2=review
  let intervalLabels = $state<string[]>(["", "", "", ""]);

  // Preferences
  let showIntervals = $state(true);
  let showRemainingCounts = $state(true);
  let hidePlayButtons = $state(false);
  let interruptAudioOnAnswer = $state(true);

  // Typed answer state
  let typeCorrect = $state<string | null>(null);
  let typeFont = $state("Arial");
  let typeSize = $state(20);
  let typeCombining = $state(true);
  let typeIgnoreCase = $state(false);
  let typedAnswer = $state("");

  // Timer & Auto-Advance
  let cardStartMs = 0;
  let elapsedSecs = $state(0);
  let maxTimeSecs = $state(0);
  let showTimer = $state(false);
  let timerStopped = $state(false);
  let timerHandle: number | null = null;
  let autoAdvanceEnabled = $state(false);
  let autoAdvanceTimer: number | null = null;

  // Audio playback queue
  let audioQueue: AVTag[] = [];
  let currentAudioEl: HTMLAudioElement | null = null;
  let audioPaused = false;

  // More menu
  let moreMenuOpen = $state(false);
  let flagSubmenuOpen = $state(false);

  // Card iframe
  let iframeEl = $state<HTMLIFrameElement | null>(null);
  let iframeReady = $state(false);
  let pendingRender: (() => void) | null = null;

  // Build isolated reviewer document (uses real upstream `/_anki/css/reviewer.css` & `/_anki/js/reviewer.js` + MathJax 3)
  const reviewerSrcDoc = `<!doctype html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<base href="${window.location.origin}/">
<link rel="stylesheet" href="/_anki/css/webview.css">
<link rel="stylesheet" href="/_anki/css/reviewer.css">
<style>
  html, body { margin: 0; padding: 16px 20px; min-height: 100%; box-sizing: border-box; overflow-x: hidden; word-wrap: break-word; -webkit-user-select: text; user-select: text; }
  .card { margin: 0 auto; max-width: 900px; }
  .card img, .card video, .card svg { max-width: 100%; height: auto; }
  .card table { max-width: 100%; display: block; overflow-x: auto; }
  .card pre, .card code { max-width: 100%; overflow-x: auto; }
  @media (max-width: 640px) { html, body { padding: 12px 14px; } #typeans { max-width: 100%; font-size: 16px !important; } }
  .replay-button { display: inline-flex; align-items: center; justify-content: center; cursor: pointer; vertical-align: middle; margin: 0 3px; text-decoration: none; }
  .replay-button svg { width: 32px; height: 32px; }
  .typeGood { background: #0f0; color: #000; }
  .typeBad { background: #f00; color: #fff; }
  .typeMissed { background: #ccc; color: #000; }
  .nightMode .typeGood { background: #16a34a; color: #fff; }
  .nightMode .typeBad { background: #dc2626; color: #fff; }
  .nightMode .typeMissed { background: #52525b; color: #fff; }
</style>
<script>
  var PARENT_ORIGIN = '${window.location.origin}';
  window.bridgeCommand = function(cmd, cb) {
    try {
      window.parent.postMessage({ type: 'cardly:reviewer-cmd', cmd: cmd }, PARENT_ORIGIN);
    } catch (_) {}
    if (typeof cb === 'function') cb();
  };
  window.pycmd = window.bridgeCommand;
  window.addEventListener('keydown', function(e) {
    var tag = e.target && e.target.tagName;
    if (tag === 'INPUT' || tag === 'TEXTAREA' || (e.target && e.target.isContentEditable)) {
      if (e.key === 'Enter' && e.target.id === 'typeans') {
        e.preventDefault();
        window.parent.postMessage({ type: 'cardly:reviewer-cmd', cmd: 'ans' }, PARENT_ORIGIN);
      }
      return;
    }
    window.parent.postMessage({
      type: 'cardly:reviewer-key',
      key: e.key,
      code: e.code,
      ctrlKey: e.ctrlKey,
      metaKey: e.metaKey,
      altKey: e.altKey,
      shiftKey: e.shiftKey
    }, PARENT_ORIGIN);
  });
<\/script>
<script src="/_anki/js/mathjax.js"><\/script>
<script src="/_anki/js/vendor/mathjax/tex-chtml-full.js"><\/script>
<script src="/_anki/js/reviewer.js"><\/script>
</head>
<body class="card">
<div id="_mark" hidden>&#x2605;</div>
<div id="_flag" hidden>&#x2691;</div>
<div id="qa" dir="auto"></div>
</body>
</html>`;

  function syncIframeTheme() {
    const doc = iframeEl?.contentDocument;
    if (!doc) return;
    const dark = store.isDark;
    doc.documentElement.classList.toggle("night-mode", dark);
    doc.documentElement.classList.toggle("nightMode", dark);
    doc.documentElement.dataset.bsTheme = dark ? "dark" : "light";
    doc.documentElement.setAttribute("data-bs-theme", dark ? "dark" : "light");
    doc.documentElement.style.colorScheme = dark ? "dark" : "light";
    if (doc.body) {
      doc.body.classList.toggle("night-mode", dark);
      doc.body.classList.toggle("nightMode", dark);
      doc.body.style.backgroundColor = dark ? "var(--canvas, #2c2c2c)" : "var(--canvas, #f5f5f5)";
      doc.body.style.color = dark ? "var(--fg, #fcfcfc)" : "var(--fg, #020202)";
    }
  }

  function handleIframeLoad() {
    iframeReady = true;
    syncIframeTheme();
    if (pendingRender) {
      const fn = pendingRender;
      pendingRender = null;
      fn();
    }
  }

  function runInCardFrame(fn: (win: any) => void) {
    if (iframeReady && iframeEl?.contentWindow) {
      fn(iframeEl.contentWindow as any);
    } else {
      pendingRender = () => {
        if (iframeEl?.contentWindow) fn(iframeEl.contentWindow as any);
      };
    }
  }

  // ============================================================================
  // Audio / Video / TTS Engine
  // ============================================================================

  function stopAudio() {
    audioQueue = [];
    audioPaused = false;
    if (currentAudioEl) {
      currentAudioEl.pause();
      currentAudioEl.src = "";
      currentAudioEl = null;
    }
    try {
      window.speechSynthesis?.cancel();
    } catch {
      // ignore
    }
  }

  function playNextTag() {
    if (audioQueue.length === 0) return;
    const tag = audioQueue.shift()!;
    if (tag.value.case === "soundOrVideo") {
      const filename = tag.value.value;
      const audio = new Audio(`/${encodeURIComponent(filename)}`);
      currentAudioEl = audio;
      audioPaused = false;
      audio.onended = () => {
        currentAudioEl = null;
        playNextTag();
      };
      audio.onerror = () => {
        currentAudioEl = null;
        playNextTag();
      };
      audio.play().catch(() => {
        currentAudioEl = null;
      });
    } else if (tag.value.case === "tts") {
      const tts = tag.value.value;
      if ("speechSynthesis" in window && tts.fieldText) {
        const utter = new SpeechSynthesisUtterance(tts.fieldText);
        if (tts.lang) utter.lang = tts.lang.replace("_", "-");
        if (tts.speed && tts.speed > 0) utter.rate = tts.speed;
        utter.onend = () => playNextTag();
        utter.onerror = () => playNextTag();
        window.speechSynthesis.speak(utter);
      } else {
        playNextTag();
      }
    }
  }

  function playTags(tags: AVTag[]) {
    stopAudio();
    if (!tags || tags.length === 0) return;
    audioQueue = [...tags];
    playNextTag();
  }

  function replayAudio() {
    if (!rendered) return;
    if (phase === "question") {
      playTags(questionAvTags);
    } else if (phase === "answer") {
      const skipQ = deckConf?.skipQuestionWhenReplayingAnswer === true;
      const tags = skipQ
        ? answerAvTags
        : [...questionAvTags, ...answerAvTags];
      playTags(tags);
    }
  }

  function togglePauseAudio() {
    if (!currentAudioEl) return;
    if (audioPaused) {
      void currentAudioEl.play();
      audioPaused = false;
    } else {
      currentAudioEl.pause();
      audioPaused = true;
    }
  }

  function seekAudio(deltaSecs: number) {
    if (!currentAudioEl) return;
    currentAudioEl.currentTime = Math.max(0, currentAudioEl.currentTime + deltaSecs);
  }

  function playClickedAv(cmd: string) {
    // cmd format: `play:q:0` or `play:a:1`
    if (!rendered) return;
    const [, side, idxStr] = cmd.split(":");
    const idx = parseInt(idxStr, 10);
    const list = side === "q" ? questionAvTags : answerAvTags;
    const tag = list[idx];
    if (tag) playTags([tag]);
  }

  // Replace Anki `[anki:play:q:0]` placeholders with clickable replay buttons (matching `prepare_card_text_for_display`)
  const PLAY_BTN_SVG = `<svg viewBox="0 0 64 64" aria-hidden="true"><circle cx="32" cy="32" r="29" fill="none" stroke="currentColor" stroke-width="4"/><polygon points="25,19 47,32 25,45" fill="currentColor"/></svg>`;

  function replaceAvPlaceholders(html: string): string {
    return html.replace(/\[anki:play:([qa]):(\d+)\]/g, (_m, side, idx) => {
      if (hidePlayButtons) return "";
      return `<a class="replay-button soundLink" href="javascript:void(0)" onclick="pycmd('play:${side}:${idx}'); return false;" title="Replay audio">${PLAY_BTN_SVG}</a>`;
    });
  }

  // ============================================================================
  // Typed Answer Filter (faithful port of qt/aqt/reviewer.py)
  // ============================================================================

  const TYPE_ANS_RE = /\[\[type:(.+?)\]\]/;

  async function filterTypeAnswerQuestion(html: string): Promise<string> {
    typeCorrect = null;
    typeCombining = true;
    typeIgnoreCase = false;
    const match = html.match(TYPE_ANS_RE);
    if (!match || !card || !note) return html;

    let fld = match[1];
    let clozeIdx: number | null = null;
    const lastColon = fld.lastIndexOf(":");
    if (lastColon !== -1) {
      const modifiers = fld.slice(0, lastColon).split(":");
      fld = fld.slice(lastColon + 1);
      for (const mod of modifiers) {
        if (mod === "cloze") clozeIdx = card.templateIdx + 1;
        else if (mod === "nc") typeCombining = false;
        else if (mod === "ci") typeIgnoreCase = true;
      }
    }

    try {
      const ntBytes = (await getNotetypeLegacy({ ntid: note.notetypeId }, { alertOnError: false })).json;
      const nt = JSON.parse(new TextDecoder().decode(ntBytes));
      const flds: Array<{ name: string; font: string; size: number }> = nt.flds ?? [];
      const fIdx = flds.findIndex((f) => f.name === fld);
      if (fIdx !== -1) {
        let expected = note.fields[fIdx] ?? "";
        if (clozeIdx !== null) {
          expected = (
            await extractClozeForTyping({ text: expected, ordinal: clozeIdx }, { alertOnError: false })
          ).val;
        }
        typeCorrect = expected || "";
        typeFont = flds[fIdx].font || "Arial";
        typeSize = flds[fIdx].size || 20;
      }
    } catch {
      // ignore
    }

    if (typeCorrect === null) {
      const warn = clozeIdx ? "(empty cloze)" : `(unknown field ${fld})`;
      return html.replace(TYPE_ANS_RE, warn);
    }
    if (!typeCorrect) {
      return html.replace(TYPE_ANS_RE, "");
    }
    return html.replace(
      TYPE_ANS_RE,
      `<center><input type="text" id="typeans" autocomplete="off" style="font-family: '${typeFont}'; font-size: ${typeSize}px;"></center>`
    );
  }

  async function filterTypeAnswerAnswer(html: string): Promise<string> {
    if (!typeCorrect) {
      return html.replace(TYPE_ANS_RE, "");
    }
    const orig = html;
    const withoutHr = html.replace("<hr id=answer>", "");
    const hadHr = withoutHr.length !== orig.length;
    if (hadHr && !TYPE_ANS_RE.test(withoutHr)) {
      return orig;
    }
    const diffHtml = (
      await compareAnswer(
        {
          expected: typeCorrect,
          provided: typedAnswer,
          combining: typeCombining,
          ignoreCase: typeIgnoreCase,
        },
        { alertOnError: false }
      )
    ).val;

    const renderedBlock = `${hadHr ? "<hr id=answer>" : ""}<div style="font-family: '${typeFont}'; font-size: ${typeSize}px;">${diffHtml}</div>`;
    return withoutHr.replace(TYPE_ANS_RE, () => renderedBlock);
  }

  // ============================================================================
  // Timers & Auto-Advance
  // ============================================================================

  function clearTimers() {
    if (timerHandle !== null) {
      clearInterval(timerHandle);
      timerHandle = null;
    }
    if (autoAdvanceTimer !== null) {
      clearTimeout(autoAdvanceTimer);
      autoAdvanceTimer = null;
    }
  }

  function startCardTimer() {
    clearTimers();
    cardStartMs = Date.now();
    elapsedSecs = 0;
    timerStopped = false;
    maxTimeSecs = deckConf?.capAnswerTimeToSecs ?? 60;
    showTimer = deckConf?.showTimer ?? false;

    timerHandle = window.setInterval(() => {
      if (!timerStopped) {
        elapsedSecs = Math.min(maxTimeSecs, elapsedSecs + 1);
      }
    }, 1000);
  }

  function scheduleAutoAdvanceForQuestion() {
    if (autoAdvanceTimer !== null) clearTimeout(autoAdvanceTimer);
    if (!autoAdvanceEnabled || !deckConf?.secondsToShowQuestion) return;
    autoAdvanceTimer = window.setTimeout(() => {
      if (phase === "question" && autoAdvanceEnabled) {
        void revealAnswer();
      }
    }, deckConf.secondsToShowQuestion * 1000);
  }

  function scheduleAutoAdvanceForAnswer() {
    if (autoAdvanceTimer !== null) clearTimeout(autoAdvanceTimer);
    if (!autoAdvanceEnabled || !deckConf?.secondsToShowAnswer) return;
    autoAdvanceTimer = window.setTimeout(() => {
      if (phase === "answer" && autoAdvanceEnabled) {
        void rateCard(3);
      }
    }, deckConf.secondsToShowAnswer * 1000);
  }

  function formatTimer(secs: number): string {
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return `${m}:${String(s).padStart(2, "0")}`;
  }

  // ============================================================================
  // Core Card Lifecycle
  // ============================================================================

  async function loadPreferences() {
    try {
      const prefs = await getPreferences({}, { alertOnError: false });
      if (prefs.reviewing) {
        showIntervals = prefs.reviewing.showIntervalsOnButtons;
        showRemainingCounts = prefs.reviewing.showRemainingDueCounts;
        hidePlayButtons = prefs.reviewing.hideAudioPlayButtons;
        interruptAudioOnAnswer = prefs.reviewing.interruptAudioWhenAnswering;
      }
    } catch {
      // keep defaults
    }
  }

  async function loadNextCard() {
    stopAudio();
    clearTimers();
    moreMenuOpen = false;
    phase = "loading";

    try {
      const queued = await getQueuedCards({ fetchLimit: 5, intradayLearningOnly: false });
      newCount = queued.newCount;
      learnCount = queued.learningCount;
      reviewCount = queued.reviewCount;

      if (queued.cards.length === 0 || !queued.cards[0].card) {
        if (card) previousCardId = Number(card.id);
        card = null;
        topQueued = null;
        store.view = "overview";
        return;
      }

      if (card && Number(card.id) !== Number(queued.cards[0].card.id)) {
        previousCardId = Number(card.id);
      }

      const top = queued.cards[0];
      const nextCard = top.card;
      if (!nextCard) return;
      topQueued = top;
      card = nextCard;
      activeQueueIdx =
        top.queue === QueuedCards_Queue.NEW
          ? 0
          : top.queue === QueuedCards_Queue.LEARNING
            ? 1
            : 2;

      const homeDeckId = nextCard.originalDeckId || nextCard.deckId;
      const [noteResp, renderResp, deckResp, labelsResp] = await Promise.all([
        getNote({ nid: nextCard.noteId }, { alertOnError: false }),
        renderExistingCard({ cardId: nextCard.id, browser: false, partialRender: false }),
        getDeck({ did: homeDeckId }, { alertOnError: false }).catch(() => null),
        top.states ? describeNextStates(top.states, { alertOnError: false }) : Promise.resolve({ vals: ["", "", "", ""] }),
      ]);

      const dcid = deckResp?.kind.case === "normal" ? deckResp.kind.value.configId : BigInt(1);
      const rawQ = nodesToHtml(renderResp.questionNodes);
      const rawA = nodesToHtml(renderResp.answerNodes);
      const [dconfResp, qAv, aAv] = await Promise.all([
        getDeckConfig({ dcid }, { alertOnError: false }).catch(() => null),
        extractAvTags({ text: rawQ, questionSide: true }, { alertOnError: false }),
        extractAvTags({ text: rawA, questionSide: false }, { alertOnError: false }),
      ]);

      note = noteResp;
      rendered = renderResp;
      questionHtml = qAv.text;
      answerHtml = aAv.text;
      questionAvTags = qAv.avTags;
      answerAvTags = aAv.avTags;
      deckConf = dconfResp?.config ?? null;
      intervalLabels = labelsResp.vals.length === 4 ? labelsResp.vals : ["", "", "", ""];

      await showQuestionSide();
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to load next card", "error");
      store.view = "overview";
    }
  }

  async function showQuestionSide() {
    if (!card || !rendered || !note) return;
    phase = "question";
    typedAnswer = "";
    startCardTimer();

    const qWithBtns = replaceAvPlaceholders(questionHtml);
    const qFinal = await filterTypeAnswerQuestion(qWithBtns);
    const css = `<style>${rendered.css}</style>`;
    const qHtml = css + qFinal;
    const aHtmlPreload = answerHtml;

    const bodyClass = `card card${card.templateIdx + 1}${store.isDark ? " nightMode night-mode" : ""}`;
    const flag = card.flags & 7;
    const isMarked = note.tags.some((t) => t.toLowerCase() === "marked");

    runInCardFrame((win) => {
      syncIframeTheme();
      if (typeof win._showQuestion === "function") {
        win._showQuestion(qHtml, aHtmlPreload, bodyClass);
      } else {
        const qa = win.document.getElementById("qa");
        if (qa) qa.innerHTML = qHtml;
      }
      if (typeof win._drawFlag === "function") win._drawFlag(flag);
      if (typeof win._drawMark === "function") win._drawMark(isMarked);
      // Focus typed answer input if present
      setTimeout(() => {
        const inp = win.document.getElementById("typeans") as HTMLInputElement | null;
        inp?.focus();
      }, 60);
    });

    const autoplay = deckConf ? !deckConf.disableAutoplay : true;
    if (autoplay && questionAvTags.length > 0) {
      playTags(questionAvTags);
    }

    scheduleAutoAdvanceForQuestion();
    pluginManager.emit("card:shown", {
      cardId: Number(card.id),
      noteId: Number(card.noteId),
      deckId: Number(card.deckId),
      phase: "question",
    });
  }

  async function revealAnswer() {
    if (phase !== "question" || !card || !rendered) return;

    // Capture typed answer if present in card iframe
    try {
      const win = iframeEl?.contentWindow as any;
      if (win && typeof win.getTypedAnswer === "function") {
        typedAnswer = win.getTypedAnswer() ?? "";
      }
    } catch {
      // ignore
    }

    phase = "answer";
    if (deckConf?.stopTimerOnAnswer) {
      timerStopped = true;
    }

    const aWithBtns = replaceAvPlaceholders(answerHtml);
    const aFinal = await filterTypeAnswerAnswer(aWithBtns);
    const aHtml = `<style>${rendered.css}</style>${aFinal}`;

    runInCardFrame((win) => {
      if (typeof win._showAnswer === "function") {
        win._showAnswer(aHtml);
      } else {
        const qa = win.document.getElementById("qa");
        if (qa) qa.innerHTML = aHtml;
      }
    });

    const autoplay = deckConf ? !deckConf.disableAutoplay : true;
    if (autoplay && answerAvTags.length > 0) {
      playTags(answerAvTags);
    }

    scheduleAutoAdvanceForAnswer();
  }

  async function rateCard(ease: 1 | 2 | 3 | 4) {
    if (phase !== "answer" || !card || !topQueued?.states) return;
    if (interruptAudioOnAnswer) stopAudio();
    clearTimers();
    phase = "transition";

    const states: SchedulingStates = topQueued.states;
    const nextState =
      ease === 1 ? states.again : ease === 2 ? states.hard : ease === 3 ? states.good : states.easy;
    const rating =
      ease === 1
        ? CardAnswer_Rating.AGAIN
        : ease === 2
          ? CardAnswer_Rating.HARD
          : ease === 3
            ? CardAnswer_Rating.GOOD
            : CardAnswer_Rating.EASY;

    const maxMs = (deckConf?.capAnswerTimeToSecs ?? 60) * 1000;
    const takenMs = Math.min(Math.max(100, Date.now() - cardStartMs), maxMs);

    try {
      await answerCard({
        cardId: card.id,
        currentState: states.current,
        newState: nextState,
        rating,
        answeredAtMillis: BigInt(Date.now()),
        millisecondsTaken: takenMs,
      });

      pluginManager.emit("card:answered", {
        cardId: Number(card.id),
        noteId: Number(card.noteId),
        deckId: Number(card.deckId),
        ease,
        takenMs,
      });

      if (nextState) {
        const leech = await stateIsLeech(nextState, { alertOnError: false }).catch(() => ({ val: false }));
        if (leech.val) {
          store.toast("Card marked as a leech!", "warning");
        }
      }
      await loadNextCard();
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to record answer", "error");
      phase = "answer";
    }
  }

  // ============================================================================
  // Card Actions (Flag, Mark, Bury, Suspend, Delete, Forget, Set Due Date)
  // ============================================================================

  async function setFlag(flag: number) {
    if (!card) return;
    moreMenuOpen = false;
    const currentFlag = card.flags & 7;
    const nextFlag = currentFlag === flag ? 0 : flag;
    await setCardFlagRpc({ flag: nextFlag, cardIds: [card.id] });
    card.flags = (card.flags & ~7) | nextFlag;
    runInCardFrame((win) => win._drawFlag?.(nextFlag));
    store.toast(nextFlag === 0 ? "Flag removed" : `Flag set: ${FLAG_NAMES[nextFlag]}`);
  }

  async function toggleMark() {
    if (!note) return;
    moreMenuOpen = false;
    const has = note.tags.some((t) => t.toLowerCase() === "marked");
    if (has) {
      await removeNoteTags({ noteIds: [note.id], tags: "marked" });
      note.tags = note.tags.filter((t) => t.toLowerCase() !== "marked");
      runInCardFrame((win) => win._drawMark?.(false));
      store.toast("Unmarked note");
    } else {
      await addNoteTags({ noteIds: [note.id], tags: "marked" });
      note.tags = [...note.tags, "marked"];
      runInCardFrame((win) => win._drawMark?.(true));
      store.toast("Marked note");
    }
  }

  async function buryCardOrNote(isNote: boolean) {
    if (!card) return;
    moreMenuOpen = false;
    await buryOrSuspendCards({
      cardIds: [card.id],
      noteIds: isNote ? [card.noteId] : [],
      mode: BuryOrSuspendCardsRequest_Mode.BURY_USER,
    });
    store.toast(isNote ? "Note buried" : "Card buried");
    await loadNextCard();
  }

  async function suspendCardOrNote(isNote: boolean) {
    if (!card) return;
    moreMenuOpen = false;
    await buryOrSuspendCards({
      cardIds: [card.id],
      noteIds: isNote ? [card.noteId] : [],
      mode: BuryOrSuspendCardsRequest_Mode.SUSPEND,
    });
    store.toast(isNote ? "Note suspended" : "Card suspended");
    await loadNextCard();
  }

  async function deleteCurrentNote() {
    if (!card) return;
    moreMenuOpen = false;
    const ok = await store.confirm("Delete the current note and all its cards?", "Delete Note", {
      danger: true,
      confirmLabel: "Delete",
    });
    if (!ok) return;
    await removeNotes({ noteIds: [card.noteId], cardIds: [] });
    store.toast("Note deleted");
    await loadNextCard();
  }

  async function forgetCurrentCard() {
    if (!card) return;
    moreMenuOpen = false;
    const ok = await store.confirm("Reset this card to New?", "Forget Card");
    if (!ok) return;
    await scheduleCardsAsNew({
      cardIds: [card.id],
      log: true,
      restorePosition: true,
      resetCounts: false,
    });
    store.toast("Card reset to New");
    await loadNextCard();
  }

  async function handleSetDueDate() {
    if (!card) return;
    moreMenuOpen = false;
    const days = await store.prompt(
      "Show in how many days? (e.g. 0 = today, 3 = in 3 days, 1-5 = random range, 3! = also update interval)",
      "0",
      "Set Due Date"
    );
    if (!days || !days.trim()) return;
    try {
      await setDueDate({ cardIds: [card.id], days: days.trim() });
      store.toast("Due date updated");
      await loadNextCard();
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Invalid due date format", "error");
    }
  }

  function toggleAutoAdvance() {
    moreMenuOpen = false;
    autoAdvanceEnabled = !autoAdvanceEnabled;
    store.toast(autoAdvanceEnabled ? "Auto Advance enabled" : "Auto Advance disabled");
    if (autoAdvanceEnabled) {
      if (phase === "question") scheduleAutoAdvanceForQuestion();
      else if (phase === "answer") scheduleAutoAdvanceForAnswer();
    } else if (autoAdvanceTimer !== null) {
      clearTimeout(autoAdvanceTimer);
      autoAdvanceTimer = null;
    }
  }

  // ============================================================================
  // Keyboard Shortcuts (matching qt/aqt/reviewer.py)
  // ============================================================================

  function handleShortcut(ev: {
    key: string;
    ctrlKey: boolean;
    metaKey: boolean;
    altKey: boolean;
    shiftKey: boolean;
    preventDefault?: () => void;
  }) {
    if (store.hasOpenModal) return;
    const k = ev.key;
    const mod = ev.ctrlKey || ev.metaKey;

    if (mod && ev.altKey && k.toLowerCase() === "i") {
      if (previousCardId) {
        ev.preventDefault?.();
        store.openCardInfo(previousCardId, "Previous Card Info");
      }
      return;
    }
    if (mod && ev.altKey && k.toLowerCase() === "n") {
      ev.preventDefault?.();
      void forgetCurrentCard();
      return;
    }
    if (mod && ev.shiftKey && k.toLowerCase() === "d") {
      ev.preventDefault?.();
      void handleSetDueDate();
      return;
    }
    if (mod && (k === "Delete" || k === "Backspace")) {
      ev.preventDefault?.();
      void deleteCurrentNote();
      return;
    }
    if (mod && !ev.shiftKey && !ev.altKey && k >= "1" && k <= "7") {
      ev.preventDefault?.();
      void setFlag(parseInt(k, 10));
      return;
    }
    if (mod || ev.altKey) return;

    if (k === " " || k === "Enter") {
      ev.preventDefault?.();
      if (phase === "question") {
        void revealAnswer();
      } else if (phase === "answer" && store.spacebarRatesCard) {
        void rateCard(3);
      }
      return;
    }

    // Answer rating keys (default 1-4, customizable in Preferences)
    if (phase === "answer") {
      for (const ease of [1, 2, 3, 4] as const) {
        if (store.answerKeys[ease] && k === store.answerKeys[ease]) {
          ev.preventDefault?.();
          void rateCard(ease);
          return;
        }
      }
    }

    if (k === "e") {
      if (card) {
        ev.preventDefault?.();
        store.openEditCurrent(Number(card.noteId), Number(card.id));
      }
    } else if (k === "m") {
      ev.preventDefault?.();
      moreMenuOpen = !moreMenuOpen;
    } else if (k === "r" || k === "F5") {
      ev.preventDefault?.();
      replayAudio();
    } else if (k === "u") {
      ev.preventDefault?.();
      void performUndo().then(() => loadNextCard());
    } else if (k === "*") {
      ev.preventDefault?.();
      void toggleMark();
    } else if (k === "-") {
      ev.preventDefault?.();
      void buryCardOrNote(false);
    } else if (k === "=") {
      ev.preventDefault?.();
      void buryCardOrNote(true);
    } else if (k === "@") {
      ev.preventDefault?.();
      void suspendCardOrNote(false);
    } else if (k === "!") {
      ev.preventDefault?.();
      void suspendCardOrNote(true);
    } else if (k === "i") {
      if (card) {
        ev.preventDefault?.();
        store.openCardInfo(Number(card.id));
      }
    } else if (k === "o") {
      if (card) {
        ev.preventDefault?.();
        store.openDeckOptions(Number(card.deckId));
      }
    } else if (k === "5") {
      ev.preventDefault?.();
      togglePauseAudio();
    } else if (k === "6") {
      ev.preventDefault?.();
      seekAudio(-5);
    } else if (k === "7") {
      ev.preventDefault?.();
      seekAudio(5);
    } else if (ev.shiftKey && k === "A") {
      ev.preventDefault?.();
      toggleAutoAdvance();
    }
  }

  function onWindowKeydown(e: KeyboardEvent) {
    const tag = (e.target as HTMLElement | null)?.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;
    handleShortcut(e);
  }

  function onWindowMessage(ev: MessageEvent) {
    if (!ev.data || (ev.source !== iframeEl?.contentWindow && ev.origin !== window.location.origin)) return;
    if (ev.data.type === "cardly:reviewer-cmd") {
      const cmd = String(ev.data.cmd ?? "");
      if (cmd === "ans") {
        void revealAnswer();
      } else if (cmd.startsWith("play:")) {
        playClickedAv(cmd);
      }
    } else if (ev.data.type === "cardly:reviewer-key") {
      handleShortcut(ev.data);
    }
  }

  // Lifecycle effects
  $effect(() => {
    void loadPreferences().then(() => loadNextCard());
    return () => {
      stopAudio();
      clearTimers();
    };
  });

  $effect(() => {
    void store.isDark;
    syncIframeTheme();
  });

  // Reload card content if note was edited via Edit Current or Undo
  let lastRev = store.opRevision;
  $effect(() => {
    const rev = store.opRevision;
    if (rev !== lastRev) {
      lastRev = rev;
      if (phase === "question" || phase === "answer") {
        void loadNextCard();
      }
    }
  });
</script>

<svelte:window
  onkeydown={onWindowKeydown}
  onmessage={onWindowMessage}
  onclick={() => {
    moreMenuOpen = false;
    flagSubmenuOpen = false;
  }}
/>

<main class="reviewer-screen">
  <div class="card-stage">
    <iframe
      bind:this={iframeEl}
      srcdoc={reviewerSrcDoc}
      sandbox="allow-scripts allow-same-origin"
      title="Card Review"
      class="card-iframe"
      onload={handleIframeLoad}
    ></iframe>
  </div>

  <PluginSlot id="card:after" class="card-after-slot" />

  <footer class="reviewer-bottom" class:hidden={store.hideBottomBar}>
    <div class="bottom-left">
      <button
        class="anki-btn"
        title="Shortcut: E"
        disabled={!card}
        onclick={() => card && store.openEditCurrent(Number(card.noteId), Number(card.id))}
      >
        Edit
      </button>
      <PluginSlot id="reviewer:actions" class="reviewer-actions-slot" />
    </div>

    <div class="bottom-center">
      {#if phase === "question"}
        <div class="show-answer-wrap">
          {#if showRemainingCounts}
            <div class="remaining-counts">
              <span class="new-count" class:active-q={activeQueueIdx === 0}>{newCount}</span>
              <span class="plus">+</span>
              <span class="learn-count" class:active-q={activeQueueIdx === 1}>{learnCount}</span>
              <span class="plus">+</span>
              <span class="review-count" class:active-q={activeQueueIdx === 2}>{reviewCount}</span>
            </div>
          {/if}
          <button id="ansbut" class="anki-btn primary show-answer-btn" title="Shortcut: Space" onclick={revealAnswer}>
            Show Answer
          </button>
        </div>
      {:else if phase === "answer"}
        <div class="ease-row">
          {#each [
            { ease: 1 as const, label: "Again", ivl: intervalLabels[0] },
            { ease: 2 as const, label: "Hard", ivl: intervalLabels[1] },
            { ease: 3 as const, label: "Good", ivl: intervalLabels[2], isDefault: true },
            { ease: 4 as const, label: "Easy", ivl: intervalLabels[3] },
          ] as btn}
            <div class="ease-col">
              {#if showIntervals}
                <div class="ivl-label">{btn.ivl}</div>
              {/if}
              <button
                class="anki-btn ease-btn"
                class:default-ease={btn.isDefault}
                data-ease={btn.ease}
                title={`Shortcut: ${store.answerKeys[btn.ease] ?? btn.ease}`}
                onclick={() => rateCard(btn.ease)}
              >
                {btn.label}
              </button>
            </div>
          {/each}
        </div>
      {:else}
        <div class="muted-label">Loading...</div>
      {/if}
    </div>

    <div class="bottom-right">
      <div class="more-anchor">
        {#if showTimer && maxTimeSecs > 0}
          <div class="card-timer" class:maxed={elapsedSecs >= maxTimeSecs}>
            {formatTimer(elapsedSecs)}
          </div>
        {/if}
        <button
          class="anki-btn more-trigger-btn"
          title="Shortcut: M"
          onclick={(e) => {
            e.stopPropagation();
            moreMenuOpen = !moreMenuOpen;
          }}
        >
          <span>More</span>
          <Icon name="chevron-down" size={14} />
        </button>

        {#if moreMenuOpen}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="more-backdrop" onclick={() => (moreMenuOpen = false)}>
            <div class="more-menu context-menu" onclick={(e) => e.stopPropagation()} role="menu">
              <div class="more-sheet-header">
                <span class="more-sheet-title">Card Options</span>
                <button
                  type="button"
                  class="more-sheet-close"
                  aria-label="Close options"
                  onclick={() => (moreMenuOpen = false)}
                >
                  <Icon name="x" size={18} />
                </button>
              </div>
              <div class="flag-row">
                <span class="flag-heading">Flag:</span>
                {#each [0, 1, 2, 3, 4, 5, 6, 7] as f}
                  <button
                    class="flag-dot"
                    class:active={card && (card.flags & 7) === f}
                    style={f === 0 ? "" : `background: ${FLAG_COLORS[f]}`}
                    title={f === 0 ? "No Flag" : `${FLAG_NAMES[f]} (Ctrl+${f})`}
                    aria-label={f === 0 ? "No Flag" : FLAG_NAMES[f]}
                    onclick={() => setFlag(f)}
                  >
                    {#if f === 0}<Icon name="x" size={12} />{/if}
                  </button>
                {/each}
              </div>
              <div class="context-menu-sep"></div>
              <button class="context-menu-item" onclick={toggleMark}>
                <span>{note?.tags.some((t) => t.toLowerCase() === "marked") ? "Unmark Note" : "Mark Note"}</span>
                <span class="shortcut">*</span>
              </button>
              <button class="context-menu-item" onclick={() => buryCardOrNote(false)}>
                <span>Bury Card</span><span class="shortcut">-</span>
              </button>
              <button class="context-menu-item" onclick={() => buryCardOrNote(true)}>
                <span>Bury Note</span><span class="shortcut">=</span>
              </button>
              <button class="context-menu-item" onclick={() => suspendCardOrNote(false)}>
                <span>Suspend Card</span><span class="shortcut">@</span>
              </button>
              <button class="context-menu-item" onclick={() => suspendCardOrNote(true)}>
                <span>Suspend Note</span><span class="shortcut">!</span>
              </button>
              <div class="context-menu-sep"></div>
              <button class="context-menu-item" onclick={forgetCurrentCard}>
                <span>Forget...</span><span class="shortcut">Ctrl+Alt+N</span>
              </button>
              <button class="context-menu-item" onclick={handleSetDueDate}>
                <span>Set Due Date...</span><span class="shortcut">Ctrl+Shift+D</span>
              </button>
              <button class="context-menu-item" onclick={deleteCurrentNote}>
                <span style="color: var(--accent-danger)">Delete Note</span>
                <span class="shortcut">Ctrl+Del</span>
              </button>
              <div class="context-menu-sep"></div>
              <button
                class="context-menu-item"
                onclick={() => {
                  moreMenuOpen = false;
                  if (card) store.openCardInfo(Number(card.id));
                }}
              >
                <span>Card Info...</span><span class="shortcut">I</span>
              </button>
              {#if previousCardId}
                <button
                  class="context-menu-item"
                  onclick={() => {
                    moreMenuOpen = false;
                    store.openCardInfo(previousCardId, "Previous Card Info");
                  }}
                >
                  <span>Previous Card Info...</span><span class="shortcut">Ctrl+Alt+I</span>
                </button>
              {/if}
              <button
                class="context-menu-item"
                onclick={() => {
                  moreMenuOpen = false;
                  if (card) store.openDeckOptions(Number(card.deckId));
                }}
              >
                <span>Options...</span><span class="shortcut">O</span>
              </button>
              <div class="context-menu-sep"></div>
              <button
                class="context-menu-item"
                onclick={() => {
                  moreMenuOpen = false;
                  replayAudio();
                }}
              >
                <span>Replay Audio</span><span class="shortcut">R</span>
              </button>
              <button class="context-menu-item" onclick={toggleAutoAdvance}>
                <span class="menu-label-with-icon">
                  {#if autoAdvanceEnabled}<Icon name="check" size={14} />{/if}
                  <span>Auto Advance</span>
                </span>
                <span class="shortcut">Shift+A</span>
              </button>
            </div>
          </div>
        {/if}
      </div>
    </div>
  </footer>
</main>

<style>
  .reviewer-screen {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .card-stage {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .card-iframe {
    width: 100%;
    height: 100%;
    border: none;
    background: var(--canvas);
    display: block;
  }
  .reviewer-bottom {
    flex-shrink: 0;
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: end;
    padding: 8px 16px 12px;
    border-top: 1px solid var(--border);
    background: var(--canvas);
    user-select: none;
    min-height: 66px;
  }
  .reviewer-bottom.hidden {
    display: none;
  }
  .bottom-left {
    justify-self: start;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    min-width: 0;
  }
  :global(.card-after-slot) {
    justify-content: center;
    padding: 6px 16px;
    border-top: 1px solid var(--border-subtle);
    background: var(--canvas);
  }
  :global(.reviewer-actions-slot) {
    gap: 6px;
  }
  .bottom-center {
    justify-self: center;
  }
  .bottom-right {
    justify-self: end;
  }
  .show-answer-wrap {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }
  .remaining-counts {
    font-size: 12.5px;
    display: flex;
    align-items: center;
    gap: 5px;
    font-variant-numeric: tabular-nums;
  }
  .remaining-counts .plus {
    color: var(--fg-subtle);
  }
  .remaining-counts .active-q {
    text-decoration: underline;
    font-weight: 700;
  }
  .show-answer-btn {
    min-width: 130px;
  }
  .ease-row {
    display: flex;
    align-items: flex-end;
    gap: 10px;
  }
  .ease-col {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
  }
  .ivl-label {
    font-size: 12px;
    color: var(--fg-subtle);
    min-height: 16px;
    white-space: nowrap;
  }
  .ease-btn {
    min-width: 72px;
  }
  .ease-btn.default-ease {
    border-color: var(--border-focus);
  }
  .more-anchor {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 3px;
  }
  .card-timer {
    font-size: 12px;
    color: var(--fg-subtle);
    font-variant-numeric: tabular-nums;
  }
  .card-timer.maxed {
    color: var(--accent-danger);
    font-weight: 600;
  }
  .more-backdrop {
    display: contents;
  }
  .more-sheet-header {
    display: none;
  }
  .menu-label-with-icon {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .more-menu {
    position: absolute;
    bottom: calc(100% + 6px);
    right: 0;
    min-width: 240px;
  }
  .flag-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
  }
  .flag-heading {
    font-size: 12px;
    color: var(--fg-subtle);
    margin-right: 2px;
  }
  .flag-dot {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 1px solid var(--border);
    background: var(--canvas-inset);
    color: var(--fg);
    cursor: pointer;
    padding: 0;
    font-size: 11px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .flag-dot.active {
    outline: 2px solid var(--border-focus);
    outline-offset: 1px;
  }
  .muted-label {
    color: var(--fg-subtle);
    font-size: 13px;
    padding-bottom: 6px;
  }
  @media (max-width: 640px) {
    .reviewer-bottom {
      display: grid;
      grid-template-columns: 1fr 1fr;
      align-items: center;
      gap: 8px;
      padding: 8px calc(10px + var(--safe-right)) calc(10px + var(--safe-bottom)) calc(10px + var(--safe-left));
      min-height: auto;
    }
    .bottom-left {
      justify-self: start;
      order: 1;
    }
    .bottom-right {
      justify-self: end;
      order: 1;
    }
    .bottom-center {
      grid-column: 1 / -1;
      width: 100%;
      order: 2;
    }
    .more-anchor {
      flex-direction: row;
      align-items: center;
      gap: 8px;
    }
    .show-answer-wrap {
      width: 100%;
    }
    .show-answer-btn {
      width: 100%;
      min-height: 44px;
      font-size: 15px;
    }
    .ease-row {
      width: 100%;
      gap: 5px;
    }
    .ease-col {
      flex: 1;
      min-width: 0;
    }
    .ease-btn {
      width: 100%;
      min-width: 0;
      min-height: 44px;
      padding: 6px 2px;
      font-size: 13px;
    }
    .ivl-label {
      font-size: 11px;
    }
  }

  @media (max-width: 640px), (hover: none) and (pointer: coarse) {
    .more-backdrop {
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.48);
      z-index: 2500;
      display: flex;
      align-items: flex-end;
      justify-content: center;
    }
    .more-menu {
      position: static;
      width: 100%;
      max-width: 540px;
      max-height: 82dvh;
      overflow-y: auto;
      border-radius: 16px 16px 0 0;
      border-bottom: none;
      padding: 0 8px calc(12px + var(--safe-bottom));
      box-shadow: 0 -8px 30px rgba(0, 0, 0, 0.3);
    }
    .more-sheet-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 10px 8px;
      border-bottom: 1px solid var(--border-subtle);
      position: sticky;
      top: 0;
      background: var(--canvas-overlay);
      z-index: 1;
    }
    .more-sheet-title {
      font-size: 15px;
      font-weight: 700;
    }
    .more-sheet-close {
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
    .more-menu .context-menu-item {
      min-height: 44px;
      padding: 10px 12px;
      font-size: 15px;
    }
    .flag-row {
      padding: 10px 8px;
      justify-content: space-between;
      flex-wrap: wrap;
    }
    .flag-dot {
      width: 30px;
      height: 30px;
    }
  }
</style>
