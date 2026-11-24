import type { RenderedTemplateNode } from "@generated/anki/card_rendering_pb";

/**
 * Joins `RenderedTemplateNode[]` returned by `renderExistingCard` / `renderUncommittedCard`
 * into an HTML string. When `partialRender: false`, rslib resolves standard filters into
 * `text` nodes; any remaining `replacement` node falls back to its current text.
 */
export function nodesToHtml(nodes: RenderedTemplateNode[]): string {
  let out = "";
  for (const node of nodes) {
    if (node.value.case === "text") {
      out += node.value.value;
    } else if (node.value.case === "replacement") {
      out += node.value.value.currentText;
    }
  }
  return out;
}

/**
 * Replaces `[sound:file.mp3]` and `[anki:play:...]` markers with clean preview badges
 * for non-interactive card previews (Card Templates & Card Previewer).
 */
export function renderSoundPlaceholders(html: string): string {
  return html.replace(
    /\[sound:([^\]]+)\]/g,
    (_m, fname: string) =>
      `<button type="button" class="replay-button" title="${fname.replace(/"/g, "&quot;")}" onclick="new Audio('/'+encodeURIComponent('${fname.replace(/'/g, "\\'")}')).play()">▶</button>`
  );
}

/**
 * Builds a complete self-contained iframe `srcdoc` document matching Desktop Anki's card rendering.
 */
export function buildCardPreviewDoc(opts: {
  bodyHtml: string;
  css: string;
  isDark: boolean;
  includeMathJax?: boolean;
}): string {
  const darkClass = opts.isDark ? "nightMode night-mode" : "";
  const mathjax = opts.includeMathJax
    ? `<script>window.MathJax={tex:{inlineMath:[['\\\\(','\\\\)']],displayMath:[['\\\\[','\\\\]']]},svg:{fontCache:'none'}};</script><script defer src="/_anki/js/vendor/mathjax/tex-chtml-full.js"></script>`
    : "";
  const bsTheme = opts.isDark ? "dark" : "light";
  return `<!doctype html>
<html class="${darkClass}" data-bs-theme="${bsTheme}" style="color-scheme: ${bsTheme}">
<head>
<meta charset="utf-8">
<base href="${window.location.origin}/">
<link rel="stylesheet" href="/_anki/css/webview.css">
<link rel="stylesheet" href="/_anki/css/reviewer.css">
<style>
  html, body { margin: 0; padding: 12px 16px; min-height: 100%; box-sizing: border-box; }
  .replay-button {
    display: inline-flex; align-items: center; justify-content: center;
    width: 28px; height: 28px; border-radius: 50%;
    border: 1px solid var(--border, #c4c4c4); background: var(--button-bg, #fff);
    color: var(--fg, #020202); cursor: pointer; font-size: 11px; vertical-align: middle; margin: 0 3px;
  }
  ${opts.css}
</style>
${mathjax}
</head>
<body class="card ${darkClass}">
<div id="qa">${renderSoundPlaceholders(opts.bodyHtml)}</div>
</body>
</html>`;
}
