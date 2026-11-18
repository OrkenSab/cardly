// Copyright: Cardly contributors
// License: GNU AGPL, version 3 or later; https://www.gnu.org/licenses/agpl.html

use std::sync::Arc;
use std::sync::OnceLock;

use axum::body::Body;
use axum::extract::State;
use axum::http::header;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use regex::bytes::Regex;

use crate::auth::authenticate_request;
use crate::media::ensure_safe_path;
use crate::media::mime_for_path;
use crate::media::serve_user_media;
use crate::AppState;

/// SvelteKit routes built from upstream `anki-upstream/ts/routes/`.
const SVELTEKIT_ROUTES: &[&str] = &[
    "graphs",
    "congrats",
    "card-info",
    "change-notetype",
    "deck-options",
    "import-anki-package",
    "import-csv",
    "import-page",
    "image-occlusion",
    "preferences",
    "editor",
];

fn is_untrusted_sveltekit_route(route: &str) -> bool {
    matches!(route, "editor" | "image-occlusion")
}

pub fn detect_sveltekit_route(path: &str) -> Option<&'static str> {
    let first = path.trim_start_matches('/').split('/').next()?;
    SVELTEKIT_ROUTES.iter().copied().find(|&r| r == first)
}

fn sveltekit_csp_meta_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"<meta http-equiv="content-security-policy" content="script-src 'self' ('sha256-[A-Za-z0-9+/=]+')">"#,
        )
        .unwrap()
    })
}

/// Bridge script injected into upstream `ts/` SvelteKit pages via `<script src="/_anki/cardly-bridge.js">`.
/// 1. Fetches/caches the user's per-session `X-Cardly-Api-Token` and attaches it to all `POST /_anki/*` calls.
/// 2. Provides `window.bridgeCommand` forwarding to the Cardly parent shell via `postMessage` when embedded.
/// 3. Strips/replaces "Anki" brand strings in document titles with "Cardly".
pub const CARDLY_BRIDGE_JS: &str = r#"// Cardly Web Bridge (AGPL-3.0-or-later)
(function () {
  const origFetch = window.fetch.bind(window);
  let tokenPromise = null;
  let cachedToken = null;

  function getToken() {
    if (cachedToken) return Promise.resolve(cachedToken);
    try {
      if (window.parent && window.parent !== window && window.parent.__CARDLY_API_TOKEN__) {
        cachedToken = window.parent.__CARDLY_API_TOKEN__;
        return Promise.resolve(cachedToken);
      }
    } catch (_) {}
    if (!tokenPromise) {
      tokenPromise = origFetch('/api/auth/me', { credentials: 'same-origin' })
        .then(function (r) { return r.ok ? r.json() : null; })
        .then(function (d) {
          if (d && d.api_token) {
            cachedToken = d.api_token;
            return cachedToken;
          }
          return null;
        })
        .catch(function () { return null; });
    }
    return tokenPromise;
  }

  // Synchronously warm the token from parent or /api/auth/me on boot.
  getToken();

  window.fetch = async function (input, init) {
    const url = typeof input === 'string' ? input : (input && input.url ? input.url : '');
    const isAnkiPost =
      (url.startsWith('/_anki/') || url.includes('/_anki/')) &&
      init &&
      String(init.method || 'GET').toUpperCase() === 'POST';

    if (isAnkiPost) {
      // Allow parent Cardly shell to intercept browser-side FrontendService RPCs (e.g. file picker, dialogs).
      try {
        if (window.parent && window.parent !== window && typeof window.parent.__CARDLY_INTERCEPT_RPC__ === 'function') {
          const method = url.split('/_anki/')[1].split('?')[0];
          const intercepted = await window.parent.__CARDLY_INTERCEPT_RPC__(method, init.body);
          if (intercepted !== null && intercepted !== undefined) {
            return intercepted;
          }
        }
      } catch (_) {}

      const tok = await getToken();
      const headers = new Headers(init.headers || {});
      if (tok && !headers.has('X-Cardly-Api-Token')) {
        headers.set('X-Cardly-Api-Token', tok);
      }
      const nextInit = Object.assign({}, init, { headers, credentials: 'same-origin' });
      const resp = await origFetch(input, nextInit);

      // Notify parent shell about OpChanges if header requested change tracking.
      try {
        const opLevel = parseInt(headers.get('Anki-Op-Changes') || '0', 10);
        if (opLevel > 0 && resp.ok && window.parent && window.parent !== window) {
          window.parent.postMessage({ type: 'cardly:op-changes', url: url, opLevel: opLevel }, window.location.origin);
        }
      } catch (_) {}
      return resp;
    }

    return origFetch(input, init);
  };

  // Implement Anki's window.bridgeCommand for reused ts/ routes.
  if (!window.bridgeCommand) {
    let reqCounter = 0;
    const pending = new Map();
    window.addEventListener('message', function (ev) {
      if (ev.origin !== window.location.origin || !ev.data || ev.data.type !== 'cardly:bridge-reply') return;
      const cb = pending.get(ev.data.id);
      if (cb) {
        pending.delete(ev.data.id);
        cb(ev.data.value);
      }
    });
    window.bridgeCommand = function (cmd, callback) {
      if (window.parent && window.parent !== window) {
        const id = ++reqCounter;
        if (typeof callback === 'function') {
          pending.set(id, callback);
        }
        window.parent.postMessage({ type: 'cardly:bridge', id: id, cmd: cmd }, window.location.origin);
      } else {
        console.debug('[Cardly bridgeCommand]', cmd);
        if (typeof callback === 'function') callback(undefined);
      }
    };
    window.pycmd = window.bridgeCommand;
  }

  // Apply parent theme class (light / dark / auto) on load.
  try {
    const theme = localStorage.getItem('cardly-theme') || 'auto';
    const isDark = theme === 'dark' || (theme === 'auto' && window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches);
    if (isDark) {
      document.documentElement.classList.add('night-mode', 'nightMode');
      if (document.body) document.body.classList.add('night-mode', 'nightMode');
    }
  } catch (_) {}
})();
"#;

/// Built-in fallback portal served at `/` during Phase 1 (before Phase 2's Svelte 5 shell is built).
/// Provides working login/logout, sync credentials summary, and direct links to live upstream `ts/` routes.
const PHASE1_PORTAL_HTML: &str = r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Cardly</title>
  <style>
    :root {
      --bg: #f5f5f5;
      --surface: #ffffff;
      --border: #d4d4d8;
      --fg: #18181b;
      --muted: #52525b;
      --accent: #2563eb;
      --accent-hover: #1d4ed8;
      --danger: #dc2626;
    }
    @media (prefers-color-scheme: dark) {
      :root {
        --bg: #2c2c2c;
        --surface: #363636;
        --border: #4a4a4a;
        --fg: #fcfcfc;
        --muted: #a1a1aa;
        --accent: #3b82f6;
        --accent-hover: #60a5fa;
      }
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      background: var(--bg);
      color: var(--fg);
      min-height: 100vh;
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 24px;
    }
    .card {
      width: 100%;
      max-width: 440px;
      background: var(--surface);
      border: 1px solid var(--border);
      border-radius: 12px;
      padding: 28px;
      box-shadow: 0 4px 20px rgba(0,0,0,0.08);
    }
    h1 { margin: 0 0 6px; font-size: 22px; font-weight: 700; }
    p.sub { margin: 0 0 20px; color: var(--muted); font-size: 14px; }
    label { display: block; font-size: 13px; font-weight: 600; margin-bottom: 6px; }
    input {
      width: 100%;
      padding: 10px 12px;
      margin-bottom: 14px;
      border-radius: 6px;
      border: 1px solid var(--border);
      background: var(--bg);
      color: var(--fg);
      font-size: 14px;
    }
    button {
      width: 100%;
      padding: 10px 14px;
      border: none;
      border-radius: 6px;
      background: var(--accent);
      color: #fff;
      font-size: 14px;
      font-weight: 600;
      cursor: pointer;
    }
    button:hover { background: var(--accent-hover); }
    button.secondary {
      background: transparent;
      color: var(--fg);
      border: 1px solid var(--border);
      margin-top: 10px;
    }
    .err { color: var(--danger); font-size: 13px; margin-bottom: 12px; min-height: 18px; }
    .links { display: grid; gap: 8px; margin: 16px 0; }
    .links a {
      display: block;
      padding: 10px 12px;
      border-radius: 6px;
      border: 1px solid var(--border);
      color: var(--fg);
      text-decoration: none;
      font-size: 14px;
    }
    .links a:hover { border-color: var(--accent); color: var(--accent); }
    .badge {
      display: inline-block;
      padding: 2px 8px;
      border-radius: 999px;
      font-size: 12px;
      background: rgba(37, 99, 235, 0.14);
      color: var(--accent);
      margin-bottom: 12px;
    }
    .hidden { display: none; }
  </style>
</head>
<body>
  <div class="card">
    <div class="badge">Cardly Backend • Phase 1</div>
    <h1>Cardly</h1>
    <div id="login-view">
      <p class="sub">Sign in to access your spaced repetition collection.</p>
      <div id="err" class="err"></div>
      <form id="login-form">
        <label for="username">Username</label>
        <input id="username" name="username" autocomplete="username" required />
        <label for="password">Password</label>
        <input id="password" name="password" type="password" autocomplete="current-password" required />
        <button type="submit">Sign In</button>
      </form>
    </div>
    <div id="authed-view" class="hidden">
      <p class="sub">Signed in as <strong id="whoami"></strong>. Upstream web modules served directly by Cardly Rust backend:</p>
      <div class="links">
        <a href="/graphs">Statistics (upstream ts/routes/graphs)</a>
        <a href="/deck-options/1">Deck Options &amp; FSRS (upstream ts/routes/deck-options/1)</a>
        <a href="/congrats">Congrats Screen (upstream ts/routes/congrats)</a>
        <a href="/editor/?mode=add">Note Editor (upstream ts/routes/editor)</a>
      </div>
      <button id="logout-btn" class="secondary">Sign Out</button>
    </div>
  </div>
  <script>
    const loginView = document.getElementById('login-view');
    const authedView = document.getElementById('authed-view');
    const errEl = document.getElementById('err');
    const whoami = document.getElementById('whoami');

    async function refreshAuth() {
      const r = await fetch('/api/auth/me', { credentials: 'same-origin' });
      if (r.ok) {
        const data = await r.json();
        window.__CARDLY_API_TOKEN__ = data.api_token;
        whoami.textContent = data.username;
        loginView.classList.add('hidden');
        authedView.classList.remove('hidden');
      } else {
        loginView.classList.remove('hidden');
        authedView.classList.add('hidden');
      }
    }

    document.getElementById('login-form').addEventListener('submit', async (e) => {
      e.preventDefault();
      errEl.textContent = '';
      const username = document.getElementById('username').value;
      const password = document.getElementById('password').value;
      const r = await fetch('/api/auth/login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        credentials: 'same-origin',
        body: JSON.stringify({ username, password }),
      });
      if (!r.ok) {
        const d = await r.json().catch(() => ({}));
        errEl.textContent = d.error || 'Login failed';
        return;
      }
      await refreshAuth();
    });

    document.getElementById('logout-btn').addEventListener('click', async () => {
      await fetch('/api/auth/logout', { method: 'POST', credentials: 'same-origin' });
      await refreshAuth();
    });

    refreshAuth();
  </script>
</body>
</html>
"##;

fn serve_builtin_web_file(
    state: &AppState,
    rel_path: &str,
    sveltekit_route: Option<&str>,
) -> Response {
    let immutable = rel_path.contains("immutable");
    let effective_path = if rel_path.starts_with("sveltekit/") && !immutable {
        "sveltekit/index.html"
    } else {
        rel_path
    };

    let Some(full_path) = ensure_safe_path(&state.config.upstream_web_dir, effective_path) else {
        return (StatusCode::NOT_FOUND, format!("Not found: {effective_path}")).into_response();
    };
    let Ok(mut data) = std::fs::read(&full_path) else {
        return (StatusCode::NOT_FOUND, "File not found").into_response();
    };

    let mime = mime_for_path(effective_path);
    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime);

    if immutable {
        builder = builder.header(header::CACHE_CONTROL, "public, max-age=31536000, immutable");
    }

    if let Some(route) = sveltekit_route {
        let is_index = effective_path.ends_with("index.html");
        if is_index {
            // Inject Cardly's external bridge script (allowed by `script-src 'self'`).
            data = inject_bridge_script(&data);
            if is_untrusted_sveltekit_route(route) {
                let hash = extract_sveltekit_csp_hash(&data);
                let mut script_src = "'self'".to_string();
                if let Some(h) = hash {
                    script_src.push(' ');
                    script_src.push_str(&h);
                }
                let csp = format!(
                    "script-src {script_src}; form-action 'none'; frame-ancestors 'self'"
                );
                // Strip meta CSP tag so our HTTP CSP header controls policy cleanly.
                data = strip_csp_meta(&data);
                builder = builder.header(header::CONTENT_SECURITY_POLICY, csp);
            } else {
                data = strip_csp_meta(&data);
                builder = builder.header(header::CONTENT_SECURITY_POLICY, "frame-ancestors 'self'");
            }
        }
    } else if mime.starts_with("text/html") {
        builder = builder.header(header::CONTENT_SECURITY_POLICY, "frame-ancestors 'self'");
    }

    builder.body(Body::from(data)).unwrap()
}

fn extract_sveltekit_csp_hash(html: &[u8]) -> Option<String> {
    let caps = sveltekit_csp_meta_re().captures(html)?;
    let hash = caps.get(1)?;
    std::str::from_utf8(hash.as_bytes()).ok().map(str::to_string)
}

fn strip_csp_meta(html: &[u8]) -> Vec<u8> {
    sveltekit_csp_meta_re().replace_all(html, &b""[..]).into_owned()
}

fn inject_bridge_script(html: &[u8]) -> Vec<u8> {
    const TAG: &[u8] = b"<link rel=\"stylesheet\" href=\"/_anki/cardly-theme.css\"><script src=\"/_anki/cardly-bridge.js\"></script>";
    if let Some(pos) = find_subslice(html, b"<head>") {
        let insert_at = pos + b"<head>".len();
        let mut out = Vec::with_capacity(html.len() + TAG.len());
        out.extend_from_slice(&html[..insert_at]);
        out.extend_from_slice(TAG);
        out.extend_from_slice(&html[insert_at..]);
        out
    } else {
        let mut out = Vec::with_capacity(html.len() + TAG.len());
        out.extend_from_slice(TAG);
        out.extend_from_slice(html);
        out
    }
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Minimal stylesheet injected into upstream SvelteKit pages to replicate Qt's system font
/// setup (`add_dynamic_styling_and_props_then_show` in `qt/aqt/webview.py`).
pub const CARDLY_THEME_CSS: &str = r#":root {
  --bs-body-font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  --bs-font-sans-serif: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
}
html, body {
  font-family: var(--bs-body-font-family);
}
"#;

/// Serves `/_anki/cardly-theme.css`.
pub async fn cardly_theme_css_handler() -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/css; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from(CARDLY_THEME_CSS))
        .unwrap()
}

/// Serves `/_anki/cardly-bridge.js`.
pub async fn cardly_bridge_js_handler() -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/javascript; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from(CARDLY_BRIDGE_JS))
        .unwrap()
}

/// Serves `/` (either the compiled Phase 2 Svelte 5 shell if `CARDLY_SHELL_DIR` exists,
/// or the built-in Phase 1 login/portal page).
pub async fn index_handler(State(state): State<Arc<AppState>>) -> Response {
    if let Some(shell_dir) = &state.config.shell_dir {
        if let Some(index_path) = ensure_safe_path(shell_dir, "index.html") {
            if let Ok(bytes) = tokio::fs::read(&index_path).await {
                return Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                    .header(header::CONTENT_SECURITY_POLICY, "frame-ancestors 'self'")
                    .body(Body::from(bytes))
                    .unwrap();
            }
        }
    }
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Body::from(PHASE1_PORTAL_HTML))
        .unwrap()
}

/// Catch-all GET handler routing:
/// 1. Upstream SvelteKit routes (`/graphs`, `/deck-options/*`, `/editor/*`, etc.)
/// 2. Upstream SvelteKit static chunks (`/_app/*`)
/// 3. Upstream built-in web assets (`/_anki/*`, with legacy `.css`/`.js` remapping like `mediasrv.py`)
/// 4. Cardly shell static assets (if `CARDLY_SHELL_DIR` is configured)
/// 5. Authenticated per-user collection media files (`/<filename>` with `UNTRUSTED_MEDIA_CSP`).
pub async fn fallback_get_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    uri: axum::http::Uri,
) -> Response {
    let raw_path = uri.path().trim_start_matches('/');
    let decoded_path = percent_decode(raw_path);

    // 1. Upstream SvelteKit route? (Requires login so user pages don't fail with 401 loops.)
    if let Some(route) = detect_sveltekit_route(&decoded_path) {
        if authenticate_request(&state, &headers).is_none() {
            return Response::builder()
                .status(StatusCode::FOUND)
                .header(header::LOCATION, "/")
                .body(Body::empty())
                .unwrap();
        }
        return serve_builtin_web_file(&state, "sveltekit/index.html", Some(route));
    }

    // 2. Upstream SvelteKit immutable chunks (`/_app/...`).
    if let Some(rest) = decoded_path.strip_prefix("_app/") {
        return serve_builtin_web_file(&state, &format!("sveltekit/_app/{rest}"), None);
    }

    // 3. Upstream `/_anki/...` assets (matching `mediasrv.py` remapping).
    if let Some(rest) = decoded_path.strip_prefix("_anki/") {
        let remapped = remap_legacy_anki_asset(rest);
        return serve_builtin_web_file(&state, &remapped, None);
    }

    // 4. Cardly shell assets (e.g. `/assets/*`, `/manifest.webmanifest`, `/sw.js`, `/icons/*`).
    if let Some(shell_dir) = &state.config.shell_dir {
        if let Some(full) = ensure_safe_path(shell_dir, &decoded_path) {
            if let Ok(bytes) = tokio::fs::read(&full).await {
                let mime = mime_for_path(&decoded_path);
                return Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, mime)
                    .body(Body::from(bytes))
                    .unwrap();
            }
        }
    }

    // 5. Per-user collection media file (`/<filename>`).
    serve_user_media(&state, &headers, &decoded_path).await
}

fn remap_legacy_anki_asset(rest: &str) -> String {
    if !rest.contains('/') {
        if rest.ends_with(".css") {
            return format!("css/{rest}");
        }
        if rest.ends_with(".js") {
            let base = rest.trim_end_matches(".js");
            if matches!(base, "jquery" | "jquery-ui" | "plot") {
                return format!("js/vendor/{rest}");
            }
            return format!("js/{rest}");
        }
    } else if rest == "js/vendor/jquery.js" {
        return "js/vendor/jquery.min.js".to_string();
    } else if rest == "js/vendor/jquery-ui.js" {
        return "js/vendor/jquery-ui.min.js".to_string();
    }
    rest.to_string()
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(s) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(val) = u8::from_str_radix(s, 16) {
                    out.push(val);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
