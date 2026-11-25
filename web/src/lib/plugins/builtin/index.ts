import type { CardlyPlugin } from "../types";
import { sessionStreakPlugin } from "./sessionStreak";

export const builtinPlugins: readonly CardlyPlugin[] = [sessionStreakPlugin];
