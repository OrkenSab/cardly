import { defineConfig } from "@playwright/test";
import fs from "fs";

const systemChromium = [
  process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH,
  "/usr/bin/chromium",
  "/usr/bin/chromium-browser",
  "/usr/bin/google-chrome-stable",
].find((p) => p && fs.existsSync(p));

export default defineConfig({
  testDir: "./tests",
  timeout: 60_000,
  expect: { timeout: 15_000 },
  fullyParallel: false,
  workers: 1,
  reporter: "list",
  use: {
    baseURL: process.env.CARDLY_BASE_URL ?? "http://127.0.0.1:8091",
    viewport: { width: 1280, height: 840 },
    headless: true,
    launchOptions: systemChromium ? { executablePath: systemChromium } : undefined,
  },
});
