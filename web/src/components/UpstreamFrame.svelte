<script lang="ts">
  import { store } from "../lib/store.svelte";

  interface Props {
    src: string;
    title: string;
    onFrameLoad?: (win: Window) => void;
  }

  let { src, title, onFrameLoad }: Props = $props();
  let iframeEl = $state<HTMLIFrameElement | null>(null);

  function syncThemeToFrame() {
    if (!iframeEl) return;
    try {
      const doc = iframeEl.contentDocument;
      if (!doc) return;
      const dark = store.isDark;
      doc.documentElement.classList.toggle("night-mode", dark);
      doc.documentElement.classList.toggle("nightMode", dark);
      if (doc.body) {
        doc.body.classList.toggle("night-mode", dark);
        doc.body.classList.toggle("nightMode", dark);
      }
    } catch {
      // ignore
    }
  }

  function handleLoad() {
    syncThemeToFrame();
    if (iframeEl?.contentWindow && onFrameLoad) {
      onFrameLoad(iframeEl.contentWindow);
    }
  }

  $effect(() => {
    void store.isDark;
    syncThemeToFrame();
  });
</script>

<iframe
  bind:this={iframeEl}
  {src}
  {title}
  class="upstream-frame"
  onload={handleLoad}
></iframe>

<style>
  .upstream-frame {
    width: 100%;
    height: 100%;
    border: none;
    background: var(--canvas);
    display: block;
    flex: 1;
  }
</style>
