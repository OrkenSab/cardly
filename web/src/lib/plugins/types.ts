import type { MainView, ThemeMode } from "../store.svelte";

export type PluginPermission =
  | "ui:decks"
  | "ui:reviewer"
  | "events:cards"
  | "events:app"
  | "stats:read"
  | "storage"
  | "app:read";

export const ALL_PLUGIN_PERMISSIONS: readonly PluginPermission[] = [
  "ui:decks",
  "ui:reviewer",
  "events:cards",
  "events:app",
  "stats:read",
  "storage",
  "app:read",
];

export interface PermissionMeta {
  readonly id: PluginPermission;
  readonly label: string;
  readonly description: string;
}

export const PLUGIN_PERMISSION_META: Readonly<Record<PluginPermission, PermissionMeta>> = {
  "ui:decks": {
    id: "ui:decks",
    label: "Deck List UI",
    description: "Mount widgets above or below the deck list (decks:top, decks:bottom).",
  },
  "ui:reviewer": {
    id: "ui:reviewer",
    label: "Reviewer UI",
    description: "Mount widgets in the reviewer action bar or below cards (reviewer:actions, card:after).",
  },
  "events:cards": {
    id: "events:cards",
    label: "Card Events",
    description: "Observe when cards are shown or answered (card:shown, card:answered).",
  },
  "events:app": {
    id: "events:app",
    label: "App Events",
    description: "Observe deck list updates and collection refreshes (decks:updated, sync:completed).",
  },
  "stats:read": {
    id: "stats:read",
    label: "Review Statistics",
    description: "Read daily review history, streaks, and upcoming due forecast.",
  },
  storage: {
    id: "storage",
    label: "Plugin Storage",
    description: "Read and write isolated persistent plugin data.",
  },
  "app:read": {
    id: "app:read",
    label: "Read App State",
    description: "Read current view, active deck ID, theme, and connection status.",
  },
};

export interface PluginManifest {
  readonly id: string;
  readonly name: string;
  readonly version: string;
  readonly description: string;
  readonly author?: string;
  readonly homepage?: string;
  readonly iconUrl?: string;
  readonly updateUrl?: string;
  readonly builtIn?: boolean;
  readonly defaultEnabled?: boolean;
  readonly configHelp?: string;
  readonly permissions?: readonly PluginPermission[];
}

export type PluginSlotId =
  | "decks:top"
  | "decks:bottom"
  | "reviewer:actions"
  | "card:after";

export interface PluginEventMap {
  "card:shown": {
    cardId: number;
    noteId: number;
    deckId: number;
    phase: "question";
  };
  "card:answered": {
    cardId: number;
    noteId: number;
    deckId: number;
    ease: 1 | 2 | 3 | 4;
    takenMs: number;
  };
  "decks:updated": {
    count: number;
    currentDeckId: number;
  };
  "sync:completed": {
    source: string;
    timestamp: number;
  };
}

export type PluginCleanup = () => void;

export type PluginSlotRenderer = (container: HTMLElement) => void | PluginCleanup;

export interface PluginAppState {
  readonly view: MainView;
  readonly currentDeckId: number;
  readonly theme: ThemeMode;
  readonly resolvedTheme: "dark" | "light";
  readonly isDark: boolean;
  readonly minimalistMode: boolean;
  readonly username: string;
  readonly isOnline: boolean;
}

export interface HeatmapStats {
  readonly history: Record<string, number>;
  readonly forecast: Record<string, number>;
  readonly today: string;
  readonly todayCount: number;
  readonly dailyAverage: number;
  readonly daysLearnedPercent: number;
  readonly currentStreak: number;
  readonly longestStreak: number;
}

export interface PluginSDK {
  readonly manifest: Readonly<PluginManifest>;
  readonly ui: {
    mount(slotId: PluginSlotId, renderOrCleanup: PluginSlotRenderer): PluginCleanup;
    refresh(slotId?: PluginSlotId): void;
  };
  readonly events: {
    on<K extends keyof PluginEventMap>(
      event: K,
      handler: (payload: PluginEventMap[K]) => void
    ): PluginCleanup;
  };
  readonly storage: {
    get<T>(key: string, fallback: T): T;
    set<T>(key: string, value: T): void;
    remove(key: string): void;
  };
  readonly config: {
    get<T extends Record<string, unknown>>(): T;
    onChange(handler: (config: Record<string, unknown>) => void): PluginCleanup;
  };
  readonly app: {
    getState(): Readonly<PluginAppState>;
  };
  readonly stats: {
    getHeatmap(options?: { deckId?: number }): Promise<HeatmapStats>;
  };
}

export interface CardlyPlugin {
  readonly manifest: PluginManifest;
  readonly defaultConfig?: Record<string, unknown>;
  onLoad(sdk: PluginSDK): void | Promise<void>;
  onUnload?(): void | Promise<void>;
}

export interface PluginInfo {
  readonly manifest: PluginManifest;
  readonly enabled: boolean;
  readonly loaded: boolean;
  readonly hasConfig: boolean;
  readonly code: string | null;
  readonly lastError: string | null;
}

export interface RegistryPluginEntry {
  readonly id: string;
  readonly name: string;
  readonly version: string;
  readonly description?: string;
  readonly author?: string;
  readonly homepage?: string;
  readonly iconUrl?: string;
  readonly updateUrl?: string;
  readonly packageUrl?: string;
  readonly codeUrl?: string;
  readonly code?: string;
  readonly defaultEnabled?: boolean;
  readonly permissions?: readonly PluginPermission[];
  readonly tags?: readonly string[];
}

export interface ActiveSlotMount {
  readonly pluginId: string;
  readonly mountId: number;
  readonly render: PluginSlotRenderer;
}
