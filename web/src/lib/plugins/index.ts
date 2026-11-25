import { builtinPlugins } from "./builtin";
import { pluginManager } from "./manager.svelte";

export { pluginManager, PluginManager } from "./manager.svelte";
export { default as PluginSlot } from "./PluginSlot.svelte";
export {
  buildPluginZipBytes,
  CARDLY_PLUGINS_REPO_URL,
  compareSemver,
  DEFAULT_REGISTRY_URL,
  fetchPluginByDirectUrl,
  fetchPluginFromSource,
  fetchRegistryEntries,
  hasExpandedPermissions,
  parsePluginFile,
  validatePermissions,
} from "./package";
export { ALL_PLUGIN_PERMISSIONS, PLUGIN_PERMISSION_META } from "./types";
export type {
  ActiveSlotMount,
  CardlyPlugin,
  HeatmapStats,
  PermissionMeta,
  PluginAppState,
  PluginCleanup,
  PluginEventMap,
  PluginInfo,
  PluginManifest,
  PluginPermission,
  PluginSDK,
  PluginSlotId,
  PluginSlotRenderer,
  RegistryPluginEntry,
} from "./types";

export function initPlugins(): void {
  pluginManager.registerAll(builtinPlugins);
  pluginManager.init();
}
