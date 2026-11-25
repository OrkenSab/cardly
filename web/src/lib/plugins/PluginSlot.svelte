<script lang="ts">
  import { pluginManager } from "./manager.svelte";
  import type { PluginCleanup, PluginSlotId, PluginSlotRenderer } from "./types";

  let {
    id,
    class: className = "",
  }: {
    id: PluginSlotId;
    class?: string;
  } = $props();

  const mounts = $derived(pluginManager.getSlotMounts(id));
  const revision = $derived(pluginManager.getSlotRevision(id));

  interface MountParams {
    pluginId: string;
    render: PluginSlotRenderer;
    revision: number;
  }

  function mountPlugin(node: HTMLElement, initial: MountParams) {
    let current = initial;
    let cleanup: PluginCleanup | void;

    function runCleanup() {
      if (typeof cleanup === "function") {
        try {
          cleanup();
        } catch (err) {
          console.error(`[Cardly Plugin:${current.pluginId}] slot cleanup error in "${id}":`, err);
        }
        cleanup = undefined;
      }
    }

    function renderIntoNode() {
      runCleanup();
      node.replaceChildren();
      try {
        cleanup = current.render(node);
      } catch (err) {
        console.error(`[Cardly Plugin:${current.pluginId}] slot render error in "${id}":`, err);
      }
    }

    renderIntoNode();

    return {
      update(next: MountParams) {
        if (
          next.render !== current.render ||
          next.revision !== current.revision ||
          next.pluginId !== current.pluginId
        ) {
          current = next;
          renderIntoNode();
        }
      },
      destroy() {
        runCleanup();
        node.replaceChildren();
      },
    };
  }
</script>

{#if mounts.length > 0}
  <div class="plugin-slot {className}" data-plugin-slot={id}>
    {#each mounts as item (`${item.pluginId}:${item.mountId}`)}
      <div
        class="plugin-mount"
        data-plugin-id={item.pluginId}
        use:mountPlugin={{ pluginId: item.pluginId, render: item.render, revision }}
      ></div>
    {/each}
  </div>
{/if}

<style>
  .plugin-slot {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .plugin-mount {
    display: contents;
  }
</style>
