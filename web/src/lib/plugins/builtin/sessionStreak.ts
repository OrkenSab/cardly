import type { CardlyPlugin, PluginSDK } from "../types";

interface StreakState {
  date: string;
  reviewed: number;
  streak: number;
}

interface StreakConfig extends Record<string, unknown> {
  showStreak: boolean;
  minStreakToShow: number;
}

const STORAGE_KEY = "today";

function getLocalDateKey(): string {
  const now = new Date();
  const y = now.getFullYear();
  const m = String(now.getMonth() + 1).padStart(2, "0");
  const d = String(now.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

function loadTodayState(sdk: PluginSDK): StreakState {
  const today = getLocalDateKey();
  const saved = sdk.storage.get<StreakState | null>(STORAGE_KEY, null);
  if (saved && saved.date === today && typeof saved.reviewed === "number") {
    return {
      date: today,
      reviewed: Math.max(0, saved.reviewed),
      streak: Math.max(0, saved.streak ?? 0),
    };
  }
  return { date: today, reviewed: 0, streak: 0 };
}

export const sessionStreakPlugin: CardlyPlugin = {
  manifest: {
    id: "session-streak",
    name: "Session Streak",
    version: "1.0.0",
    description:
      "Shows a compact badge of cards reviewed today and your current Good/Easy answer streak in the reviewer.",
    author: "Cardly",
    homepage: "https://github.com/cardly-anki/cardly",
    builtIn: true,
    defaultEnabled: true,
    permissions: ["ui:reviewer", "events:cards", "storage"],
    configHelp:
      "Controls the Session Streak badge in the reviewer bottom bar.\n\n" +
      "• showStreak (boolean): Display consecutive Good/Easy streak beside today's review count.\n" +
      "• minStreakToShow (number): Minimum streak required before the streak counter appears.",
  },
  defaultConfig: {
    showStreak: true,
    minStreakToShow: 2,
  },

  onLoad(sdk: PluginSDK) {
    let state = loadTodayState(sdk);
    let config = sdk.config.get<StreakConfig>();

    sdk.config.onChange((next) => {
      config = {
        showStreak: next.showStreak !== false,
        minStreakToShow:
          typeof next.minStreakToShow === "number" && next.minStreakToShow >= 1
            ? Math.floor(next.minStreakToShow)
            : 2,
      };
      sdk.ui.refresh("reviewer:actions");
    });

    sdk.events.on("card:answered", ({ ease }) => {
      const today = getLocalDateKey();
      if (state.date !== today) {
        state = { date: today, reviewed: 0, streak: 0 };
      }

      state = {
        date: today,
        reviewed: state.reviewed + 1,
        streak: ease >= 3 ? state.streak + 1 : ease === 1 ? 0 : state.streak,
      };
      sdk.storage.set(STORAGE_KEY, state);
      sdk.ui.refresh("reviewer:actions");
    });

    sdk.ui.mount("reviewer:actions", (container) => {
      const today = getLocalDateKey();
      if (state.date !== today) {
        state = loadTodayState(sdk);
      }

      const badge = document.createElement("div");
      badge.className = "cardly-streak-badge";
      badge.title = `Reviewed today: ${state.reviewed} · Current Good/Easy streak: ${state.streak}`;
      badge.style.display = "inline-flex";
      badge.style.alignItems = "center";
      badge.style.gap = "6px";
      badge.style.padding = "4px 10px";
      badge.style.borderRadius = "999px";
      badge.style.fontSize = "12px";
      badge.style.fontWeight = "600";
      badge.style.fontVariantNumeric = "tabular-nums";
      badge.style.lineHeight = "1.2";
      badge.style.background = "var(--canvas-elevated)";
      badge.style.border = "1px solid var(--border-subtle)";
      badge.style.color = "var(--fg-subtle)";
      badge.style.userSelect = "none";
      badge.style.whiteSpace = "nowrap";

      const countSpan = document.createElement("span");
      countSpan.style.color = state.reviewed > 0 ? "var(--state-review)" : "var(--fg-subtle)";
      countSpan.textContent = `${state.reviewed} reviewed`;
      badge.appendChild(countSpan);

      const minStreak =
        typeof config.minStreakToShow === "number" && config.minStreakToShow >= 1
          ? config.minStreakToShow
          : 2;

      if (config.showStreak !== false && state.streak >= minStreak) {
        const sep = document.createElement("span");
        sep.textContent = "·";
        sep.style.opacity = "0.55";
        badge.appendChild(sep);

        const streakSpan = document.createElement("span");
        streakSpan.style.color = "var(--fg)";
        streakSpan.textContent = `${state.streak} streak`;
        badge.appendChild(streakSpan);
      }

      container.appendChild(badge);
    });
  },
};
