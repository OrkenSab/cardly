// Cardly API Client & Global Fetch Interceptor for `@generated/backend`

import { getUndoStatus, redo, undo } from "@generated/backend";
import { store } from "./store.svelte";

declare global {
  interface Window {
    __CARDLY_API_TOKEN__?: string;
    __CARDLY_INTERCEPT_RPC__?: (
      method: string,
      body: BodyInit | null | undefined
    ) => Promise<Response | null>;
  }
}

let fetchInstalled = false;

/**
 * Installs a fetch wrapper in the Cardly shell so every `@generated/backend` protobuf RPC
 * automatically carries `X-Cardly-Api-Token`, sends session cookies, handles 401 expiry,
 * and triggers `store.bumpOpRevision()` whenever an `OpChanges` mutation succeeds.
 */
export function installFetchInterceptor(): void {
  if (fetchInstalled) return;
  fetchInstalled = true;

  const origFetch = window.fetch.bind(window);
  window.fetch = async (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
    const url = typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
    const isAnkiRpc = url.startsWith("/_anki/") && String(init?.method ?? "GET").toUpperCase() === "POST";
    const isCardlyApi = url.startsWith("/api/");

    if (isAnkiRpc || isCardlyApi) {
      const headers = new Headers(init?.headers ?? {});
      if (store.apiToken && !headers.has("X-Cardly-Api-Token")) {
        headers.set("X-Cardly-Api-Token", store.apiToken);
      }
      const nextInit: RequestInit = {
        ...init,
        headers,
        credentials: "same-origin",
      };
      const resp = await origFetch(input, nextInit);

      if (resp.status === 401 && !url.startsWith("/api/auth/")) {
        store.authStatus = "unauthenticated";
        store.username = "";
        store.apiToken = "";
        window.__CARDLY_API_TOKEN__ = undefined;
      }

      if (isAnkiRpc && resp.ok) {
        const opLevel = parseInt(headers.get("Anki-Op-Changes") ?? "0", 10);
        if (opLevel > 0) {
          store.bumpOpRevision();
          void refreshUndoStatus();
        }
      }
      return resp;
    }

    return origFetch(input, init);
  };
}

export async function checkAuth(): Promise<boolean> {
  try {
    const resp = await fetch("/api/auth/me", { credentials: "same-origin" });
    const data = await resp.json().catch(() => ({}));
    if (data.demo_mode !== undefined) {
      store.demoMode = Boolean(data.demo_mode);
    }
    if (!resp.ok) {
      store.authStatus = "unauthenticated";
      store.username = "";
      store.apiToken = "";
      store.isDemoUser = false;
      window.__CARDLY_API_TOKEN__ = undefined;
      return false;
    }
    store.username = data.username;
    store.apiToken = data.api_token;
    store.isDemoUser = Boolean(data.is_demo_user);
    window.__CARDLY_API_TOKEN__ = data.api_token;
    store.authStatus = "authenticated";
    void syncTimezone();
    void refreshUndoStatus();
    if (store.demoMode) {
      void refreshDemoStatus();
    }
    return true;
  } catch {
    store.authStatus = "unauthenticated";
    return false;
  }
}

export async function refreshDemoStatus(): Promise<void> {
  try {
    const resp = await fetch("/api/demo/status");
    if (!resp.ok) return;
    const data = await resp.json();
    if (data.reset_interval_minutes !== undefined) {
      store.demoResetIntervalMinutes = data.reset_interval_minutes;
    }
    if (data.resets_in_seconds !== undefined) {
      store.demoResetsInSeconds = data.resets_in_seconds;
    }
  } catch {
    // Non-fatal
  }
}

export async function login(username: string, password: string): Promise<{ ok: boolean; error?: string }> {
  const resp = await fetch("/api/auth/login", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    credentials: "same-origin",
    body: JSON.stringify({ username, password }),
  });
  if (!resp.ok) {
    const d = await resp.json().catch(() => ({}));
    return { ok: false, error: d.error || "Invalid username or password" };
  }
  const data = await resp.json();
  store.username = data.username;
  store.apiToken = data.api_token;
  store.demoMode = Boolean(data.demo_mode);
  store.isDemoUser = Boolean(data.is_demo_user);
  window.__CARDLY_API_TOKEN__ = data.api_token;
  store.authStatus = "authenticated";
  void syncTimezone();
  void refreshUndoStatus();
  return { ok: true };
}

export async function register(username: string, password: string): Promise<{ ok: boolean; error?: string }> {
  const resp = await fetch("/api/auth/register", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    credentials: "same-origin",
    body: JSON.stringify({ username, password }),
  });
  if (!resp.ok) {
    const d = await resp.json().catch(() => ({}));
    return { ok: false, error: d.error || "Registration failed" };
  }
  const data = await resp.json();
  store.username = data.username;
  store.apiToken = data.api_token;
  store.demoMode = Boolean(data.demo_mode);
  store.isDemoUser = Boolean(data.is_demo_user);
  window.__CARDLY_API_TOKEN__ = data.api_token;
  store.authStatus = "authenticated";
  void syncTimezone();
  void refreshUndoStatus();
  return { ok: true };
}

export async function continueAsGuest(): Promise<{ ok: boolean; error?: string }> {
  const resp = await fetch("/api/auth/guest", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    credentials: "same-origin",
  });
  if (!resp.ok) {
    const d = await resp.json().catch(() => ({}));
    return { ok: false, error: d.error || "Failed to start guest session" };
  }
  const data = await resp.json();
  store.username = data.username;
  store.apiToken = data.api_token;
  store.demoMode = Boolean(data.demo_mode);
  store.isDemoUser = Boolean(data.is_demo_user);
  window.__CARDLY_API_TOKEN__ = data.api_token;
  store.authStatus = "authenticated";
  void syncTimezone();
  void refreshUndoStatus();
  if (store.demoMode) {
    void refreshDemoStatus();
  }
  return { ok: true };
}

export async function logout(): Promise<void> {
  await fetch("/api/auth/logout", { method: "POST", credentials: "same-origin" });
  store.authStatus = "unauthenticated";
  store.username = "";
  store.apiToken = "";
  window.__CARDLY_API_TOKEN__ = undefined;
  store.view = "decks";
}

export async function syncTimezone(): Promise<void> {
  try {
    await fetch("/api/timezone", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ utc_offset_mins: new Date().getTimezoneOffset() }),
    });
  } catch {
    // non-fatal
  }
}

export async function refreshUndoStatus(): Promise<void> {
  if (store.authStatus !== "authenticated") return;
  try {
    const status = await getUndoStatus({}, { alertOnError: false });
    store.undoLabel = status.undo;
    store.redoLabel = status.redo;
  } catch {
    // ignore
  }
}

export async function performUndo(): Promise<void> {
  try {
    const res = await undo({}, { alertOnError: false });
    if (res.operation) {
      store.toast(`Undone: ${res.operation}`);
    } else {
      store.toast("Nothing to undo");
    }
    if (res.newStatus) {
      store.undoLabel = res.newStatus.undo;
      store.redoLabel = res.newStatus.redo;
    } else {
      void refreshUndoStatus();
    }
    store.bumpOpRevision();
  } catch (err) {
    store.toast(err instanceof Error ? err.message : "Nothing to undo", "warning");
  }
}

export async function performRedo(): Promise<void> {
  try {
    const res = await redo({}, { alertOnError: false });
    if (res.operation) {
      store.toast(`Redone: ${res.operation}`);
    } else {
      store.toast("Nothing to redo");
    }
    if (res.newStatus) {
      store.undoLabel = res.newStatus.undo;
      store.redoLabel = res.newStatus.redo;
    } else {
      void refreshUndoStatus();
    }
    store.bumpOpRevision();
  } catch (err) {
    store.toast(err instanceof Error ? err.message : "Nothing to redo", "warning");
  }
}

export interface UploadResult {
  upload_id: string;
  filename: string;
  size: number;
  kind: "apkg" | "colpkg" | "csv" | "other";
}

export async function uploadFile(file: File): Promise<UploadResult> {
  const form = new FormData();
  form.append("file", file, file.name);
  const resp = await fetch("/api/uploads", {
    method: "POST",
    body: form,
  });
  if (!resp.ok) {
    throw new Error(await resp.text());
  }
  return await resp.json();
}

export interface BrowserCellJson {
  text: string;
  is_rtl: boolean;
  elide_mode: number;
}

export interface BrowserRowJson {
  id: number;
  cells: BrowserCellJson[];
  color: number;
  font_name: string;
  font_size: number;
}

export async function fetchBrowserRowsBatch(ids: number[]): Promise<BrowserRowJson[]> {
  if (ids.length === 0) return [];
  const resp = await fetch("/api/browser/rows", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ ids }),
  });
  if (!resp.ok) {
    throw new Error(await resp.text());
  }
  const data = await resp.json();
  return data.rows ?? [];
}

export interface ExportOptionsPayload {
  format: "apkg" | "colpkg" | "notes_csv" | "cards_csv";
  deck_id?: number;
  note_ids?: number[];
  card_ids?: number[];
  include_scheduling?: boolean;
  include_deck_configs?: boolean;
  include_media?: boolean;
  legacy?: boolean;
  include_html?: boolean;
  include_tags?: boolean;
  include_deck?: boolean;
  include_notetype?: boolean;
  include_guid?: boolean;
  filename?: string;
}

export async function downloadExport(payload: ExportOptionsPayload): Promise<void> {
  const resp = await fetch("/api/export", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });
  if (!resp.ok) {
    throw new Error(await resp.text());
  }
  const disp = resp.headers.get("Content-Disposition") ?? "";
  const match = disp.match(/filename="([^"]+)"/);
  const filename = match ? match[1] : `cardly-export.${payload.format === "colpkg" ? "colpkg" : payload.format === "apkg" ? "apkg" : "txt"}`;
  const blob = await resp.blob();
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(url);
}

export interface BackupEntry {
  name: string;
  size: number;
  modified_secs: number;
}

export async function listBackups(): Promise<BackupEntry[]> {
  const resp = await fetch("/api/backups");
  if (!resp.ok) throw new Error(await resp.text());
  const data = await resp.json();
  return data.backups ?? [];
}

export async function createBackupNow(): Promise<void> {
  const resp = await fetch("/api/backups/create", { method: "POST" });
  if (!resp.ok) throw new Error(await resp.text());
}

export async function restoreBackup(name: string): Promise<void> {
  const resp = await fetch(`/api/backups/${encodeURIComponent(name)}/restore`, { method: "POST" });
  if (!resp.ok) throw new Error(await resp.text());
  store.bumpOpRevision();
}

export async function importColpkgUpload(uploadId: string): Promise<void> {
  const resp = await fetch("/api/import/colpkg", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ upload_id: uploadId }),
  });
  if (!resp.ok) throw new Error(await resp.text());
  store.bumpOpRevision();
}

export interface StoredExternalPluginDto {
  manifest: Record<string, unknown>;
  code: string;
  updated_at: number;
}

export interface StoredPluginStateDto {
  enabled?: boolean;
  config?: Record<string, unknown> | null;
  storage?: Record<string, unknown>;
}

export interface PluginsListDto {
  plugins: StoredExternalPluginDto[];
  states: Record<string, StoredPluginStateDto>;
}

export async function fetchUserPlugins(): Promise<PluginsListDto> {
  const resp = await fetch("/api/plugins");
  if (!resp.ok) throw new Error(await resp.text());
  return (await resp.json()) as PluginsListDto;
}

export async function saveExternalPlugin(
  id: string,
  manifest: Record<string, unknown>,
  code: string
): Promise<StoredExternalPluginDto> {
  const resp = await fetch(`/api/plugins/${encodeURIComponent(id)}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ manifest, code }),
  });
  if (!resp.ok) {
    const data = await resp.json().catch(() => null);
    throw new Error(data?.error || (await resp.text()) || "Failed to save plugin");
  }
  return (await resp.json()) as StoredExternalPluginDto;
}

export async function deleteExternalPlugin(id: string): Promise<void> {
  const resp = await fetch(`/api/plugins/${encodeURIComponent(id)}`, {
    method: "DELETE",
  });
  if (!resp.ok) {
    const data = await resp.json().catch(() => null);
    throw new Error(data?.error || (await resp.text()) || "Failed to delete plugin");
  }
}

export async function savePluginState(
  id: string,
  state: StoredPluginStateDto
): Promise<void> {
  const resp = await fetch(`/api/plugins/${encodeURIComponent(id)}/state`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(state),
  });
  if (!resp.ok) {
    const data = await resp.json().catch(() => null);
    throw new Error(data?.error || (await resp.text()) || "Failed to save plugin state");
  }
}

export interface HeatmapStatsDto {
  history: Record<string, number>;
  forecast: Record<string, number>;
  today: string;
  todayCount: number;
  dailyAverage: number;
  daysLearnedPercent: number;
  currentStreak: number;
  longestStreak: number;
}

export async function fetchHeatmapStats(deckId?: number): Promise<HeatmapStatsDto> {
  const query =
    typeof deckId === "number" && Number.isFinite(deckId) && deckId > 0
      ? `?deck_id=${encodeURIComponent(String(deckId))}`
      : "";
  const resp = await fetch(`/api/stats/heatmap${query}`);
  if (!resp.ok) {
    const data = await resp.json().catch(() => null);
    throw new Error(data?.error || (await resp.text()) || "Failed to fetch heatmap stats");
  }
  return (await resp.json()) as HeatmapStatsDto;
}
