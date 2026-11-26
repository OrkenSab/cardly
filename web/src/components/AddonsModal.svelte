<script lang="ts">
  import { untrack } from "svelte";
  import {
    buildPluginZipBytes,
    CARDLY_MARKETPLACE_URL,
    CARDLY_PLUGINS_REPO_URL,
    compareSemver,
    DEFAULT_REGISTRY_URL,
    fetchPluginByDirectUrl,
    fetchPluginFromSource,
    fetchRegistryEntries,
    hasExpandedPermissions,
    parsePluginFile,
    PLUGIN_PERMISSION_META,
    pluginManager,
    type PluginInfo,
    type PluginManifest,
    type PluginPermission,
    type RegistryPluginEntry,
  } from "../lib/plugins";
  import { store } from "../lib/store.svelte";

  type MainTab = "installed" | "marketplace";
  let activeTab = $state<MainTab>("installed");

  // Multi-selection state (matches Qt ExtendedSelection)
  let selectedIds = $state<string[]>([]);
  let anchorId = $state<string | null>(null);

  // File input for Install from file...
  let fileInputEl = $state<HTMLInputElement | null>(null);
  let installingFile = $state(false);

  // Sub-modals matching Anki's GetAddons, ConfigEditor, and View Files
  let getAddonsOpen = $state(false);
  let addonCodeInput = $state("");
  let fetchingAddons = $state(false);
  let getAddonsError = $state("");

  let checkingUpdates = $state(false);

  let configPlugin = $state<PluginInfo | null>(null);
  let configText = $state("");
  let configError = $state("");

  type FilesTab = "manifest" | "code" | "storage";
  let filesPlugin = $state<PluginInfo | null>(null);
  let filesOpen = $state(false);
  let filesTab = $state<FilesTab>("manifest");

  // Marketplace state
  let registryEntries = $state<RegistryPluginEntry[] | null>(null);
  let registryLoading = $state(false);
  let registryError = $state("");
  let marketplaceSearch = $state("");
  let marketplaceBusyId = $state<string | null>(null);

  // Permissions confirmation dialog state
  interface PendingInstallRequest {
    manifest: PluginManifest;
    code: string;
    source: string;
    isUpdate: boolean;
    previousPermissions: readonly PluginPermission[];
    resolve: (accepted: boolean) => void;
  }
  let pendingInstall = $state<PendingInstallRequest | null>(null);

  function shouldGrey(p: PluginInfo): boolean {
    return !p.enabled;
  }

  // Exact Desktop Anki sort order: enabled first (alpha), disabled second (alpha)
  const sortedPlugins = $derived.by(() => {
    const list = [...pluginManager.plugins];
    list.sort((a, b) => {
      const greyDiff = Number(shouldGrey(a)) - Number(shouldGrey(b));
      if (greyDiff !== 0) return greyDiff;
      return a.manifest.name.localeCompare(b.manifest.name, undefined, { sensitivity: "base" });
    });
    return list;
  });

  const installedById = $derived.by(() => {
    const map = new Map<string, PluginInfo>();
    for (const p of pluginManager.plugins) {
      map.set(p.manifest.id, p);
    }
    return map;
  });

  const filteredMarketplace = $derived.by(() => {
    const entries = registryEntries ?? [];
    const q = marketplaceSearch.trim().toLowerCase();
    if (!q) return entries;
    return entries.filter((e) => {
      if (e.id.toLowerCase().includes(q)) return true;
      if (e.name.toLowerCase().includes(q)) return true;
      if ((e.description ?? "").toLowerCase().includes(q)) return true;
      if ((e.author ?? "").toLowerCase().includes(q)) return true;
      if ((e.tags ?? []).some((t) => t.toLowerCase().includes(q))) return true;
      return false;
    });
  });

  const selectedPlugins = $derived(
    sortedPlugins.filter((p) => selectedIds.includes(p.manifest.id))
  );

  const singleSelected = $derived(
    selectedPlugins.length === 1 ? selectedPlugins[0] : null
  );

  const canViewPage = $derived(Boolean(singleSelected?.manifest.homepage));
  const canConfig = $derived(Boolean(singleSelected?.hasConfig));
  const canViewFiles = $derived(selectedPlugins.length <= 1);
  const canToggle = $derived(selectedPlugins.length > 0);
  const canDelete = $derived(
    selectedPlugins.length > 0 && selectedPlugins.every((p) => !p.manifest.builtIn)
  );

  function formatListName(p: PluginInfo): string {
    return p.enabled ? p.manifest.name : `${p.manifest.name} (disabled)`;
  }

  function permLabel(perm: PluginPermission): string {
    return PLUGIN_PERMISSION_META[perm]?.label ?? perm;
  }

  function permDescription(perm: PluginPermission): string {
    return PLUGIN_PERMISSION_META[perm]?.description ?? "";
  }

  async function loadMarketplace(force = false) {
    if (registryLoading) return;
    if (!force && registryEntries !== null) return;
    registryLoading = true;
    registryError = "";
    try {
      registryEntries = await fetchRegistryEntries(DEFAULT_REGISTRY_URL);
    } catch (err) {
      registryError = err instanceof Error ? err.message : String(err);
    } finally {
      registryLoading = false;
    }
  }

  $effect(() => {
    if (activeTab === "marketplace") {
      untrack(() => {
        void loadMarketplace(false);
      });
    }
  });

  $effect(() => {
    const pendingId = store.pendingInstallAddonId;
    if (pendingId) {
      untrack(() => {
        store.pendingInstallAddonId = null;
        activeTab = "marketplace";
        void (async () => {
          try {
            const { manifest, code } = await fetchPluginFromSource(pendingId);
            const info = await installWithPermissionPrompt(
              manifest,
              code,
              "Cardly Marketplace"
            );
            if (info) {
              store.toast(`Installed ${info.manifest.name} v${info.manifest.version}`);
            }
          } catch (err) {
            await store.alert(
              err instanceof Error ? err.message : String(err),
              "Add-on installation error"
            );
          }
        })();
      });
    }
  });

  function askPermissionConfirmation(
    manifest: PluginManifest,
    code: string,
    source: string,
    isUpdate: boolean,
    previousPermissions: readonly PluginPermission[]
  ): Promise<boolean> {
    return new Promise((resolve) => {
      pendingInstall = {
        manifest,
        code,
        source,
        isUpdate,
        previousPermissions,
        resolve: (accepted: boolean) => {
          pendingInstall = null;
          resolve(accepted);
        },
      };
    });
  }

  async function installWithPermissionPrompt(
    manifest: PluginManifest,
    code: string,
    source: string,
    options?: { skipPromptIfPermissionsUnchanged?: boolean }
  ): Promise<PluginInfo | null> {
    const existing = pluginManager.plugins.find((p) => p.manifest.id === manifest.id);
    if (existing?.manifest.builtIn) {
      throw new Error(`Cannot overwrite built-in plugin "${manifest.id}".`);
    }

    const isUpdate = Boolean(existing);
    const prevPerms = existing?.manifest.permissions ?? [];
    const nextPerms = manifest.permissions ?? [];
    const expanded = hasExpandedPermissions(prevPerms, nextPerms);

    const needsPrompt =
      !isUpdate ||
      !options?.skipPromptIfPermissionsUnchanged ||
      expanded;

    if (needsPrompt) {
      const accepted = await askPermissionConfirmation(
        manifest,
        code,
        source,
        isUpdate,
        prevPerms
      );
      if (!accepted) {
        return null;
      }
    }

    const info = await pluginManager.installFromCode(manifest, code, { enable: true });
    selectedIds = [info.manifest.id];
    anchorId = info.manifest.id;
    return info;
  }

  function handleItemClick(id: string, e: MouseEvent) {
    if (e.shiftKey && anchorId) {
      const ids = sortedPlugins.map((p) => p.manifest.id);
      const a = ids.indexOf(anchorId);
      const b = ids.indexOf(id);
      if (a !== -1 && b !== -1) {
        const [start, end] = a <= b ? [a, b] : [b, a];
        selectedIds = ids.slice(start, end + 1);
        return;
      }
    }
    if (e.ctrlKey || e.metaKey) {
      if (selectedIds.includes(id)) {
        selectedIds = selectedIds.filter((x) => x !== id);
      } else {
        selectedIds = [...selectedIds, id];
      }
      anchorId = id;
      return;
    }
    selectedIds = [id];
    anchorId = id;
  }

  function handleListKeydown(e: KeyboardEvent) {
    if (sortedPlugins.length === 0) return;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const ids = sortedPlugins.map((p) => p.manifest.id);
      const currentIdx = singleSelected ? ids.indexOf(singleSelected.manifest.id) : -1;
      const nextIdx =
        e.key === "ArrowDown"
          ? Math.min(ids.length - 1, currentIdx + 1)
          : Math.max(0, currentIdx === -1 ? 0 : currentIdx - 1);
      selectedIds = [ids[nextIdx]];
      anchorId = ids[nextIdx];
    } else if (e.key === "Enter" && singleSelected) {
      e.preventDefault();
      handleConfig();
    }
  }

  async function handleToggleEnabled() {
    if (selectedPlugins.length === 0) return;
    for (const p of selectedPlugins) {
      await pluginManager.toggle(p.manifest.id);
    }
  }

  function handleViewPage() {
    if (!singleSelected) {
      store.toast("Please select a single add-on first.");
      return;
    }
    const url = singleSelected.manifest.homepage;
    if (url) {
      window.open(url, "_blank", "noopener,noreferrer");
    }
  }

  function handleConfig() {
    if (!singleSelected) {
      store.toast("Please select a single add-on first.");
      return;
    }
    const conf = pluginManager.getConfig(singleSelected.manifest.id);
    if (conf === null) {
      void store.alert("Add-on has no configuration.", "Add-ons");
      return;
    }
    configPlugin = singleSelected;
    configText = JSON.stringify(conf, null, 2);
    configError = "";
  }

  function saveConfig() {
    if (!configPlugin) return;
    try {
      const parsed: unknown = JSON.parse(configText);
      if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
        configError = "Invalid configuration: top level object must be a map";
        return;
      }
      pluginManager.setConfig(configPlugin.manifest.id, parsed as Record<string, unknown>);
      store.toast(`Saved configuration for "${configPlugin.manifest.name}"`);
      configPlugin = null;
    } catch (err) {
      configError = `Invalid configuration: ${err instanceof Error ? err.message : String(err)}`;
    }
  }

  function restoreDefaultConfig() {
    if (!configPlugin) return;
    const defaults = pluginManager.resetConfig(configPlugin.manifest.id);
    if (defaults) {
      configText = JSON.stringify(defaults, null, 2);
      configError = "";
      store.toast("Restored defaults");
    }
  }

  function handleViewFiles() {
    if (selectedPlugins.length > 1) {
      store.toast("Please select a single add-on first.");
      return;
    }
    filesPlugin = singleSelected;
    filesTab = "manifest";
    filesOpen = true;
  }

  function triggerInstallFromFile() {
    if (installingFile) return;
    fileInputEl?.click();
  }

  async function handleFileSelected(ev: Event) {
    const input = ev.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;

    installingFile = true;
    try {
      const { manifest, code } = await parsePluginFile(file);
      const installed = await installWithPermissionPrompt(manifest, code, `File: ${file.name}`);
      if (installed) {
        store.toast(`Installed "${installed.manifest.name}" v${installed.manifest.version}`);
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      await store.alert(msg, "Add-on installation error");
    } finally {
      installingFile = false;
    }
  }

  async function handleMarketplaceInstall(entry: RegistryPluginEntry) {
    if (marketplaceBusyId) return;
    marketplaceBusyId = entry.id;
    try {
      const { manifest, code } = await fetchPluginFromSource(entry.id, DEFAULT_REGISTRY_URL);
      const existing = installedById.get(manifest.id);
      const info = await installWithPermissionPrompt(manifest, code, "Cardly Marketplace", {
        skipPromptIfPermissionsUnchanged: Boolean(existing),
      });
      if (info) {
        store.toast(
          existing
            ? `Updated "${info.manifest.name}" to v${info.manifest.version}`
            : `Installed "${info.manifest.name}" v${info.manifest.version}`
        );
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      await store.alert(msg, "Marketplace installation error");
    } finally {
      marketplaceBusyId = null;
    }
  }

  async function handleCheckForUpdates() {
    if (checkingUpdates) return;
    const externals = pluginManager.plugins.filter((p) => !p.manifest.builtIn);
    if (externals.length === 0) {
      store.toast("All add-ons are up to date.");
      return;
    }

    checkingUpdates = true;
    const updatedNames: string[] = [];
    const errors: string[] = [];

    try {
      let cachedRegistry: RegistryPluginEntry[] | null = null;

      for (const p of externals) {
        const updateUrl = p.manifest.updateUrl;
        try {
          let candidate: { manifest: PluginInfo["manifest"]; code: string } | null = null;

          if (!updateUrl || updateUrl === DEFAULT_REGISTRY_URL || updateUrl.endsWith("registry.json")) {
            const url = updateUrl || DEFAULT_REGISTRY_URL;
            const entries: RegistryPluginEntry[] =
              url === DEFAULT_REGISTRY_URL && cachedRegistry
                ? cachedRegistry
                : await fetchRegistryEntries(url);
            if (url === DEFAULT_REGISTRY_URL) cachedRegistry = entries;
            const match = entries.find((e) => e.id === p.manifest.id);
            if (match && compareSemver(match.version, p.manifest.version) > 0) {
              candidate = await fetchPluginFromSource(p.manifest.id, url);
            }
          } else {
            const fetched = await fetchPluginByDirectUrl(updateUrl, updateUrl);
            if (
              fetched.manifest.id === p.manifest.id &&
              compareSemver(fetched.manifest.version, p.manifest.version) > 0
            ) {
              candidate = fetched;
            }
          }

          if (candidate) {
            const installed = await installWithPermissionPrompt(
              candidate.manifest,
              candidate.code,
              updateUrl || "Cardly Registry",
              { skipPromptIfPermissionsUnchanged: true }
            );
            if (installed) {
              updatedNames.push(
                `${installed.manifest.name} (${p.manifest.version} -> ${installed.manifest.version})`
              );
            }
          }
        } catch (err) {
          errors.push(`${p.manifest.name}: ${err instanceof Error ? err.message : String(err)}`);
        }
      }

      if (updatedNames.length > 0) {
        store.toast(`Updated: ${updatedNames.join(", ")}`, "info", 4200);
      } else if (errors.length > 0) {
        store.toast(`Update check failed: ${errors[0]}`, "warning", 4200);
      } else {
        store.toast("All add-ons are up to date.");
      }
    } finally {
      checkingUpdates = false;
    }
  }

  async function handleDelete() {
    if (selectedPlugins.length === 0) return;
    if (selectedPlugins.some((p) => p.manifest.builtIn)) {
      store.toast("Built-in add-ons cannot be deleted — use Toggle Enabled to disable.", "warning");
      return;
    }
    const count = selectedPlugins.length;
    const confirmed = await store.confirm(
      `Delete the ${count} selected add-on${count === 1 ? "" : "s"}?`,
      "Delete Add-on",
      { danger: true, confirmLabel: "Delete" }
    );
    if (!confirmed) return;

    try {
      for (const p of selectedPlugins) {
        await pluginManager.unregister(p.manifest.id);
      }
      selectedIds = [];
      anchorId = null;
      store.toast(count === 1 ? "Add-on deleted" : `Deleted ${count} add-ons`);
    } catch (err) {
      store.toast(err instanceof Error ? err.message : "Failed to delete add-on", "error");
    }
  }

  async function submitGetAddons() {
    const tokens = addonCodeInput.trim().split(/\s+/).filter(Boolean);
    if (tokens.length === 0) {
      getAddonsError = "Please enter an add-on ID or URL.";
      return;
    }

    fetchingAddons = true;
    getAddonsError = "";
    const installed: string[] = [];

    try {
      for (const token of tokens) {
        const { manifest, code } = await fetchPluginFromSource(token);
        const info = await installWithPermissionPrompt(manifest, code, `Source: ${token}`);
        if (info) {
          installed.push(`${info.manifest.name} v${info.manifest.version}`);
        }
      }
      getAddonsOpen = false;
      addonCodeInput = "";
      if (installed.length > 0) {
        store.toast(`Installed ${installed.join(", ")}`);
      }
    } catch (err) {
      getAddonsError = err instanceof Error ? err.message : String(err);
    } finally {
      fetchingAddons = false;
    }
  }

  function downloadPluginBundle(p: PluginInfo) {
    let blob: Blob;
    let filename: string;
    if (p.code !== null) {
      const zipBytes = buildPluginZipBytes(p.manifest, p.code);
      const copy = new Uint8Array(zipBytes.length);
      copy.set(zipBytes);
      blob = new Blob([copy], { type: "application/zip" });
      filename = `${p.manifest.id}-${p.manifest.version}.cardlyaddon`;
    } else {
      blob = new Blob([JSON.stringify(p.manifest, null, 2)], { type: "application/json" });
      filename = `${p.manifest.id}-manifest.json`;
    }
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
  }

  function copyCode(code: string) {
    void navigator.clipboard?.writeText(code);
    store.toast("Plugin code copied");
  }

  function close() {
    store.addonsOpen = false;
  }

  function handleModalKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      if (pendingInstall) {
        e.stopPropagation();
        pendingInstall.resolve(false);
      } else if (configPlugin) {
        e.stopPropagation();
        configPlugin = null;
      } else if (getAddonsOpen) {
        e.stopPropagation();
        if (!fetchingAddons) getAddonsOpen = false;
      } else if (filesOpen) {
        e.stopPropagation();
        filesOpen = false;
      }
    }
  }
</script>

<input
  bind:this={fileInputEl}
  type="file"
  accept=".zip,.cardlyaddon,.js,.mjs"
  style="display: none"
  onchange={handleFileSelected}
/>

<div
  class="modal-backdrop"
  role="dialog"
  aria-modal="true"
  aria-label="Add-ons"
  tabindex="-1"
  onkeydown={handleModalKeydown}
>
  <div class="modal-window addons-window">
    <div class="modal-header">
      <h3 class="modal-title">Add-ons</h3>
      <button type="button" class="modal-close" aria-label="Close" onclick={close}>×</button>
    </div>

    <div class="addons-tabs" role="tablist">
      <button
        type="button"
        role="tab"
        aria-selected={activeTab === "installed"}
        class="addons-tab"
        class:active={activeTab === "installed"}
        onclick={() => (activeTab = "installed")}
      >
        Installed ({sortedPlugins.length})
      </button>
      <button
        type="button"
        role="tab"
        aria-selected={activeTab === "marketplace"}
        class="addons-tab"
        class:active={activeTab === "marketplace"}
        onclick={() => (activeTab = "marketplace")}
      >
        Marketplace
      </button>
    </div>

    {#if activeTab === "installed"}
      <div class="modal-body addons-body">
        <div class="addons-left">
          <div class="addons-notice">Changes take effect immediately.</div>

          <!-- svelte-ignore a11y_interactive_supports_focus -->
          <div
            class="addons-list"
            role="listbox"
            aria-label="Installed Add-ons"
            aria-multiselectable="true"
            tabindex="0"
            onkeydown={handleListKeydown}
          >
            {#if sortedPlugins.length === 0}
              <div class="addons-empty">No add-ons installed.</div>
            {:else}
              {#each sortedPlugins as p (p.manifest.id)}
                {@const selected = selectedIds.includes(p.manifest.id)}
                <button
                  type="button"
                  role="option"
                  aria-selected={selected}
                  class="addon-item"
                  class:selected
                  class:disabled-item={!p.enabled}
                  onclick={(e) => handleItemClick(p.manifest.id, e)}
                  ondblclick={handleConfig}
                >
                  <span class="addon-item-left">
                    {#if p.manifest.iconUrl}
                      <img src={p.manifest.iconUrl} alt="" class="addon-mini-icon" />
                    {:else}
                      <span class="addon-mini-fallback" aria-hidden="true">❖</span>
                    {/if}
                    <span class="addon-item-name">{formatListName(p)}</span>
                  </span>
                  <span class="addon-item-ver">v{p.manifest.version}</span>
                </button>
              {/each}
            {/if}
          </div>

          {#if singleSelected}
            <div class="addon-details">
              <div class="addon-details-meta">
                <strong>{singleSelected.manifest.name}</strong>
                <span>v{singleSelected.manifest.version}</span>
                {#if singleSelected.manifest.author}
                  <span>· {singleSelected.manifest.author}</span>
                {/if}
                {#if singleSelected.manifest.builtIn}
                  <span class="addon-tag">Built-in</span>
                {:else}
                  <span class="addon-tag">External</span>
                {/if}
              </div>
              <div class="addon-details-desc">{singleSelected.manifest.description}</div>
              <div class="perm-pills-row">
                {#if (singleSelected.manifest.permissions ?? []).length === 0}
                  <span class="perm-pill muted">No special permissions</span>
                {:else}
                  {#each singleSelected.manifest.permissions ?? [] as perm (perm)}
                    <span class="perm-pill" title={permDescription(perm)}>{permLabel(perm)}</span>
                  {/each}
                {/if}
              </div>
              {#if singleSelected.lastError}
                <div class="addon-details-err">Error: {singleSelected.lastError}</div>
              {/if}
            </div>
          {/if}
        </div>

        <div class="addons-actions">
          <button
            type="button"
            class="anki-btn"
            onclick={() => {
              getAddonsError = "";
              getAddonsOpen = true;
            }}
          >
            Get Add-ons...
          </button>
          <button
            type="button"
            class="anki-btn"
            disabled={installingFile}
            onclick={triggerInstallFromFile}
          >
            {installingFile ? "Installing..." : "Install from file..."}
          </button>
          <button
            type="button"
            class="anki-btn"
            disabled={checkingUpdates}
            onclick={handleCheckForUpdates}
          >
            {checkingUpdates ? "Checking..." : "Check for Updates"}
          </button>

          <div class="addons-spacer"></div>

          <button type="button" class="anki-btn" disabled={!canViewPage} onclick={handleViewPage}>
            View Add-on Page
          </button>
          <button type="button" class="anki-btn" disabled={!canConfig} onclick={handleConfig}>
            Config
          </button>
          <button type="button" class="anki-btn" disabled={!canViewFiles} onclick={handleViewFiles}>
            View Files
          </button>
          <button type="button" class="anki-btn" disabled={!canToggle} onclick={handleToggleEnabled}>
            Toggle Enabled
          </button>
          <button
            type="button"
            class="anki-btn"
            disabled={!canDelete}
            title={selectedPlugins.some((p) => p.manifest.builtIn)
              ? "Built-in add-ons cannot be deleted"
              : "Delete selected add-on(s)"}
            onclick={handleDelete}
          >
            Delete
          </button>
        </div>
      </div>
    {:else}
      <!-- Marketplace Tab -->
      <div class="modal-body mp-body">
        <div class="mp-toolbar">
          <input
            type="search"
            class="anki-input mp-search"
            placeholder="Search add-ons by name, ID, author, or tag..."
            aria-label="Search Marketplace"
            bind:value={marketplaceSearch}
          />
          <button
            type="button"
            class="anki-btn"
            disabled={registryLoading}
            onclick={() => void loadMarketplace(true)}
          >
            {registryLoading ? "Loading..." : "Refresh"}
          </button>
        </div>

        <div class="mp-list">
          {#if registryLoading && registryEntries === null}
            <div class="mp-state">Loading Cardly Marketplace catalog...</div>
          {:else if registryError}
            <div class="mp-state mp-error-box">
              <div class="mp-error-title">Could not load Marketplace catalog</div>
              <div class="mp-error-detail">{registryError}</div>
              <div class="mp-error-hint">
                You can still install add-ons directly via <strong>Get Add-ons...</strong> (URL) or <strong>Install from file...</strong>.
              </div>
              <button type="button" class="anki-btn small" onclick={() => void loadMarketplace(true)}>
                Retry
              </button>
            </div>
          {:else if filteredMarketplace.length === 0}
            <div class="mp-state">
              {marketplaceSearch.trim()
                ? "No add-ons match your search."
                : "Marketplace catalog is currently empty."}
            </div>
          {:else}
            {#each filteredMarketplace as entry (entry.id)}
              {@const existing = installedById.get(entry.id)}
              {@const hasUpdate = Boolean(existing && compareSemver(entry.version, existing.manifest.version) > 0)}
              {@const isBusy = marketplaceBusyId === entry.id}
              <div class="mp-card">
                {#if entry.iconUrl}
                  <img src={entry.iconUrl} alt="{entry.name} icon" class="mp-card-icon" />
                {:else}
                  <div class="mp-card-icon mp-card-icon-fallback" aria-hidden="true">❖</div>
                {/if}
                <div class="mp-card-main">
                  <div class="mp-card-head">
                    <span class="mp-name">{entry.name}</span>
                    <span class="addon-tag">v{entry.version}</span>
                    {#if entry.author}
                      <span class="mp-author">by {entry.author}</span>
                    {/if}
                  </div>
                  {#if entry.description}
                    <div class="mp-desc">{entry.description}</div>
                  {/if}
                  <div class="perm-pills-row">
                    {#if (entry.permissions ?? []).length === 0}
                      <span class="perm-pill muted">No special permissions</span>
                    {:else}
                      {#each entry.permissions ?? [] as perm (perm)}
                        <span class="perm-pill" title={permDescription(perm)}>{permLabel(perm)}</span>
                      {/each}
                    {/if}
                    {#each entry.tags ?? [] as tag (tag)}
                      <span class="mp-tag">#{tag}</span>
                    {/each}
                  </div>
                </div>
                <div class="mp-card-action">
                  {#if existing && !hasUpdate}
                    <button type="button" class="anki-btn small" disabled>Installed</button>
                  {:else if existing && hasUpdate}
                    <button
                      type="button"
                      class="anki-btn small primary"
                      disabled={isBusy || marketplaceBusyId !== null}
                      onclick={() => void handleMarketplaceInstall(entry)}
                    >
                      {isBusy ? "Updating..." : `Update v${entry.version}`}
                    </button>
                  {:else}
                    <button
                      type="button"
                      class="anki-btn small primary"
                      disabled={isBusy || marketplaceBusyId !== null}
                      onclick={() => void handleMarketplaceInstall(entry)}
                    >
                      {isBusy ? "Installing..." : "Install"}
                    </button>
                  {/if}
                </div>
              </div>
            {/each}
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>

<!-- Sub-dialog: Permissions Confirmation -->
{#if pendingInstall}
  {@const req = pendingInstall}
  {@const perms = req.manifest.permissions ?? []}
  <div
    class="modal-backdrop perm-backdrop"
    role="dialog"
    aria-modal="true"
    aria-label="Confirm Add-on Permissions"
  >
    <div class="modal-window perm-window">
      <div class="modal-header">
        <h3 class="modal-title">
          {req.isUpdate ? "Update Add-on — Permissions" : "Install Add-on — Permissions"}
        </h3>
        <button
          type="button"
          class="modal-close"
          aria-label="Close"
          onclick={() => req.resolve(false)}
        >
          ×
        </button>
      </div>
      <div class="modal-body perm-body">
        <div class="perm-summary">
          <div class="perm-summary-head">
            {#if req.manifest.iconUrl}
              <img src={req.manifest.iconUrl} alt="" class="perm-summary-icon" />
            {/if}
            <div class="perm-summary-head-text">
              <div class="perm-summary-title">
                <strong>{req.manifest.name}</strong>
                <span class="addon-tag">v{req.manifest.version}</span>
                {#if req.manifest.author}
                  <span class="mp-author">by {req.manifest.author}</span>
                {/if}
              </div>
              {#if req.manifest.description}
                <div class="perm-summary-desc">{req.manifest.description}</div>
              {/if}
            </div>
          </div>
          <div class="perm-source">Source: <code>{req.source}</code></div>
        </div>

        <div class="perm-section-label">
          {req.isUpdate
            ? "This update requests the following permissions:"
            : "This add-on requests the following permissions:"}
        </div>

        {#if perms.length === 0}
          <div class="perm-none-box">
            No special permissions requested. This add-on cannot access your decks UI, reviewer UI, events, storage, or app state.
          </div>
        {:else}
          <div class="perm-list">
            {#each perms as perm (perm)}
              {@const isNew = req.isUpdate && !req.previousPermissions.includes(perm)}
              <div class="perm-row">
                <div class="perm-row-head">
                  <span class="perm-pill">{permLabel(perm)}</span>
                  <code class="perm-code">{perm}</code>
                  {#if isNew}
                    <span class="perm-new-badge">New</span>
                  {/if}
                </div>
                <div class="perm-row-desc">{permDescription(perm)}</div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
      <div class="modal-footer">
        <button type="button" class="anki-btn" onclick={() => req.resolve(false)}>Cancel</button>
        <button type="button" class="anki-btn primary" onclick={() => req.resolve(true)}>
          Install Add-on
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Sub-dialog: Get Add-ons (matches aqt/forms/getaddons.ui) -->
{#if getAddonsOpen}
  <div class="modal-backdrop sub-backdrop" role="dialog" aria-modal="true" aria-label="Install Add-on">
    <div class="modal-window get-addons-window">
      <div class="modal-header">
        <h3 class="modal-title">Install Add-on</h3>
        <button
          type="button"
          class="modal-close"
          aria-label="Close"
          disabled={fetchingAddons}
          onclick={() => (getAddonsOpen = false)}
        >
          ×
        </button>
      </div>
      <div class="modal-body get-addons-body">
        <p class="get-addons-text">
          To browse Cardly JS add-ons, click <strong>Browse Add-ons</strong> below.
        </p>
        <p class="get-addons-text">
          Paste a plugin ID from the Cardly registry or a direct <code>https://</code> URL to a <code>.js</code>, <code>.json</code>, or <code>.cardlyaddon</code> plugin.
        </p>
        <p class="get-addons-hint">
          Note: Only Cardly JavaScript plugins are supported (Python add-ons from AnkiWeb are incompatible).
        </p>
        <div class="get-addons-row">
          <label for="addon-code-input">Code / URL:</label>
          <input
            id="addon-code-input"
            type="text"
            class="anki-input"
            placeholder="e.g. review-heatmap or https://..."
            disabled={fetchingAddons}
            bind:value={addonCodeInput}
            onkeydown={(e) => e.key === "Enter" && void submitGetAddons()}
          />
        </div>
        {#if getAddonsError}
          <div class="config-error">{getAddonsError}</div>
        {/if}
      </div>
      <div class="modal-footer get-addons-footer">
        <a
          class="anki-btn"
          href={CARDLY_MARKETPLACE_URL}
          target="_blank"
          rel="noopener noreferrer"
        >
          Browse Add-ons
        </a>
        <div class="footer-right">
          <button
            type="button"
            class="anki-btn"
            disabled={fetchingAddons}
            onclick={() => (getAddonsOpen = false)}
          >
            Cancel
          </button>
          <button
            type="button"
            class="anki-btn primary"
            disabled={fetchingAddons}
            onclick={() => void submitGetAddons()}
          >
            {fetchingAddons ? "Installing..." : "OK"}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- Sub-dialog: Config Editor (matches aqt/forms/addonconf.ui) -->
{#if configPlugin}
  <div
    class="modal-backdrop sub-backdrop"
    role="dialog"
    aria-modal="true"
    aria-label={`Configure '${configPlugin.manifest.name}'`}
  >
    <div class="modal-window config-window">
      <div class="modal-header">
        <h3 class="modal-title">Configure '{configPlugin.manifest.name}'</h3>
        <button type="button" class="modal-close" aria-label="Close" onclick={() => (configPlugin = null)}>×</button>
      </div>
      <div class="modal-body config-body">
        <div class="config-split">
          <div class="config-editor-pane">
            <textarea
              class="anki-textarea config-editor"
              spellcheck="false"
              bind:value={configText}
              aria-label="Configuration JSON"
            ></textarea>
            {#if configError}
              <div class="config-error">{configError}</div>
            {/if}
          </div>
          <div class="config-help-pane">
            <div class="config-help-title">Configuration</div>
            <p class="config-help-text">
              {configPlugin.manifest.configHelp ?? "Edit the JSON configuration on the left and click OK to apply."}
            </p>
          </div>
        </div>
      </div>
      <div class="modal-footer config-footer">
        <button type="button" class="anki-btn" onclick={restoreDefaultConfig}>Restore Defaults</button>
        <div class="footer-right">
          <button type="button" class="anki-btn" onclick={() => (configPlugin = null)}>Cancel</button>
          <button type="button" class="anki-btn primary" onclick={saveConfig}>OK</button>
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- Sub-dialog: View Files (Manifest / index.js / Storage / Export) -->
{#if filesOpen}
  <div class="modal-backdrop sub-backdrop" role="dialog" aria-modal="true" aria-label="Add-on Files">
    <div class="modal-window files-window">
      <div class="modal-header">
        <h3 class="modal-title">
          {filesPlugin ? `Add-on Files — ${filesPlugin.manifest.name}` : "Add-ons Folder"}
        </h3>
        <button type="button" class="modal-close" aria-label="Close" onclick={() => (filesOpen = false)}>×</button>
      </div>

      {#if filesPlugin}
        <div class="files-tabs" role="tablist">
          <button
            type="button"
            class="files-tab"
            class:active={filesTab === "manifest"}
            onclick={() => (filesTab = "manifest")}
          >
            manifest.json
          </button>
          <button
            type="button"
            class="files-tab"
            class:active={filesTab === "code"}
            onclick={() => (filesTab = "code")}
          >
            index.js
          </button>
          <button
            type="button"
            class="files-tab"
            class:active={filesTab === "storage"}
            onclick={() => (filesTab = "storage")}
          >
            Storage
          </button>
        </div>
      {/if}

      <div class="modal-body files-body">
        {#if filesPlugin}
          {@const storageEntries = pluginManager.getPluginStorageEntries(filesPlugin.manifest.id)}
          {#if filesTab === "manifest"}
            <div class="files-section">
              <div class="files-label">Manifest (<code>{filesPlugin.manifest.id}</code>)</div>
              <pre class="files-pre">{JSON.stringify(filesPlugin.manifest, null, 2)}</pre>
            </div>
          {:else if filesTab === "code"}
            <div class="files-section">
              {#if filesPlugin.code !== null}
                <div class="files-code-header">
                  <span class="files-label">index.js ({filesPlugin.code.length} chars)</span>
                  <button
                    type="button"
                    class="anki-btn small"
                    onclick={() => filesPlugin?.code && copyCode(filesPlugin.code)}
                  >
                    Copy Code
                  </button>
                </div>
                <pre class="files-pre files-code-pre">{filesPlugin.code}</pre>
              {:else}
                <div class="files-empty">
                  Built-in plugin bundled in Cardly (`web/src/lib/plugins/builtin/`).
                </div>
              {/if}
            </div>
          {:else}
            <div class="files-section">
              <div class="files-label">Isolated Storage (<code>cardly:plugin:{filesPlugin.manifest.id}:*</code>)</div>
              {#if storageEntries.length === 0}
                <div class="files-empty">No stored keys yet.</div>
              {:else}
                <div class="files-kv-list">
                  {#each storageEntries as entry (entry.key)}
                    <div class="files-kv-row">
                      <code>{entry.key}</code>
                      <code class="files-val">{entry.value}</code>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          {/if}
        {:else}
          <div class="files-section">
            <div class="files-label">Registered Add-ons ({sortedPlugins.length})</div>
            <div class="files-kv-list">
              {#each sortedPlugins as p (p.manifest.id)}
                <div class="files-kv-row">
                  <span>{p.manifest.name}</span>
                  <code>{p.manifest.id}</code>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>
      <div class="modal-footer files-footer">
        {#if filesPlugin}
          <button
            type="button"
            class="anki-btn"
            onclick={() => filesPlugin && downloadPluginBundle(filesPlugin)}
          >
            {filesPlugin.code !== null ? "Export .cardlyaddon" : "Export manifest.json"}
          </button>
        {:else}
          <div></div>
        {/if}
        <button type="button" class="anki-btn" onclick={() => (filesOpen = false)}>Close</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .addons-window {
    width: 720px;
    max-width: 95vw;
    height: min(580px, 86vh);
  }
  .addons-tabs {
    display: flex;
    border-bottom: 1px solid var(--border);
    background: var(--canvas-elevated);
    padding: 0 12px;
    gap: 4px;
    flex-shrink: 0;
  }
  .addons-tab {
    border: none;
    background: transparent;
    color: var(--fg-subtle);
    padding: 8px 14px;
    font-size: 13px;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
  }
  .addons-tab:hover {
    color: var(--fg);
  }
  .addons-tab.active {
    color: var(--fg);
    font-weight: 600;
    border-bottom-color: var(--button-primary-bg);
  }
  .addons-body {
    display: flex;
    gap: 12px;
    min-height: 0;
    overflow: hidden;
  }
  .addons-left {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
    min-height: 0;
  }
  .addons-notice {
    font-size: 13px;
    color: var(--fg);
    flex-shrink: 0;
  }
  .addons-list {
    flex: 1;
    min-height: 180px;
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    background: var(--canvas-inset);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    outline: none;
  }
  .addons-list:focus-visible {
    border-color: var(--border-focus);
  }
  .addons-empty {
    padding: 18px;
    font-size: 13px;
    color: var(--fg-subtle);
    text-align: center;
  }
  .addon-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 7px 10px;
    border: none;
    background: transparent;
    color: var(--fg);
    font-family: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
    flex-shrink: 0;
  }
  .addon-item:hover {
    background: var(--highlight-bg);
  }
  .addon-item.disabled-item {
    color: var(--fg-subtle);
  }
  .addon-item.selected {
    background: var(--button-primary-bg);
    color: #ffffff;
  }
  .addon-item-left {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .addon-mini-icon,
  .addon-mini-fallback {
    width: 18px;
    height: 18px;
    border-radius: 5px;
    flex-shrink: 0;
    object-fit: cover;
  }
  .addon-mini-fallback {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    background: var(--canvas-elevated);
    border: 1px solid var(--border-subtle);
    color: var(--fg-subtle);
  }
  .addon-item-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .addon-item-ver {
    font-size: 11.5px;
    font-family: var(--font-mono);
    opacity: 0.78;
    flex-shrink: 0;
  }
  .addon-details {
    border: 1px solid var(--border-subtle);
    border-radius: var(--border-radius);
    background: var(--canvas-elevated);
    padding: 8px 10px;
    font-size: 12px;
    display: flex;
    flex-direction: column;
    gap: 5px;
    flex-shrink: 0;
  }
  .addon-details-meta {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    color: var(--fg);
  }
  .addon-tag {
    font-size: 10.5px;
    padding: 1px 5px;
    border-radius: 4px;
    background: var(--canvas-inset);
    border: 1px solid var(--border-subtle);
    color: var(--fg-subtle);
  }
  .addon-details-desc {
    color: var(--fg-subtle);
    line-height: 1.35;
  }
  .addon-details-err {
    color: var(--accent-danger);
    margin-top: 2px;
    word-break: break-word;
  }
  .addons-actions {
    width: 172px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex-shrink: 0;
  }
  .addons-actions .anki-btn {
    width: 100%;
    justify-content: center;
  }
  .addons-spacer {
    flex: 1;
    min-height: 16px;
  }

  /* Permission pills & badges */
  .perm-pills-row {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    align-items: center;
  }
  .perm-pill {
    font-size: 11px;
    font-weight: 500;
    padding: 2px 7px;
    border-radius: 999px;
    background: rgba(59, 130, 246, 0.12);
    border: 1px solid rgba(59, 130, 246, 0.28);
    color: var(--fg);
  }
  .perm-pill.muted {
    background: var(--canvas-inset);
    border-color: var(--border-subtle);
    color: var(--fg-subtle);
  }

  /* Marketplace Tab */
  .mp-body {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 0;
    overflow: hidden;
  }
  .mp-toolbar {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }
  .mp-search {
    flex: 1;
    min-width: 0;
  }
  .mp-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .mp-state {
    padding: 24px 16px;
    text-align: center;
    color: var(--fg-subtle);
    font-size: 13px;
  }
  .mp-error-box {
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    background: var(--canvas-elevated);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }
  .mp-error-title {
    font-weight: 600;
    color: var(--fg);
  }
  .mp-error-detail {
    font-size: 12px;
    color: var(--accent-danger);
    word-break: break-word;
  }
  .mp-error-hint {
    font-size: 12px;
    color: var(--fg-subtle);
  }
  .mp-card {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    background: var(--canvas-elevated);
  }
  .mp-card-icon {
    width: 44px;
    height: 44px;
    border-radius: 10px;
    flex-shrink: 0;
    object-fit: cover;
    border: 1px solid var(--border);
    background: var(--canvas-inset);
  }
  .mp-card-icon-fallback {
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 18px;
    color: var(--fg-subtle);
  }
  .mp-card-main {
    display: flex;
    flex-direction: column;
    gap: 5px;
    min-width: 0;
    flex: 1;
  }
  .mp-card-head {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
  }
  .mp-name {
    font-size: 13.5px;
    font-weight: 600;
    color: var(--fg);
  }
  .mp-author {
    font-size: 12px;
    color: var(--fg-subtle);
  }
  .mp-desc {
    font-size: 12.5px;
    color: var(--fg-subtle);
    line-height: 1.4;
  }
  .mp-tag {
    font-size: 11px;
    color: var(--fg-subtle);
  }
  .mp-card-action {
    flex-shrink: 0;
  }

  /* Sub-dialogs */
  .sub-backdrop {
    z-index: 1100;
  }
  .perm-backdrop {
    z-index: 1200;
  }
  .perm-window {
    width: 480px;
    max-width: 94vw;
  }
  .perm-body {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .perm-summary {
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    background: var(--canvas-elevated);
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .perm-summary-head {
    display: flex;
    align-items: flex-start;
    gap: 10px;
  }
  .perm-summary-icon {
    width: 38px;
    height: 38px;
    border-radius: 9px;
    object-fit: cover;
    border: 1px solid var(--border);
    flex-shrink: 0;
  }
  .perm-summary-head-text {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .perm-summary-title {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    font-size: 13.5px;
  }
  .perm-summary-desc {
    font-size: 12.5px;
    color: var(--fg-subtle);
    line-height: 1.4;
  }
  .perm-source {
    font-size: 11.5px;
    color: var(--fg-subtle);
    margin-top: 2px;
  }
  .perm-section-label {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--fg);
  }
  .perm-none-box {
    border: 1px solid var(--border-subtle);
    border-radius: var(--border-radius);
    background: var(--canvas-inset);
    padding: 10px 12px;
    font-size: 12.5px;
    color: var(--fg-subtle);
    line-height: 1.4;
  }
  .perm-list {
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    background: var(--canvas-inset);
    overflow: hidden;
  }
  .perm-row {
    padding: 8px 10px;
    border-bottom: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .perm-row:last-child {
    border-bottom: none;
  }
  .perm-row-head {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .perm-code {
    font-size: 11px;
    color: var(--fg-subtle);
  }
  .perm-new-badge {
    font-size: 10.5px;
    font-weight: 600;
    padding: 1px 5px;
    border-radius: 4px;
    background: rgba(245, 158, 11, 0.16);
    color: var(--state-buried);
  }
  .perm-row-desc {
    font-size: 12px;
    color: var(--fg-subtle);
  }

  .get-addons-window {
    width: 440px;
    max-width: 94vw;
  }
  .get-addons-body {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .get-addons-text {
    margin: 0;
    font-size: 13px;
    line-height: 1.45;
    color: var(--fg);
  }
  .get-addons-hint {
    margin: 0;
    font-size: 12px;
    line-height: 1.4;
    color: var(--fg-subtle);
  }
  .get-addons-row {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 4px;
    font-size: 13px;
  }
  .get-addons-row input {
    flex: 1;
  }
  .get-addons-footer,
  .config-footer,
  .files-footer {
    justify-content: space-between;
  }
  .footer-right {
    display: flex;
    gap: 8px;
  }

  .config-window {
    width: 640px;
    max-width: 95vw;
    height: min(500px, 84vh);
  }
  .config-body {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .config-split {
    flex: 1;
    display: grid;
    grid-template-columns: 3fr 2fr;
    gap: 12px;
    min-height: 0;
  }
  .config-editor-pane {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-height: 0;
  }
  .config-editor {
    flex: 1;
    width: 100%;
    resize: none;
    font-family: var(--font-mono);
    font-size: 12.5px;
    line-height: 1.45;
    white-space: pre;
    overflow: auto;
  }
  .config-error {
    font-size: 12px;
    color: var(--accent-danger);
  }
  .config-help-pane {
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    background: var(--canvas-inset);
    padding: 10px 12px;
    overflow-y: auto;
    font-size: 12.5px;
  }
  .config-help-title {
    font-weight: 600;
    margin-bottom: 6px;
  }
  .config-help-text {
    margin: 0;
    color: var(--fg-subtle);
    white-space: pre-line;
    line-height: 1.45;
  }

  .files-window {
    width: 600px;
    max-width: 94vw;
    height: min(520px, 84vh);
  }
  .files-tabs {
    display: flex;
    border-bottom: 1px solid var(--border);
    background: var(--canvas-elevated);
    padding: 0 12px;
    gap: 2px;
    flex-shrink: 0;
  }
  .files-tab {
    border: none;
    background: transparent;
    color: var(--fg-subtle);
    padding: 8px 12px;
    font-size: 12.5px;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
  }
  .files-tab.active {
    color: var(--fg);
    font-weight: 600;
    border-bottom-color: var(--button-primary-bg);
  }
  .files-body {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
  }
  .files-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-height: 0;
  }
  .files-code-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .files-label {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--fg);
  }
  .files-pre {
    margin: 0;
    padding: 10px;
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    background: var(--canvas-code);
    font-family: var(--font-mono);
    font-size: 12px;
    overflow: auto;
  }
  .files-code-pre {
    max-height: 300px;
  }
  .files-empty {
    font-size: 12.5px;
    color: var(--fg-subtle);
  }
  .files-kv-list {
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    background: var(--canvas-inset);
    overflow: hidden;
  }
  .files-kv-row {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 7px 10px;
    font-size: 12px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .files-kv-row:last-child {
    border-bottom: none;
  }
  .files-val {
    color: var(--fg-subtle);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  @media (max-width: 640px) {
    .addons-body {
      flex-direction: column;
      overflow-y: auto;
    }
    .addons-actions {
      width: 100%;
      flex-direction: row;
      flex-wrap: wrap;
    }
    .addons-actions .anki-btn {
      flex: 1;
      min-width: 130px;
    }
    .addons-spacer {
      display: none;
    }
    .config-split {
      grid-template-columns: 1fr;
    }
    .mp-card {
      flex-direction: column;
    }
  }
</style>
