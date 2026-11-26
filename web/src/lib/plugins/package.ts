import { strFromU8, strToU8, unzipSync, zipSync } from "fflate";
import {
  ALL_PLUGIN_PERMISSIONS,
  type PluginManifest,
  type PluginPermission,
  type RegistryPluginEntry,
} from "./types";

export const PLUGIN_ID_RE = /^[a-z0-9][a-z0-9-]{1,62}$/;
export const MAX_PLUGIN_BYTES = 2 * 1024 * 1024;
export const MAX_EXTERNAL_PLUGINS = 25;

export const DEFAULT_REGISTRY_URL =
  ((import.meta as unknown as { env?: Record<string, string | undefined> }).env
    ?.VITE_PLUGIN_REGISTRY_URL) ||
  "https://raw.githubusercontent.com/cardly-anki/plugins/main/registry.json";

export const CARDLY_PLUGINS_REPO_URL = "https://github.com/cardly-anki/plugins";
export const CARDLY_MARKETPLACE_URL = "https://cardly-anki.github.io/plugins/";

export function validatePermissions(raw: unknown): PluginPermission[] {
  if (raw === undefined || raw === null) {
    return [];
  }
  if (!Array.isArray(raw)) {
    throw new Error("Invalid plugin manifest: 'permissions' must be an array of strings.");
  }
  const allowed = new Set<string>(ALL_PLUGIN_PERMISSIONS);
  const out: PluginPermission[] = [];
  for (const item of raw) {
    if (typeof item !== "string") {
      throw new Error("Invalid plugin manifest: each permission must be a string.");
    }
    if (!allowed.has(item)) {
      throw new Error(`Unknown plugin permission "${item}".`);
    }
    const perm = item as PluginPermission;
    if (!out.includes(perm)) {
      out.push(perm);
    }
  }
  return out;
}

export function hasExpandedPermissions(
  prev: readonly PluginPermission[] | undefined,
  next: readonly PluginPermission[] | undefined
): boolean {
  const prevSet = new Set(prev ?? []);
  return (next ?? []).some((p) => !prevSet.has(p));
}

export function sanitizeIconUrl(raw: unknown, baseUrl = CARDLY_MARKETPLACE_URL): string | undefined {
  if (typeof raw !== "string") return undefined;
  const trimmed = raw.trim();
  if (!trimmed) return undefined;
  try {
    const resolved = new URL(trimmed, baseUrl);
    if (resolved.protocol === "https:" || resolved.protocol === "http:") {
      return resolved.toString();
    }
  } catch {
    // Ignore invalid URLs
  }
  return undefined;
}

export function validatePluginManifest(raw: unknown): PluginManifest {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
    throw new Error("Invalid plugin manifest: expected JSON object");
  }
  const obj = raw as Record<string, unknown>;
  const id = typeof obj.id === "string" ? obj.id.trim() : "";
  if (!PLUGIN_ID_RE.test(id)) {
    throw new Error(
      `Invalid plugin id "${id}". Must be 2–63 lowercase alphanumeric characters or hyphens.`
    );
  }
  const name = typeof obj.name === "string" ? obj.name.trim() : "";
  if (!name) {
    throw new Error("Plugin manifest is missing a valid 'name'");
  }
  const version = typeof obj.version === "string" ? obj.version.trim() : "";
  if (!version) {
    throw new Error("Plugin manifest is missing a valid 'version'");
  }

  const permissions = validatePermissions(obj.permissions);

  return {
    id,
    name,
    version,
    description: typeof obj.description === "string" ? obj.description : "",
    author: typeof obj.author === "string" ? obj.author : undefined,
    homepage: typeof obj.homepage === "string" ? obj.homepage : undefined,
    iconUrl: sanitizeIconUrl(obj.iconUrl),
    updateUrl: typeof obj.updateUrl === "string" ? obj.updateUrl : undefined,
    builtIn: false,
    defaultEnabled: typeof obj.defaultEnabled === "boolean" ? obj.defaultEnabled : true,
    configHelp: typeof obj.configHelp === "string" ? obj.configHelp : undefined,
    permissions,
  };
}

export function compareSemver(a: string, b: string): number {
  const parse = (v: string) =>
    v
      .trim()
      .replace(/^v/i, "")
      .split("-")[0]
      .split(".")
      .map((n) => parseInt(n, 10) || 0);

  const pa = parse(a);
  const pb = parse(b);
  const len = Math.max(pa.length, pb.length, 3);
  for (let i = 0; i < len; i++) {
    const da = pa[i] ?? 0;
    const db = pb[i] ?? 0;
    if (da > db) return 1;
    if (da < db) return -1;
  }
  return 0;
}

function parseJsDocManifest(code: string): Partial<PluginManifest> {
  const out: Record<string, unknown> = {};
  const tagRe = /@(id|name|version|description|author|homepage|updateUrl|permissions)\s+([^\r\n*]+)/g;
  let match: RegExpExecArray | null;
  while ((match = tagRe.exec(code)) !== null) {
    const key = match[1];
    const val = match[2].trim();
    if (key === "permissions") {
      out.permissions = val
        .split(/[,\s]+/)
        .map((s) => s.trim())
        .filter(Boolean);
    } else {
      out[key] = val;
    }
  }
  return out as Partial<PluginManifest>;
}

export async function inspectModuleExports(
  code: string
): Promise<{ manifest?: unknown; hasOnLoad: boolean; defaultConfig?: Record<string, unknown> }> {
  const blob = new Blob([code], { type: "text/javascript" });
  const url = URL.createObjectURL(blob);
  try {
    const mod = (await import(/* @vite-ignore */ url)) as Record<string, unknown>;
    const def = (mod.default && typeof mod.default === "object"
      ? (mod.default as Record<string, unknown>)
      : null);
    const manifest = def?.manifest ?? mod.manifest;
    const onLoad = def?.onLoad ?? mod.onLoad;
    const defaultConfig =
      (def?.defaultConfig as Record<string, unknown> | undefined) ??
      (mod.defaultConfig as Record<string, unknown> | undefined);
    return {
      manifest,
      hasOnLoad: typeof onLoad === "function",
      defaultConfig,
    };
  } finally {
    URL.revokeObjectURL(url);
  }
}

export async function parseSingleJsPlugin(
  code: string,
  fallbackUpdateUrl?: string
): Promise<{ manifest: PluginManifest; code: string }> {
  if (new TextEncoder().encode(code).length > MAX_PLUGIN_BYTES) {
    throw new Error("Plugin code exceeds the 2 MiB limit.");
  }

  const inspected = await inspectModuleExports(code);
  if (!inspected.hasOnLoad) {
    throw new Error(
      "Invalid Cardly JS plugin: module must export `default` (or named) `onLoad(sdk)` function."
    );
  }

  const jsdoc = parseJsDocManifest(code);
  const rawManifest =
    inspected.manifest && typeof inspected.manifest === "object"
      ? { ...jsdoc, ...(inspected.manifest as Record<string, unknown>) }
      : jsdoc;

  if (fallbackUpdateUrl && !(rawManifest as Record<string, unknown>).updateUrl) {
    (rawManifest as Record<string, unknown>).updateUrl = fallbackUpdateUrl;
  }

  const manifest = validatePluginManifest(rawManifest);
  return { manifest, code };
}

export function parseZipPluginBytes(
  bytes: Uint8Array,
  fallbackUpdateUrl?: string
): { manifest: PluginManifest; code: string } {
  const files = unzipSync(bytes);
  const paths = Object.keys(files).filter((p) => !p.endsWith("/") && !p.startsWith("__MACOSX/"));

  const findEntry = (filename: string): string | undefined => {
    const exact = paths.find((p) => p === filename);
    if (exact) return exact;
    // Allow a single root folder wrapper (e.g. my-addon/manifest.json)
    const nested = paths.filter((p) => p.endsWith(`/${filename}`) && p.split("/").length === 2);
    if (nested.length === 1) return nested[0];
    return undefined;
  };

  const manifestEntry = findEntry("manifest.json");
  const codeEntry = findEntry("index.js") ?? findEntry("index.mjs");
  if (!manifestEntry || !codeEntry) {
    throw new Error("Archive must contain both `manifest.json` and `index.js`.");
  }

  const rawManifest = JSON.parse(strFromU8(files[manifestEntry])) as Record<string, unknown>;
  if (fallbackUpdateUrl && !rawManifest.updateUrl) {
    rawManifest.updateUrl = fallbackUpdateUrl;
  }
  const manifest = validatePluginManifest(rawManifest);
  const code = strFromU8(files[codeEntry]);
  if (!code.trim()) {
    throw new Error("Plugin `index.js` inside archive is empty.");
  }
  if (new TextEncoder().encode(code).length > MAX_PLUGIN_BYTES) {
    throw new Error("Plugin code exceeds the 2 MiB limit.");
  }
  return { manifest, code };
}

export async function parsePluginFile(
  file: File
): Promise<{ manifest: PluginManifest; code: string }> {
  if (file.size > MAX_PLUGIN_BYTES * 2) {
    throw new Error("Selected file is too large (max 2 MiB plugin size).");
  }
  const lower = file.name.toLowerCase();
  if (lower.endsWith(".zip") || lower.endsWith(".cardlyaddon")) {
    const buf = new Uint8Array(await file.arrayBuffer());
    const parsed = parseZipPluginBytes(buf);
    const inspected = await inspectModuleExports(parsed.code);
    if (!inspected.hasOnLoad) {
      throw new Error("`index.js` inside archive must export `onLoad(sdk)`.");
    }
    return parsed;
  }
  if (lower.endsWith(".js") || lower.endsWith(".mjs")) {
    const text = await file.text();
    return await parseSingleJsPlugin(text);
  }
  throw new Error("Unsupported file type. Choose a .zip, .cardlyaddon, .js, or .mjs plugin file.");
}

export function buildPluginZipBytes(manifest: PluginManifest, code: string): Uint8Array {
  const cleanManifest = { ...manifest };
  delete (cleanManifest as Record<string, unknown>).builtIn;
  return zipSync({
    "manifest.json": strToU8(JSON.stringify(cleanManifest, null, 2)),
    "index.js": strToU8(code),
  });
}

export async function fetchRegistryEntries(
  registryUrl = DEFAULT_REGISTRY_URL
): Promise<RegistryPluginEntry[]> {
  const resp = await fetch(registryUrl);
  if (!resp.ok) {
    throw new Error(`Registry unavailable (${resp.status} ${resp.statusText})`);
  }
  const data = (await resp.json()) as unknown;
  const normalizeEntries = (list: RegistryPluginEntry[]): RegistryPluginEntry[] =>
    list.map((entry) => ({
      ...entry,
      iconUrl: sanitizeIconUrl(entry.iconUrl),
    }));

  if (Array.isArray(data)) {
    return normalizeEntries(data as RegistryPluginEntry[]);
  }
  if (data && typeof data === "object" && Array.isArray((data as { plugins?: unknown }).plugins)) {
    return normalizeEntries((data as { plugins: RegistryPluginEntry[] }).plugins);
  }
  throw new Error("Invalid registry format: expected `{ plugins: [...] }`");
}

function isUrlLike(input: string): boolean {
  return /^https?:\/\//i.test(input) || input.startsWith("/");
}

export async function fetchPluginFromSource(
  sourceInput: string,
  registryUrl = DEFAULT_REGISTRY_URL
): Promise<{ manifest: PluginManifest; code: string }> {
  const trimmed = sourceInput.trim();
  if (!trimmed) {
    throw new Error("Please enter a plugin ID or HTTPS URL.");
  }

  if (isUrlLike(trimmed)) {
    return await fetchPluginByDirectUrl(trimmed);
  }

  const entries = await fetchRegistryEntries(registryUrl);
  const entry = entries.find((e) => e.id === trimmed);
  if (!entry) {
    throw new Error(`Plugin "${trimmed}" was not found in the Cardly registry.`);
  }

  const updateUrl = entry.updateUrl ?? registryUrl;
  if (entry.code) {
    const manifest = validatePluginManifest({
      id: entry.id,
      name: entry.name,
      version: entry.version,
      description: entry.description ?? "",
      author: entry.author,
      homepage: entry.homepage,
      iconUrl: entry.iconUrl,
      updateUrl,
      defaultEnabled: entry.defaultEnabled,
      permissions: entry.permissions,
    });
    const inspected = await inspectModuleExports(entry.code);
    if (!inspected.hasOnLoad) {
      throw new Error(`Registry plugin "${entry.id}" is missing an onLoad(sdk) export.`);
    }
    return { manifest, code: entry.code };
  }

  const targetUrl = entry.packageUrl ?? entry.codeUrl;
  if (!targetUrl) {
    throw new Error(`Registry entry "${entry.id}" has no codeUrl or packageUrl.`);
  }
  const fetched = await fetchPluginByDirectUrl(targetUrl, updateUrl);
  return {
    manifest: validatePluginManifest({
      ...fetched.manifest,
      id: entry.id,
      name: entry.name || fetched.manifest.name,
      version: entry.version || fetched.manifest.version,
      description: entry.description ?? fetched.manifest.description,
      author: entry.author ?? fetched.manifest.author,
      homepage: entry.homepage ?? fetched.manifest.homepage,
      iconUrl: entry.iconUrl ?? fetched.manifest.iconUrl,
      updateUrl,
      permissions: entry.permissions ?? fetched.manifest.permissions,
    }),
    code: fetched.code,
  };
}

export async function fetchPluginByDirectUrl(
  url: string,
  fallbackUpdateUrl?: string
): Promise<{ manifest: PluginManifest; code: string }> {
  const resp = await fetch(url);
  if (!resp.ok) {
    throw new Error(`Failed to fetch plugin (${resp.status} ${resp.statusText})`);
  }

  const lower = url.split("?")[0].toLowerCase();
  const ctype = (resp.headers.get("content-type") ?? "").toLowerCase();

  if (
    lower.endsWith(".zip") ||
    lower.endsWith(".cardlyaddon") ||
    ctype.includes("application/zip")
  ) {
    const buf = new Uint8Array(await resp.arrayBuffer());
    const parsed = parseZipPluginBytes(buf, fallbackUpdateUrl ?? url);
    const inspected = await inspectModuleExports(parsed.code);
    if (!inspected.hasOnLoad) {
      throw new Error("`index.js` inside archive must export `onLoad(sdk)`.");
    }
    return parsed;
  }

  const text = await resp.text();
  if (lower.endsWith(".json") || ctype.includes("application/json") || text.trim().startsWith("{")) {
    const json = JSON.parse(text) as Record<string, unknown>;
    if (json.manifest && typeof json.code === "string") {
      const m = validatePluginManifest({
        ...(json.manifest as Record<string, unknown>),
        updateUrl: (json.manifest as Record<string, unknown>).updateUrl ?? fallbackUpdateUrl ?? url,
      });
      const inspected = await inspectModuleExports(json.code);
      if (!inspected.hasOnLoad) {
        throw new Error("Remote plugin code must export `onLoad(sdk)`.");
      }
      return { manifest: m, code: json.code };
    }
    if (typeof json.codeUrl === "string" || typeof json.packageUrl === "string") {
      const remoteUrl = (json.packageUrl ?? json.codeUrl) as string;
      const inner = await fetchPluginByDirectUrl(remoteUrl, fallbackUpdateUrl ?? url);
      return {
        manifest: validatePluginManifest({
          ...inner.manifest,
          ...json,
          updateUrl: (json.updateUrl as string | undefined) ?? fallbackUpdateUrl ?? url,
        }),
        code: inner.code,
      };
    }
    throw new Error("JSON plugin descriptor must contain `{ manifest, code }` or `codeUrl`.");
  }

  return await parseSingleJsPlugin(text, fallbackUpdateUrl ?? url);
}
