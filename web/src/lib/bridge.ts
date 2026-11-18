// Browser-side FrontendService RPC interceptor & bridgeCommand handler for embedded upstream `ts/` iframes

import { addMediaFile, buildSearchString, changeNotetype, getNote, unburyDeck } from "@generated/backend";
import {
  AskUserRequest,
  ReadClipboardRequest,
  ReadClipboardResponse,
  ShowMessageBoxRequest,
  WriteClipboardRequest,
  openFilePickerRequest,
} from "@generated/anki/frontend_pb";
import { Bool, String as ProtoString } from "@generated/anki/generic_pb";
import { ChangeNotetypeRequest } from "@generated/anki/notetypes_pb";
import { UnburyDeckRequest_Mode } from "@generated/anki/scheduler_pb";
import { SearchNode } from "@generated/anki/search_pb";
import { refreshUndoStatus, uploadFile } from "./api";
import { store } from "./store.svelte";

let activeAudio: HTMLAudioElement | null = null;
let bridgeInstalled = false;

function toUint8Array(body: BodyInit | null | undefined): Uint8Array {
  if (!body) return new Uint8Array(0);
  if (body instanceof Uint8Array) return body;
  if (body instanceof ArrayBuffer) return new Uint8Array(body);
  if (ArrayBuffer.isView(body)) {
    return new Uint8Array(body.buffer, body.byteOffset, body.byteLength);
  }
  return new Uint8Array(0);
}

function protoResponse(bytes: Uint8Array): Response {
  if (bytes.byteLength === 0) {
    return new Response(null, { status: 204 });
  }
  return new Response(bytes as unknown as BodyInit, {
    status: 200,
    headers: { "Content-Type": "application/binary" },
  });
}

function pickLocalFile(extensions: string[]): Promise<File | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    if (extensions.length > 0) {
      input.accept = extensions.map((e) => (e.startsWith(".") ? e : `.${e}`)).join(",");
    }
    let settled = false;
    const finish = (file: File | null) => {
      if (settled) return;
      settled = true;
      input.remove();
      resolve(file);
    };
    input.addEventListener("change", () => {
      finish(input.files?.[0] ?? null);
    });
    input.addEventListener("cancel", () => finish(null));
    // Fallback if user dismisses picker without firing `cancel`
    window.addEventListener(
      "focus",
      () => {
        setTimeout(() => {
          if (!settled && (!input.files || input.files.length === 0)) {
            finish(null);
          }
        }, 400);
      },
      { once: true }
    );
    input.style.display = "none";
    document.body.appendChild(input);
    input.click();
  });
}

export function playMediaFile(filename: string): void {
  if (!filename) return;
  if (activeAudio) {
    activeAudio.pause();
    activeAudio = null;
  }
  const url = `/${encodeURIComponent(filename)}`;
  const audio = new Audio(url);
  activeAudio = audio;
  audio.play().catch((err) => {
    console.warn("Audio playback failed:", err);
  });
}

export function installUpstreamBridge(): void {
  if (bridgeInstalled) return;
  bridgeInstalled = true;

  // 1. Intercept browser-side `FrontendService` RPCs called by upstream `ts/` iframes.
  window.__CARDLY_INTERCEPT_RPC__ = async (method: string, body: BodyInit | null | undefined) => {
    const bytes = toUint8Array(body);

    switch (method) {
      case "deckOptionsReady": {
        return protoResponse(new Uint8Array(0));
      }

      case "deckOptionsRequireClose": {
        store.deckOptions = { open: false, deckId: store.deckOptions.deckId };
        store.bumpOpRevision();
        return protoResponse(new Uint8Array(0));
      }

      case "importDone": {
        store.bumpOpRevision();
        void refreshUndoStatus();
        return protoResponse(new Uint8Array(0));
      }

      case "importDialogRequireClose": {
        store.importModal = { open: false, mode: "pick" };
        store.bumpOpRevision();
        return protoResponse(new Uint8Array(0));
      }

      case "closeAddCards": {
        const shouldPrompt = bytes.length > 0 ? Bool.fromBinary(bytes).val : false;
        if (shouldPrompt) {
          const ok = await store.confirm("Discard current changes and close Add window?", "Close Add");
          if (!ok) return protoResponse(new Uint8Array(0));
        }
        store.addCardsOpen = false;
        store.bumpOpRevision();
        return protoResponse(new Uint8Array(0));
      }

      case "closeEditCurrent": {
        store.editCurrent = { open: false, noteId: 0 };
        store.bumpOpRevision();
        return protoResponse(new Uint8Array(0));
      }

      case "searchInBrowser": {
        const node = SearchNode.fromBinary(bytes);
        try {
          const query = (await buildSearchString(node, { alertOnError: false })).val;
          store.openBrowser(query);
        } catch {
          store.openBrowser("");
        }
        return protoResponse(new Uint8Array(0));
      }

      case "changeNotetype": {
        // Attach the noteIds stored when opening the Change Note Type modal (matches qt/aqt/changenotetype.py)
        const req = ChangeNotetypeRequest.fromBinary(bytes);
        if (req.noteIds.length === 0 && store.changeNotetype.noteIds.length > 0) {
          req.noteIds = store.changeNotetype.noteIds.map((id) => BigInt(id));
        }
        await changeNotetype(req);
        store.toast(`Updated ${req.noteIds.length} note(s)`);
        store.changeNotetype = { open: false, noteIds: [], notetypeId: 0 };
        store.bumpOpRevision();
        return protoResponse(new Uint8Array(0));
      }

      case "openFilePicker": {
        const req = openFilePickerRequest.fromBinary(bytes);
        const file = await pickLocalFile(req.extensions);
        if (!file) {
          return protoResponse(new ProtoString({ val: "" }).toBinary());
        }
        // Upload to user's temp dir so subsequent `addMediaFromPath` or `image-occlusion` works safely
        try {
          const uploaded = await uploadFile(file);
          return protoResponse(new ProtoString({ val: uploaded.upload_id }).toBinary());
        } catch (err) {
          store.toast(err instanceof Error ? err.message : "File upload failed", "error");
          return protoResponse(new ProtoString({ val: "" }).toBinary());
        }
      }

      case "recordAudio": {
        const filename = await store.requestAudioRecording();
        return protoResponse(new ProtoString({ val: filename ?? "" }).toBinary());
      }

      case "playFile": {
        const fname = ProtoString.fromBinary(bytes).val;
        playMediaFile(fname);
        return protoResponse(new Uint8Array(0));
      }

      case "openMedia": {
        const fname = ProtoString.fromBinary(bytes).val;
        if (fname) {
          window.open(`/${encodeURIComponent(fname)}`, "_blank", "noopener");
        }
        return protoResponse(new Uint8Array(0));
      }

      case "showInMediaFolder": {
        const fname = ProtoString.fromBinary(bytes).val;
        store.toast(`Media file stored on server: ${fname}`);
        return protoResponse(new Uint8Array(0));
      }

      case "openLink": {
        const url = ProtoString.fromBinary(bytes).val;
        if (url.startsWith("https://") || url.startsWith("http://")) {
          window.open(url, "_blank", "noopener,noreferrer");
        }
        return protoResponse(new Uint8Array(0));
      }

      case "askUser": {
        const req = AskUserRequest.fromBinary(bytes);
        const answer = await store.confirm(req.text, req.title ?? "Cardly");
        return protoResponse(new Bool({ val: answer }).toBinary());
      }

      case "showMessageBox": {
        const req = ShowMessageBoxRequest.fromBinary(bytes);
        await store.alert(req.text, req.title ?? "Cardly");
        return protoResponse(new Bool({ val: true }).toBinary());
      }

      case "openFieldsDialog": {
        let ntid = store.activeEditorNotetypeId;
        if (!ntid && store.activeEditorNoteId) {
          try {
            const note = await getNote({ nid: BigInt(store.activeEditorNoteId) });
            ntid = Number(note.notetypeId);
          } catch {
            // fallback
          }
        }
        if (ntid) {
          store.openFields(ntid);
        } else {
          store.manageNotetypesOpen = true;
        }
        return protoResponse(new Uint8Array(0));
      }

      case "openCardsDialog": {
        let ntid = store.activeEditorNotetypeId;
        if (!ntid && store.activeEditorNoteId) {
          try {
            const note = await getNote({ nid: BigInt(store.activeEditorNoteId) });
            ntid = Number(note.notetypeId);
          } catch {
            // fallback
          }
        }
        if (ntid) {
          store.openCardTemplates(ntid, store.activeEditorNoteId || undefined);
        } else {
          store.manageNotetypesOpen = true;
        }
        return protoResponse(new Uint8Array(0));
      }

      case "readClipboard": {
        const req = ReadClipboardRequest.fromBinary(bytes);
        const data: Record<string, Uint8Array> = {};
        try {
          if (navigator.clipboard && navigator.clipboard.read) {
            const items = await navigator.clipboard.read();
            for (const item of items) {
              for (const t of req.types) {
                if (item.types.includes(t)) {
                  const blob = await item.getType(t);
                  data[t] = new Uint8Array(await blob.arrayBuffer());
                }
              }
            }
          }
        } catch {
          // Clipboard permission denied or unavailable; return empty response gracefully
        }
        return protoResponse(new ReadClipboardResponse({ data }).toBinary());
      }

      case "writeClipboard": {
        const req = WriteClipboardRequest.fromBinary(bytes);
        try {
          if (navigator.clipboard && navigator.clipboard.write) {
            const blobMap: Record<string, Blob> = {};
            for (const [mime, raw] of Object.entries(req.data)) {
              blobMap[mime] = new Blob([new Uint8Array(raw)], { type: mime });
            }
            if (Object.keys(blobMap).length > 0) {
              await navigator.clipboard.write([new ClipboardItem(blobMap)]);
            }
          }
        } catch {
          // ignore
        }
        return protoResponse(new Uint8Array(0));
      }

      default:
        // Forward to Rust backend
        return null;
    }
  };

  // 2. Listen for `postMessage` bridge commands & OpChanges from embedded upstream iframes.
  window.addEventListener("message", async (ev: MessageEvent) => {
    if (ev.origin !== window.location.origin || !ev.data || typeof ev.data !== "object") {
      return;
    }
    const { type } = ev.data;

    if (type === "cardly:op-changes") {
      store.bumpOpRevision();
      void refreshUndoStatus();
      return;
    }

    if (type === "cardly:bridge") {
      const cmd = String(ev.data.cmd ?? "");
      const id = ev.data.id;
      const reply = (value: unknown) => {
        try {
          (ev.source as Window | null)?.postMessage(
            { type: "cardly:bridge-reply", id, value },
            window.location.origin
          );
        } catch {
          // ignore
        }
      };

      if (cmd.startsWith("browserSearch:")) {
        const query = cmd.slice("browserSearch:".length).trim();
        store.openBrowser(query);
        reply(true);
      } else if (cmd === "customStudy") {
        store.customStudyOpen = true;
        reply(true);
      } else if (cmd === "unbury") {
        await unburyDeck({
          deckId: BigInt(store.currentDeckId),
          mode: UnburyDeckRequest_Mode.ALL,
        });
        store.bumpOpRevision();
        store.toast("Unburied cards");
        reply(true);
      } else if (cmd === "saved") {
        store.bumpOpRevision();
        void refreshUndoStatus();
        reply(true);
      } else if (cmd === "preview") {
        if (store.editCurrent.cardId) {
          store.openPreviewer([store.editCurrent.cardId]);
        }
        reply(true);
      } else if (cmd === "fields") {
        if (store.activeEditorNotetypeId) {
          store.openFields(store.activeEditorNotetypeId);
        } else {
          store.manageNotetypesOpen = true;
        }
        reply(true);
      } else if (cmd === "cards") {
        if (store.activeEditorNotetypeId) {
          store.openCardTemplates(store.activeEditorNotetypeId, store.activeEditorNoteId || undefined);
        } else {
          store.manageNotetypesOpen = true;
        }
        reply(true);
      } else if (cmd === "addImageForOcclusion") {
        const file = await pickLocalFile(["png", "jpg", "jpeg", "gif", "webp", "svg"]);
        if (file) {
          const buf = new Uint8Array(await file.arrayBuffer());
          const savedName = (await addMediaFile({ desiredName: file.name, data: buf })).val;
          store.openImageOcclusion(savedName);
        }
        reply(true);
      } else {
        reply(undefined);
      }
    }
  });
}
