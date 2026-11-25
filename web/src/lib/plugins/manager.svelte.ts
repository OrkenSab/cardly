import {
  deleteExternalPlugin,
  fetchHeatmapStats,
  fetchUserPlugins,
  saveExternalPlugin,
  savePluginState,
  type StoredPluginStateDto,
} from "../api";
import { store } from "../store.svelte";
import {
  MAX_EXTERNAL_PLUGINS,
  MAX_PLUGIN_BYTES,
  validatePluginManifest,
} from "./package";
import type {
  ActiveSlotMount,
  CardlyPlugin,
  HeatmapStats,
  PluginAppState,
  PluginCleanup,
  PluginEventMap,
  PluginInfo,
  PluginManifest,
  PluginPermission,
  PluginSDK,
  PluginSlotId,
  PluginSlotRenderer,
} from "./types";

const SLOT_PERMISSION: Readonly<Record<PluginSlotId, PluginPermission>> = {
  "decks:top": "ui:decks",
  "decks:bottom": "ui:decks",
  "reviewer:actions": "ui:reviewer",
  "card:after": "ui:reviewer",
};

const EVENT_PERMISSION: Readonly<Record<keyof PluginEventMap, PluginPermission>> = {
  "card:shown": "events:cards",
  "card:answered": "events:cards",
  "decks:updated": "events:app",
  "sync:completed": "events:app",
};

const ENABLED_STORAGE_KEY = "cardly:plugins:enabled";
const PLUGIN_STORAGE_PREFIX = "cardly:plugin:";
const CONFIG_STORAGE_KEY = "__config__";

const ALL_SLOTS: readonly PluginSlotId[] = [
  "decks:top",
  "decks:bottom",
  "reviewer:actions",
  "card:after",
];

interface PluginEntry {
  plugin: CardlyPlugin;
  enabled: boolean;
  loaded: boolean;
  lastError: string | null;
  externalCode: string | null;
}

interface ListenerRecord {
  pluginId: string;
  handler: (payload: never) => void;
}

interface PluginRuntime {
  unsubs: Set<PluginCleanup>;
  mounts: Map<PluginSlotId, Map<number, PluginSlotRenderer>>;
  configListeners: Set<(config: Record<string, unknown>) => void>;
}

function createInitialSlotRevisions(): Record<PluginSlotId, number> {
  return {
    "decks:top": 0,
    "decks:bottom": 0,
    "reviewer:actions": 0,
    "card:after": 0,
  };
}

function readEnabledMap(): Record<string, boolean> {
  if (typeof localStorage === "undefined") return {};
  try {
    const raw = localStorage.getItem(ENABLED_STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw);
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
      return parsed as Record<string, boolean>;
    }
  } catch {
    // Ignore corrupted storage
  }
  return {};
}

function writeEnabledMap(map: Record<string, boolean>): void {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(ENABLED_STORAGE_KEY, JSON.stringify(map));
  } catch (err) {
    console.warn("[Cardly Plugins] Failed to persist plugin states:", err);
  }
}

export class PluginManager {
  private entries = $state<PluginEntry[]>([]);
  private slotRevisions = $state<Record<PluginSlotId, number>>(createInitialSlotRevisions());
  private mountsVersion = $state<number>(0);

  private initialized = false;
  private nextMountId = 1;
  private enabledMap: Record<string, boolean> = readEnabledMap();
  private runtimes = new Map<string, PluginRuntime>();
  private listeners = new Map<keyof PluginEventMap, Set<ListenerRecord>>();
  private stateSyncTimers = new Map<string, number>();

  get plugins(): PluginInfo[] {
    return this.entries.map((e) => ({
      manifest: e.plugin.manifest,
      enabled: e.enabled,
      loaded: e.loaded,
      hasConfig: e.plugin.defaultConfig !== undefined,
      code: e.externalCode,
      lastError: e.lastError,
    }));
  }

  register(plugin: CardlyPlugin): void {
    const id = plugin.manifest.id;
    if (!id) {
      console.warn("[Cardly Plugins] Ignored plugin with empty manifest.id");
      return;
    }
    if (this.entries.some((e) => e.plugin.manifest.id === id)) {
      return;
    }

    const persisted = this.enabledMap[id];
    const enabled = typeof persisted === "boolean" ? persisted : Boolean(plugin.manifest.defaultEnabled);

    const entry: PluginEntry = {
      plugin,
      enabled,
      loaded: false,
      lastError: null,
      externalCode: null,
    };
    this.entries = [...this.entries, entry];

    if (this.initialized && entry.enabled) {
      void this.loadEntry(entry);
    }
  }

  registerAll(plugins: readonly CardlyPlugin[]): void {
    for (const plugin of plugins) {
      this.register(plugin);
    }
  }

  init(): void {
    if (this.initialized) return;
    this.initialized = true;
    for (const entry of this.entries) {
      if (entry.enabled && !entry.loaded) {
        void this.loadEntry(entry);
      }
    }
  }

  private async loadExternalModule(
    code: string,
    manifestOverride?: PluginManifest
  ): Promise<CardlyPlugin> {
    const blob = new Blob([code], { type: "text/javascript" });
    const blobUrl = URL.createObjectURL(blob);
    try {
      const mod = (await import(/* @vite-ignore */ blobUrl)) as Record<string, unknown>;
      const exported =
        mod.default && typeof mod.default === "object"
          ? (mod.default as Record<string, unknown>)
          : mod;

      const onLoad = exported.onLoad ?? mod.onLoad;
      if (typeof onLoad !== "function") {
        throw new Error("Plugin must export an `onLoad(sdk)` function.");
      }

      const onUnload =
        typeof (exported.onUnload ?? mod.onUnload) === "function"
          ? ((exported.onUnload ?? mod.onUnload) as CardlyPlugin["onUnload"])
          : undefined;

      const rawManifest = manifestOverride ?? exported.manifest ?? mod.manifest;
      const manifest = validatePluginManifest(rawManifest);

      const defaultConfig =
        (exported.defaultConfig as Record<string, unknown> | undefined) ??
        (mod.defaultConfig as Record<string, unknown> | undefined);

      return {
        manifest: { ...manifest, builtIn: false },
        defaultConfig,
        onLoad: onLoad as CardlyPlugin["onLoad"],
        onUnload,
      };
    } finally {
      URL.revokeObjectURL(blobUrl);
    }
  }

  async installFromCode(
    rawManifest: PluginManifest,
    code: string,
    options?: { enable?: boolean; skipServerSave?: boolean }
  ): Promise<PluginInfo> {
    const manifest = validatePluginManifest(rawManifest);
    if (new TextEncoder().encode(code).length > MAX_PLUGIN_BYTES) {
      throw new Error("Plugin exceeds maximum allowed size of 2 MiB.");
    }

    const existing = this.entries.find((e) => e.plugin.manifest.id === manifest.id);
    if (existing?.plugin.manifest.builtIn) {
      throw new Error(`Cannot overwrite built-in plugin "${manifest.id}".`);
    }

    if (!existing) {
      const extCount = this.entries.filter((e) => !e.plugin.manifest.builtIn).length;
      if (extCount >= MAX_EXTERNAL_PLUGINS) {
        throw new Error(`Maximum of ${MAX_EXTERNAL_PLUGINS} external plugins reached.`);
      }
    }

    const plugin = await this.loadExternalModule(code, manifest);

    if (!options?.skipServerSave && store.authStatus === "authenticated") {
      await saveExternalPlugin(
        plugin.manifest.id,
        plugin.manifest as unknown as Record<string, unknown>,
        code
      );
    }

    if (existing) {
      await this.unloadEntry(existing);
      const nextEnabled =
        options?.enable !== undefined ? options.enable : existing.enabled;
      existing.plugin = plugin;
      existing.externalCode = code;
      existing.enabled = nextEnabled;
      existing.lastError = null;
      this.enabledMap[plugin.manifest.id] = nextEnabled;
      writeEnabledMap(this.enabledMap);
      this.entries = [...this.entries];

      if (this.initialized && existing.enabled) {
        await this.loadEntry(existing);
      }
      if (!options?.skipServerSave) {
        this.scheduleStateSync(plugin.manifest.id, true);
      }
      return this.plugins.find((p) => p.manifest.id === plugin.manifest.id)!;
    }

    const enabled =
      options?.enable !== undefined
        ? options.enable
        : typeof this.enabledMap[plugin.manifest.id] === "boolean"
          ? this.enabledMap[plugin.manifest.id]
          : Boolean(plugin.manifest.defaultEnabled ?? true);

    const entry: PluginEntry = {
      plugin,
      enabled,
      loaded: false,
      lastError: null,
      externalCode: code,
    };
    this.enabledMap[plugin.manifest.id] = enabled;
    writeEnabledMap(this.enabledMap);
    this.entries = [...this.entries, entry];

    if (this.initialized && entry.enabled) {
      await this.loadEntry(entry);
    }
    if (!options?.skipServerSave) {
      this.scheduleStateSync(plugin.manifest.id, true);
    }

    return this.plugins.find((p) => p.manifest.id === plugin.manifest.id)!;
  }

  async unregister(pluginId: string): Promise<void> {
    const entry = this.entries.find((e) => e.plugin.manifest.id === pluginId);
    if (!entry) return;
    if (entry.plugin.manifest.builtIn) {
      throw new Error("Built-in plugins cannot be deleted.");
    }

    const timer = this.stateSyncTimers.get(pluginId);
    if (timer !== undefined) {
      clearTimeout(timer);
      this.stateSyncTimers.delete(pluginId);
    }

    await this.unloadEntry(entry);
    this.entries = this.entries.filter((e) => e.plugin.manifest.id !== pluginId);
    delete this.enabledMap[pluginId];
    writeEnabledMap(this.enabledMap);
    this.clearPluginLocalStorage(pluginId);

    if (store.authStatus === "authenticated") {
      await deleteExternalPlugin(pluginId);
    }
  }

  async resetExternalPlugins(): Promise<void> {
    const external = this.entries.filter((e) => !e.plugin.manifest.builtIn);
    for (const entry of external) {
      await this.unloadEntry(entry);
    }
    this.entries = this.entries.filter((e) => Boolean(e.plugin.manifest.builtIn));
  }

  async syncFromServer(): Promise<void> {
    if (store.authStatus !== "authenticated") return;
    try {
      const data = await fetchUserPlugins();

      // Apply server-stored states (enabled, config, storage) into local state
      for (const [pluginId, st] of Object.entries(data.states ?? {})) {
        if (typeof st.enabled === "boolean") {
          this.enabledMap[pluginId] = st.enabled;
        }
        if (typeof localStorage !== "undefined") {
          const prefix = `${PLUGIN_STORAGE_PREFIX}${pluginId}:`;
          if (st.config !== undefined) {
            if (st.config === null) {
              localStorage.removeItem(`${prefix}${CONFIG_STORAGE_KEY}`);
            } else {
              localStorage.setItem(`${prefix}${CONFIG_STORAGE_KEY}`, JSON.stringify(st.config));
            }
          }
          if (st.storage && typeof st.storage === "object") {
            for (const [k, v] of Object.entries(st.storage)) {
              localStorage.setItem(`${prefix}${k}`, JSON.stringify(v));
            }
          }
        }
      }
      writeEnabledMap(this.enabledMap);

      // Unload external plugins that are no longer on server for this user
      const serverIds = new Set(
        data.plugins
          .map((p) => (typeof p.manifest?.id === "string" ? p.manifest.id : ""))
          .filter(Boolean)
      );
      for (const entry of [...this.entries]) {
        if (!entry.plugin.manifest.builtIn && !serverIds.has(entry.plugin.manifest.id)) {
          await this.unloadEntry(entry);
          this.entries = this.entries.filter(
            (e) => e.plugin.manifest.id !== entry.plugin.manifest.id
          );
        }
      }

      // Sync built-in plugins with server state (enabled & live reload if needed)
      for (const entry of this.entries) {
        if (!entry.plugin.manifest.builtIn) continue;
        const id = entry.plugin.manifest.id;
        const st = data.states?.[id];
        const targetEnabled =
          typeof st?.enabled === "boolean"
            ? st.enabled
            : typeof this.enabledMap[id] === "boolean"
              ? this.enabledMap[id]
              : Boolean(entry.plugin.manifest.defaultEnabled);

        if (targetEnabled !== entry.enabled) {
          entry.enabled = targetEnabled;
          if (!targetEnabled) {
            await this.unloadEntry(entry);
          } else if (this.initialized) {
            await this.loadEntry(entry);
          }
        } else if (entry.enabled && entry.loaded && st) {
          // Reload built-in plugin so synced storage/config takes effect immediately
          await this.unloadEntry(entry);
          if (this.initialized) {
            await this.loadEntry(entry);
          }
        }
      }

      // Install/reload external plugins from server
      for (const item of data.plugins) {
        try {
          const manifest = validatePluginManifest(item.manifest);
          const st = data.states?.[manifest.id];
          const enabled =
            typeof st?.enabled === "boolean"
              ? st.enabled
              : typeof this.enabledMap[manifest.id] === "boolean"
                ? this.enabledMap[manifest.id]
                : Boolean(manifest.defaultEnabled ?? true);

          await this.installFromCode(manifest, item.code, {
            enable: enabled,
            skipServerSave: true,
          });
        } catch (err) {
          console.error("[Cardly Plugins] Failed to load server plugin:", err);
        }
      }
    } catch (err) {
      // Soft fallback to localStorage if server is temporarily unavailable
      console.warn("[Cardly Plugins] Server sync skipped (using local state):", err);
    }
  }

  isEnabled(pluginId: string): boolean {
    return this.entries.find((e) => e.plugin.manifest.id === pluginId)?.enabled ?? false;
  }

  async enable(pluginId: string): Promise<void> {
    const entry = this.entries.find((e) => e.plugin.manifest.id === pluginId);
    if (!entry || entry.enabled) return;

    entry.enabled = true;
    entry.lastError = null;
    this.enabledMap[pluginId] = true;
    writeEnabledMap(this.enabledMap);

    if (this.initialized && !entry.loaded) {
      await this.loadEntry(entry);
    }
    this.scheduleStateSync(pluginId, true);
  }

  async disable(pluginId: string): Promise<void> {
    const entry = this.entries.find((e) => e.plugin.manifest.id === pluginId);
    if (!entry || !entry.enabled) return;

    entry.enabled = false;
    this.enabledMap[pluginId] = false;
    writeEnabledMap(this.enabledMap);

    await this.unloadEntry(entry);
    this.scheduleStateSync(pluginId, true);
  }

  async toggle(pluginId: string): Promise<void> {
    if (this.isEnabled(pluginId)) {
      await this.disable(pluginId);
    } else {
      await this.enable(pluginId);
    }
  }

  emit<K extends keyof PluginEventMap>(event: K, payload: PluginEventMap[K]): void {
    const set = this.listeners.get(event);
    if (!set || set.size === 0) return;

    for (const record of Array.from(set)) {
      try {
        (record.handler as (p: PluginEventMap[K]) => void)(payload);
      } catch (err) {
        console.error(`[Cardly Plugin:${record.pluginId}] Error in "${event}" handler:`, err);
      }
    }
  }

  getSlotMounts(slotId: PluginSlotId): ActiveSlotMount[] {
    void this.mountsVersion;
    const out: ActiveSlotMount[] = [];
    for (const entry of this.entries) {
      if (!entry.enabled || !entry.loaded) continue;
      const runtime = this.runtimes.get(entry.plugin.manifest.id);
      const slotMap = runtime?.mounts.get(slotId);
      if (!slotMap) continue;
      for (const [mountId, render] of slotMap) {
        out.push({
          pluginId: entry.plugin.manifest.id,
          mountId,
          render,
        });
      }
    }
    return out;
  }

  getSlotRevision(slotId: PluginSlotId): number {
    return this.slotRevisions[slotId];
  }

  getConfig(pluginId: string): Record<string, unknown> | null {
    const entry = this.entries.find((e) => e.plugin.manifest.id === pluginId);
    if (!entry || entry.plugin.defaultConfig === undefined) return null;
    const defaults = entry.plugin.defaultConfig;
    if (typeof localStorage === "undefined") return { ...defaults };
    try {
      const raw = localStorage.getItem(`${PLUGIN_STORAGE_PREFIX}${pluginId}:${CONFIG_STORAGE_KEY}`);
      if (!raw) return { ...defaults };
      const parsed = JSON.parse(raw);
      if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
        return { ...defaults, ...(parsed as Record<string, unknown>) };
      }
    } catch {
      // ignore corrupted config
    }
    return { ...defaults };
  }

  setConfig(pluginId: string, config: Record<string, unknown>): void {
    const entry = this.entries.find((e) => e.plugin.manifest.id === pluginId);
    if (!entry || entry.plugin.defaultConfig === undefined) return;
    if (typeof localStorage !== "undefined") {
      try {
        localStorage.setItem(
          `${PLUGIN_STORAGE_PREFIX}${pluginId}:${CONFIG_STORAGE_KEY}`,
          JSON.stringify(config)
        );
      } catch (err) {
        console.warn(`[Cardly Plugin:${pluginId}] Config write error:`, err);
      }
    }
    this.notifyConfigListeners(pluginId, config);
    this.scheduleStateSync(pluginId, true);
  }

  resetConfig(pluginId: string): Record<string, unknown> | null {
    const entry = this.entries.find((e) => e.plugin.manifest.id === pluginId);
    if (!entry || entry.plugin.defaultConfig === undefined) return null;
    if (typeof localStorage !== "undefined") {
      try {
        localStorage.removeItem(`${PLUGIN_STORAGE_PREFIX}${pluginId}:${CONFIG_STORAGE_KEY}`);
      } catch {
        // ignore
      }
    }
    const defaults = { ...entry.plugin.defaultConfig };
    this.notifyConfigListeners(pluginId, defaults);
    this.scheduleStateSync(pluginId, true);
    return defaults;
  }

  getPluginStorageEntries(pluginId: string): Array<{ key: string; value: string }> {
    if (typeof localStorage === "undefined") return [];
    const prefix = `${PLUGIN_STORAGE_PREFIX}${pluginId}:`;
    const out: Array<{ key: string; value: string }> = [];
    for (let i = 0; i < localStorage.length; i++) {
      const fullKey = localStorage.key(i);
      if (!fullKey || !fullKey.startsWith(prefix)) continue;
      out.push({
        key: fullKey.slice(prefix.length),
        value: localStorage.getItem(fullKey) ?? "",
      });
    }
    out.sort((a, b) => a.key.localeCompare(b.key));
    return out;
  }

  private clearPluginLocalStorage(pluginId: string): void {
    if (typeof localStorage === "undefined") return;
    const prefix = `${PLUGIN_STORAGE_PREFIX}${pluginId}:`;
    const keys: string[] = [];
    for (let i = 0; i < localStorage.length; i++) {
      const k = localStorage.key(i);
      if (k && k.startsWith(prefix)) keys.push(k);
    }
    for (const k of keys) {
      localStorage.removeItem(k);
    }
  }

  private buildStateSnapshot(pluginId: string): StoredPluginStateDto {
    const entry = this.entries.find((e) => e.plugin.manifest.id === pluginId);
    const storageMap: Record<string, unknown> = {};
    let configVal: Record<string, unknown> | null | undefined;

    if (typeof localStorage !== "undefined") {
      const prefix = `${PLUGIN_STORAGE_PREFIX}${pluginId}:`;
      for (let i = 0; i < localStorage.length; i++) {
        const fullKey = localStorage.key(i);
        if (!fullKey || !fullKey.startsWith(prefix)) continue;
        const shortKey = fullKey.slice(prefix.length);
        const raw = localStorage.getItem(fullKey);
        if (raw === null) continue;
        try {
          const parsed = JSON.parse(raw);
          if (shortKey === CONFIG_STORAGE_KEY) {
            if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
              configVal = parsed as Record<string, unknown>;
            }
          } else {
            storageMap[shortKey] = parsed;
          }
        } catch {
          // ignore non-JSON
        }
      }
    }

    if (configVal === undefined && entry?.plugin.defaultConfig !== undefined) {
      configVal = null;
    }

    return {
      enabled: entry ? entry.enabled : undefined,
      config: configVal,
      storage: storageMap,
    };
  }

  private scheduleStateSync(pluginId: string, immediate = false): void {
    if (store.authStatus !== "authenticated") return;
    const existing = this.stateSyncTimers.get(pluginId);
    if (existing !== undefined) {
      clearTimeout(existing);
      this.stateSyncTimers.delete(pluginId);
    }

    const run = () => {
      this.stateSyncTimers.delete(pluginId);
      if (store.authStatus !== "authenticated") return;
      const snap = this.buildStateSnapshot(pluginId);
      void savePluginState(pluginId, snap).catch((err) => {
        console.warn(`[Cardly Plugin:${pluginId}] State sync failed:`, err);
      });
    };

    if (immediate) {
      run();
    } else {
      const handle = window.setTimeout(run, 200);
      this.stateSyncTimers.set(pluginId, handle);
    }
  }

  private notifyConfigListeners(pluginId: string, config: Record<string, unknown>): void {
    const runtime = this.runtimes.get(pluginId);
    if (!runtime) return;
    const snapshot = { ...config };
    for (const cb of Array.from(runtime.configListeners)) {
      try {
        cb(snapshot);
      } catch (err) {
        console.error(`[Cardly Plugin:${pluginId}] Error in config.onChange handler:`, err);
      }
    }
  }

  refreshSlot(slotId?: PluginSlotId): void {
    if (slotId) {
      this.slotRevisions[slotId] += 1;
      return;
    }
    for (const id of ALL_SLOTS) {
      this.slotRevisions[id] += 1;
    }
  }

  private getOrCreateRuntime(pluginId: string): PluginRuntime {
    let runtime = this.runtimes.get(pluginId);
    if (!runtime) {
      runtime = {
        unsubs: new Set(),
        mounts: new Map(),
        configListeners: new Set(),
      };
      this.runtimes.set(pluginId, runtime);
    }
    return runtime;
  }

  private cleanupRuntime(pluginId: string): void {
    const runtime = this.runtimes.get(pluginId);
    if (!runtime) return;

    for (const unsub of Array.from(runtime.unsubs)) {
      try {
        unsub();
      } catch (err) {
        console.error(`[Cardly Plugin:${pluginId}] Cleanup error:`, err);
      }
    }
    runtime.unsubs.clear();
    runtime.configListeners.clear();

    if (runtime.mounts.size > 0) {
      runtime.mounts.clear();
      this.mountsVersion += 1;
    }

    this.runtimes.delete(pluginId);
  }

  private requirePermission(
    pluginId: string,
    granted: ReadonlySet<PluginPermission>,
    permission: PluginPermission,
    action: string
  ): void {
    if (granted.has(permission)) return;
    const msg = `Permission "${permission}" is required to ${action}.`;
    console.error(`[Cardly Plugin:${pluginId}] ${msg}`);
    const entry = this.entries.find((e) => e.plugin.manifest.id === pluginId);
    if (entry) {
      entry.lastError = msg;
    }
    throw new Error(msg);
  }

  private createSdk(plugin: CardlyPlugin): PluginSDK {
    const pluginId = plugin.manifest.id;
    const storagePrefix = `${PLUGIN_STORAGE_PREFIX}${pluginId}:`;
    const granted = new Set<PluginPermission>(plugin.manifest.permissions ?? []);

    return {
      manifest: Object.freeze({ ...plugin.manifest }),
      ui: {
        mount: (slotId: PluginSlotId, renderOrCleanup: PluginSlotRenderer): PluginCleanup => {
          this.requirePermission(pluginId, granted, SLOT_PERMISSION[slotId], `mount UI in "${slotId}"`);
          const entry = this.entries.find((e) => e.plugin.manifest.id === pluginId);
          if (!entry || !entry.enabled) {
            return () => {};
          }
          const runtime = this.getOrCreateRuntime(pluginId);
          let slotMap = runtime.mounts.get(slotId);
          if (!slotMap) {
            slotMap = new Map();
            runtime.mounts.set(slotId, slotMap);
          }
          const mountId = this.nextMountId++;
          slotMap.set(mountId, renderOrCleanup);
          this.mountsVersion += 1;
          this.slotRevisions[slotId] += 1;

          const unmount = () => {
            const currentRuntime = this.runtimes.get(pluginId);
            const currentSlotMap = currentRuntime?.mounts.get(slotId);
            if (currentSlotMap?.delete(mountId)) {
              if (currentSlotMap.size === 0) {
                currentRuntime?.mounts.delete(slotId);
              }
              this.mountsVersion += 1;
            }
          };
          runtime.unsubs.add(unmount);
          return unmount;
        },
        refresh: (slotId?: PluginSlotId) => {
          if (slotId) {
            this.requirePermission(
              pluginId,
              granted,
              SLOT_PERMISSION[slotId],
              `refresh slot "${slotId}"`
            );
          }
          this.refreshSlot(slotId);
        },
      },
      events: {
        on: <K extends keyof PluginEventMap>(
          event: K,
          handler: (payload: PluginEventMap[K]) => void
        ): PluginCleanup => {
          this.requirePermission(
            pluginId,
            granted,
            EVENT_PERMISSION[event],
            `subscribe to "${event}"`
          );
          const entry = this.entries.find((e) => e.plugin.manifest.id === pluginId);
          if (!entry || !entry.enabled) {
            return () => {};
          }
          const runtime = this.getOrCreateRuntime(pluginId);
          let set = this.listeners.get(event);
          if (!set) {
            set = new Set();
            this.listeners.set(event, set);
          }
          const record: ListenerRecord = {
            pluginId,
            handler: handler as (payload: never) => void,
          };
          set.add(record);

          const unsub = () => {
            set?.delete(record);
            if (set && set.size === 0) {
              this.listeners.delete(event);
            }
          };
          runtime.unsubs.add(unsub);
          return () => {
            unsub();
            runtime.unsubs.delete(unsub);
          };
        },
      },
      storage: {
        get: <T>(key: string, fallback: T): T => {
          this.requirePermission(pluginId, granted, "storage", "read plugin storage");
          if (typeof localStorage === "undefined") return fallback;
          try {
            const raw = localStorage.getItem(`${storagePrefix}${key}`);
            if (raw === null) return fallback;
            return JSON.parse(raw) as T;
          } catch {
            return fallback;
          }
        },
        set: <T>(key: string, value: T): void => {
          this.requirePermission(pluginId, granted, "storage", "write plugin storage");
          if (typeof localStorage === "undefined") return;
          try {
            localStorage.setItem(`${storagePrefix}${key}`, JSON.stringify(value));
            this.scheduleStateSync(pluginId, false);
          } catch (err) {
            console.warn(`[Cardly Plugin:${pluginId}] Storage write error:`, err);
          }
        },
        remove: (key: string): void => {
          this.requirePermission(pluginId, granted, "storage", "remove plugin storage keys");
          if (typeof localStorage === "undefined") return;
          try {
            localStorage.removeItem(`${storagePrefix}${key}`);
            this.scheduleStateSync(pluginId, false);
          } catch {
            // ignore
          }
        },
      },
      config: {
        get: <T extends Record<string, unknown>>(): T => {
          return (this.getConfig(pluginId) ?? {}) as T;
        },
        onChange: (handler: (config: Record<string, unknown>) => void): PluginCleanup => {
          const entry = this.entries.find((e) => e.plugin.manifest.id === pluginId);
          if (!entry || !entry.enabled) return () => {};
          const runtime = this.getOrCreateRuntime(pluginId);
          runtime.configListeners.add(handler);
          const unsub = () => {
            runtime.configListeners.delete(handler);
          };
          runtime.unsubs.add(unsub);
          return () => {
            unsub();
            runtime.unsubs.delete(unsub);
          };
        },
      },
      app: {
        getState: (): Readonly<PluginAppState> => {
          this.requirePermission(pluginId, granted, "app:read", "read app state");
          return Object.freeze({
            view: store.view,
            currentDeckId: store.currentDeckId,
            theme: store.theme,
            resolvedTheme: store.isDark ? "dark" : "light",
            isDark: store.isDark,
            minimalistMode: store.minimalistMode,
            username: store.username,
            isOnline: store.isOnline,
          });
        },
      },
      stats: {
        getHeatmap: async (options?: { deckId?: number }): Promise<HeatmapStats> => {
          this.requirePermission(pluginId, granted, "stats:read", "read review statistics");
          return await fetchHeatmapStats(options?.deckId);
        },
      },
    };
  }

  private async loadEntry(entry: PluginEntry): Promise<void> {
    const id = entry.plugin.manifest.id;
    const sdk = this.createSdk(entry.plugin);
    entry.loaded = true;
    entry.lastError = null;
    this.mountsVersion += 1;

    try {
      await entry.plugin.onLoad(sdk);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      console.error(`[Cardly Plugin:${id}] onLoad failed:`, err);
      entry.lastError = message;
      entry.loaded = false;
      this.cleanupRuntime(id);
      return;
    }

    if (!entry.enabled) {
      await this.unloadEntry(entry);
    }
  }

  private async unloadEntry(entry: PluginEntry): Promise<void> {
    const id = entry.plugin.manifest.id;
    const wasLoaded = entry.loaded;
    entry.loaded = false;

    this.cleanupRuntime(id);

    if (wasLoaded && typeof entry.plugin.onUnload === "function") {
      try {
        await entry.plugin.onUnload();
      } catch (err) {
        console.error(`[Cardly Plugin:${id}] onUnload failed:`, err);
      }
    }
  }
}

export const pluginManager = new PluginManager();
