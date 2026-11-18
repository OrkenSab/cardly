<script lang="ts">
  import { addMediaFile } from "@generated/backend";
  import { store } from "../lib/store.svelte";

  let recording = $state(false);
  let secs = $state(0);
  let error = $state("");
  let recorder: MediaRecorder | null = null;
  let stream: MediaStream | null = null;
  let chunks: Blob[] = [];
  let timer: number | null = null;

  async function startRecording() {
    error = "";
    try {
      stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const mimeType = MediaRecorder.isTypeSupported("audio/webm;codecs=opus")
        ? "audio/webm;codecs=opus"
        : "audio/webm";
      recorder = new MediaRecorder(stream, { mimeType });
      chunks = [];
      recorder.ondataavailable = (ev) => {
        if (ev.data.size > 0) chunks.push(ev.data);
      };
      recorder.onstop = async () => {
        stopStream();
        if (chunks.length === 0) {
          store.audioRecorder.resolve?.(null);
          return;
        }
        const blob = new Blob(chunks, { type: "audio/webm" });
        const bytes = new Uint8Array(await blob.arrayBuffer());
        const filename = `rec-${Date.now()}.webm`;
        try {
          const saved = (await addMediaFile({ desiredName: filename, data: bytes })).val;
          store.audioRecorder.resolve?.(saved);
        } catch (err) {
          error = err instanceof Error ? err.message : "Failed to upload audio";
        }
      };
      recorder.start();
      recording = true;
      secs = 0;
      timer = window.setInterval(() => (secs += 1), 1000);
    } catch (err) {
      error = err instanceof Error ? err.message : "Microphone access denied";
    }
  }

  function stopStream() {
    if (timer !== null) {
      clearInterval(timer);
      timer = null;
    }
    stream?.getTracks().forEach((t) => t.stop());
    stream = null;
    recording = false;
  }

  function finish() {
    if (recorder && recorder.state !== "inactive") {
      recorder.stop();
    } else {
      cancel();
    }
  }

  function cancel() {
    chunks = [];
    if (recorder && recorder.state !== "inactive") {
      recorder.onstop = null;
      recorder.stop();
    }
    stopStream();
    store.audioRecorder.resolve?.(null);
  }

  $effect(() => {
    void startRecording();
    return () => {
      if (recorder && recorder.state !== "inactive") {
        recorder.onstop = null;
        recorder.stop();
      }
      stopStream();
    };
  });
</script>

<div class="modal-backdrop" style="z-index: 2500" role="dialog" aria-modal="true">
  <div class="modal-window rec-box">
    <div class="modal-header">
      <h3 class="modal-title">Record Audio</h3>
      <button class="modal-close" onclick={cancel}>×</button>
    </div>
    <div class="modal-body rec-body">
      {#if error}
        <p class="err">{error}</p>
      {:else}
        <div class="rec-indicator" class:live={recording}></div>
        <div class="rec-time">
          {recording ? `Recording… ${secs}s` : "Preparing microphone…"}
        </div>
      {/if}
    </div>
    <div class="modal-footer">
      <button class="anki-btn" onclick={cancel}>Cancel</button>
      <button class="anki-btn primary" disabled={!recording} onclick={finish}>Save Recording</button>
    </div>
  </div>
</div>

<style>
  .rec-box {
    width: 340px;
  }
  .rec-body {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 24px 16px;
  }
  .rec-indicator {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--fg-faint);
  }
  .rec-indicator.live {
    background: var(--accent-danger);
    box-shadow: 0 0 0 6px rgba(239, 68, 68, 0.25);
  }
  .rec-time {
    font-size: 14px;
    font-variant-numeric: tabular-nums;
  }
  .err {
    color: var(--accent-danger);
    font-size: 13px;
    margin: 0;
  }
</style>
