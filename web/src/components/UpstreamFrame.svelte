<script lang="ts">
  import { injectMobileResponsiveCss } from "../lib/bridge";
  import { store } from "../lib/store.svelte";

  interface Props {
    src: string;
    title: string;
    onFrameLoad?: (win: Window) => void;
  }

  let { src, title, onFrameLoad }: Props = $props();
  let iframeEl = $state<HTMLIFrameElement | null>(null);

  const effectiveSrc = $derived.by(() => {
    const [path, hash] = src.split("#");
    if (store.isDark) {
      return `${path}#night`;
    }
    return hash && hash !== "night" ? `${path}#${hash}` : path;
  });

  function syncThemeToFrame() {
    if (!iframeEl) return;
    try {
      const doc = iframeEl.contentDocument;
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
      injectMobileResponsiveCss(doc);
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
  src={effectiveSrc}
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
