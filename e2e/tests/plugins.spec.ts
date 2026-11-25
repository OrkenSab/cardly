import { expect, test } from "@playwright/test";
import fs from "fs";
import path from "path";

const USERNAME = process.env.CARDLY_TEST_USER ?? "alice";
const PASSWORD = process.env.CARDLY_TEST_PASS ?? "secret123";

async function login(page: import("@playwright/test").Page) {
  await page.goto("/");
  const loginHeading = page.locator("h1", { hasText: "Cardly" });
  const decksBtn = page.getByRole("button", { name: "Decks", exact: true });
  await expect(loginHeading.or(decksBtn)).toBeVisible();

  if (await page.locator('input[autocomplete="username"]').isVisible()) {
    await page.locator('input[autocomplete="username"]').fill(USERNAME);
    await page.locator('input[autocomplete="current-password"]').fill(PASSWORD);
    await page.getByRole("button", { name: "Sign In" }).click();
    await expect(decksBtn).toBeVisible();
  }
}

const EXTERNAL_PLUGIN_V1 = `
export const manifest = {
  id: "deck-banner-demo",
  name: "Deck Banner Demo",
  version: "1.0.0",
  description: "Displays a custom banner in decks:top",
  author: "Test Author",
  permissions: ["ui:decks"]
};

export default {
  manifest,
  onLoad(sdk) {
    sdk.ui.mount("decks:top", (container) => {
      const el = document.createElement("div");
      el.className = "test-external-banner";
      el.textContent = "External Plugin v1.0.0";
      container.appendChild(el);
    });
  }
};
`;

const EXTERNAL_PLUGIN_V1_1 = `
export const manifest = {
  id: "deck-banner-demo",
  name: "Deck Banner Demo",
  version: "1.1.0",
  description: "Displays a custom banner in decks:top",
  author: "Test Author",
  permissions: ["ui:decks"]
};

export default {
  manifest,
  onLoad(sdk) {
    sdk.ui.mount("decks:top", (container) => {
      const el = document.createElement("div");
      el.className = "test-external-banner";
      el.textContent = "External Plugin v1.1.0";
      container.appendChild(el);
    });
  }
};
`;

const MARKETPLACE_HEATMAP_JS = `
export const manifest = {
  id: "review-heatmap-demo",
  name: "Review Heatmap Demo",
  version: "1.0.0",
  description: "Shows review activity heatmap above decks.",
  author: "Cardly Community",
  permissions: ["ui:decks", "events:cards"]
};

export default {
  manifest,
  onLoad(sdk) {
    sdk.events.on("card:answered", () => {});
    sdk.ui.mount("decks:top", (container) => {
      const box = document.createElement("div");
      box.className = "mp-heatmap-widget";
      box.textContent = "Heatmap Ready";
      container.appendChild(box);
    });
  }
};
`;

const SNEAKY_NO_PERM_JS = `
export const manifest = {
  id: "sneaky-no-perm",
  name: "Sneaky No Perm",
  version: "1.0.0",
  description: "Tries to mount without permission",
  author: "Untrusted",
  permissions: []
};

export default {
  manifest,
  onLoad(sdk) {
    sdk.ui.mount("decks:top", (container) => {
      const el = document.createElement("div");
      el.className = "sneaky-banner";
      el.textContent = "SHOULD NOT RENDER";
      container.appendChild(el);
    });
  }
};
`;

test.describe("Cardly Add-ons / Plugin System E2E", () => {
  test("Tools -> Add-ons (Ctrl+Shift+A) matches Anki workflow, toggles, configures, and updates Session Streak", async ({
    page,
  }) => {
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(e.message));

    await login(page);

    await page.getByRole("button", { name: "Add", exact: true }).click();
    const addModal = page.locator(".modal-window.add-window");
    await expect(addModal).toBeVisible();

    const editorFrame = addModal.frameLocator("iframe.editor-iframe");
    const richFields = editorFrame.locator('.rich-text-editable [contenteditable="true"]');
    await expect(richFields.first()).toBeVisible({ timeout: 20_000 });

    for (const n of [1, 2]) {
      await richFields.nth(0).click();
      await page.keyboard.type(`Addon Test Front ${n}`);
      await richFields.nth(1).click();
      await page.keyboard.type(`Addon Test Back ${n}`);
      await editorFrame.getByRole("button", { name: "Add", exact: true }).click();
      await expect(richFields.nth(0)).toHaveText("", { timeout: 10_000 });
    }
    await addModal.locator(".modal-close").click();
    await expect(addModal).toBeHidden();

    await page.getByRole("button", { name: "Tools" }).click();
    await page.getByRole("menuitem", { name: /Add-ons\.\.\./ }).click();

    const addonsWindow = page.locator(".modal-window.addons-window");
    await expect(addonsWindow).toBeVisible();
    await expect(addonsWindow.locator(".modal-title")).toHaveText("Add-ons");

    for (const label of [
      "Get Add-ons...",
      "Install from file...",
      "Check for Updates",
      "View Add-on Page",
      "Config",
      "View Files",
      "Toggle Enabled",
      "Delete",
    ]) {
      await expect(addonsWindow.getByRole("button", { name: label })).toBeVisible();
    }

    const streakItem = addonsWindow.locator(".addon-item", { hasText: "Session Streak" });
    await expect(streakItem).toBeVisible();
    await streakItem.click();
    await expect(streakItem).toHaveClass(/selected/);

    await addonsWindow.getByRole("button", { name: "Config" }).click();
    const configWin = page.locator(".modal-window.config-window");
    await expect(configWin).toBeVisible();
    await expect(configWin.locator(".modal-title")).toContainText("Configure 'Session Streak'");
    await configWin.getByRole("button", { name: "Cancel" }).click();
    await expect(configWin).toBeHidden();

    await addonsWindow.getByRole("button", { name: "Toggle Enabled" }).click();
    await expect(streakItem).toContainText("Session Streak (disabled)");
    await addonsWindow.getByRole("button", { name: "Toggle Enabled" }).click();
    await expect(streakItem).not.toContainText("(disabled)");

    await addonsWindow.getByRole("button", { name: "Close" }).click();
    await expect(addonsWindow).toBeHidden();

    await page.locator(".deck-row", { hasText: "Default" }).locator(".deck-link").click();
    await page.getByRole("button", { name: "Study Now" }).click();

    const reviewerSlot = page.locator('[data-plugin-slot="reviewer:actions"]');
    const badge = reviewerSlot.locator(".cardly-streak-badge");
    await expect(badge).toContainText("0 reviewed");

    await page.getByRole("button", { name: /Show Answer/ }).click();
    await page.locator("button.ease-btn", { hasText: "Good" }).click();
    await expect(badge).toContainText("1 reviewed");

    await page.keyboard.press("Control+Shift+A");
    await expect(addonsWindow).toBeVisible();
    await streakItem.click();
    await addonsWindow.getByRole("button", { name: "Toggle Enabled" }).click();
    await expect(streakItem).toContainText("(disabled)");
    await addonsWindow.getByRole("button", { name: "Close" }).click();

    await expect(page.locator('[data-plugin-slot="reviewer:actions"]')).toHaveCount(0);

    await page.keyboard.press("Control+Shift+A");
    await expect(addonsWindow).toBeVisible();
    await streakItem.click();
    await addonsWindow.getByRole("button", { name: "Toggle Enabled" }).click();
    await addonsWindow.getByRole("button", { name: "Close" }).click();

    await expect(badge).toContainText("1 reviewed");
    expect(errors).toEqual([]);
  });

  test("External plugin install from file with permissions prompt, server persistence, Check for Updates, and Delete", async ({
    page,
  }) => {
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(e.message));

    await page.route("https://raw.githubusercontent.com/cardly-anki/plugins/main/registry.json", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          plugins: [
            {
              id: "deck-banner-demo",
              name: "Deck Banner Demo",
              version: "1.1.0",
              description: "Displays a custom banner in decks:top",
              author: "Test Author",
              permissions: ["ui:decks"],
              codeUrl: "https://raw.githubusercontent.com/cardly-anki/plugins/main/deck-banner-demo.js",
            },
          ],
        }),
      });
    });

    await page.route(
      "https://raw.githubusercontent.com/cardly-anki/plugins/main/deck-banner-demo.js",
      async (route) => {
        await route.fulfill({
          status: 200,
          contentType: "text/javascript",
          body: EXTERNAL_PLUGIN_V1_1,
        });
      }
    );

    await login(page);

    // 1. Open Add-ons -> Install from file... (.js) -> confirm permissions
    await page.keyboard.press("Control+Shift+A");
    const addonsWindow = page.locator(".modal-window.addons-window");
    await expect(addonsWindow).toBeVisible();

    const fileChooserPromise = page.waitForEvent("filechooser");
    await addonsWindow.getByRole("button", { name: "Install from file..." }).click();
    const fileChooser = await fileChooserPromise;
    await fileChooser.setFiles({
      name: "deck-banner-demo.js",
      mimeType: "text/javascript",
      buffer: Buffer.from(EXTERNAL_PLUGIN_V1, "utf8"),
    });

    const permWindow = page.locator(".modal-window.perm-window");
    await expect(permWindow).toBeVisible();
    await expect(permWindow).toContainText("Deck List UI");
    await permWindow.getByRole("button", { name: "Install Add-on" }).click();
    await expect(permWindow).toBeHidden();

    const pluginItem = addonsWindow.locator(".addon-item", { hasText: "Deck Banner Demo" });
    await expect(pluginItem).toBeVisible({ timeout: 10_000 });
    await expect(pluginItem).toContainText("v1.0.0");

    // Check View Files -> index.js tab shows code
    await pluginItem.click();
    await addonsWindow.getByRole("button", { name: "View Files" }).click();
    const filesWin = page.locator(".modal-window.files-window");
    await expect(filesWin).toBeVisible();
    await filesWin.getByRole("button", { name: "index.js" }).click();
    await expect(filesWin.locator(".files-code-pre")).toContainText("External Plugin v1.0.0");
    await filesWin.locator(".modal-footer").getByRole("button", { name: "Close" }).click();

    await addonsWindow.getByRole("button", { name: "Close" }).click();

    const decksTopSlot = page.locator('[data-plugin-slot="decks:top"]');
    await expect(decksTopSlot.locator(".test-external-banner")).toHaveText("External Plugin v1.0.0");

    // 2. Reload page and verify server persistence
    await page.reload();
    await expect(decksTopSlot.locator(".test-external-banner")).toHaveText("External Plugin v1.0.0", {
      timeout: 15_000,
    });

    // 3. Check for Updates -> upgrades 1.0.0 -> 1.1.0 without reprompting (permissions unchanged)
    await page.keyboard.press("Control+Shift+A");
    await expect(addonsWindow).toBeVisible();
    await addonsWindow.getByRole("button", { name: "Check for Updates" }).click();
    await expect(pluginItem).toContainText("v1.1.0", { timeout: 10_000 });
    await addonsWindow.getByRole("button", { name: "Close" }).click();

    await expect(decksTopSlot.locator(".test-external-banner")).toHaveText("External Plugin v1.1.0");

    await page.reload();
    await expect(decksTopSlot.locator(".test-external-banner")).toHaveText("External Plugin v1.1.0", {
      timeout: 15_000,
    });

    // 4. Delete external plugin
    await page.keyboard.press("Control+Shift+A");
    await expect(addonsWindow).toBeVisible();
    await pluginItem.click();
    await addonsWindow.getByRole("button", { name: "Delete" }).click();

    const confirmDialog = page.locator(".modal-window", { hasText: "Delete Add-on" });
    await expect(confirmDialog).toBeVisible();
    await confirmDialog.getByRole("button", { name: "Delete" }).click();

    await expect(pluginItem).toHaveCount(0);
    await addonsWindow.getByRole("button", { name: "Close" }).click();
    await expect(page.locator('[data-plugin-slot="decks:top"]')).toHaveCount(0);

    await page.reload();
    await expect(page.getByRole("button", { name: "Decks", exact: true })).toBeVisible();
    await expect(page.locator('[data-plugin-slot="decks:top"]')).toHaveCount(0);

    expect(errors).toEqual([]);
  });

  test("Marketplace search, permission prompt Cancel, 1-click Install, and runtime permission enforcement", async ({
    page,
  }) => {
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(e.message));

    await page.route("https://raw.githubusercontent.com/cardly-anki/plugins/main/registry.json", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          plugins: [
            {
              id: "review-heatmap-demo",
              name: "Review Heatmap Demo",
              version: "1.0.0",
              description: "Shows review activity heatmap above decks.",
              author: "Cardly Community",
              permissions: ["ui:decks", "events:cards"],
              tags: ["stats", "heatmap"],
              codeUrl: "https://raw.githubusercontent.com/cardly-anki/plugins/main/review-heatmap-demo.js",
            },
            {
              id: "sneaky-no-perm",
              name: "Sneaky No Perm",
              version: "1.0.0",
              description: "Tries to mount without permission",
              author: "Untrusted",
              permissions: [],
              tags: ["security"],
              codeUrl: "https://raw.githubusercontent.com/cardly-anki/plugins/main/sneaky-no-perm.js",
            },
          ],
        }),
      });
    });

    await page.route(
      "https://raw.githubusercontent.com/cardly-anki/plugins/main/review-heatmap-demo.js",
      async (route) => {
        await route.fulfill({
          status: 200,
          contentType: "text/javascript",
          body: MARKETPLACE_HEATMAP_JS,
        });
      }
    );

    await page.route(
      "https://raw.githubusercontent.com/cardly-anki/plugins/main/sneaky-no-perm.js",
      async (route) => {
        await route.fulfill({
          status: 200,
          contentType: "text/javascript",
          body: SNEAKY_NO_PERM_JS,
        });
      }
    );

    await login(page);

    // 1. Open Add-ons -> Marketplace tab
    await page.keyboard.press("Control+Shift+A");
    const addonsWindow = page.locator(".modal-window.addons-window");
    await expect(addonsWindow).toBeVisible();

    await addonsWindow.getByRole("tab", { name: "Marketplace" }).click();
    const heatmapCard = addonsWindow.locator(".mp-card", { hasText: "Review Heatmap Demo" });
    const sneakyCard = addonsWindow.locator(".mp-card", { hasText: "Sneaky No Perm" });
    await expect(heatmapCard).toBeVisible({ timeout: 10_000 });
    await expect(sneakyCard).toBeVisible();

    // Verify search filters catalog
    await addonsWindow.getByRole("searchbox", { name: "Search Marketplace" }).fill("heatmap");
    await expect(heatmapCard).toBeVisible();
    await expect(sneakyCard).toHaveCount(0);

    // 2. Click Install -> Cancel in permission dialog -> plugin must NOT install
    await heatmapCard.getByRole("button", { name: "Install" }).click();
    const permWindow = page.locator(".modal-window.perm-window");
    await expect(permWindow).toBeVisible();
    await expect(permWindow).toContainText("Deck List UI");
    await expect(permWindow).toContainText("Card Events");
    await permWindow.getByRole("button", { name: "Cancel" }).click();
    await expect(permWindow).toBeHidden();
    await expect(heatmapCard.getByRole("button", { name: "Install" })).toBeVisible();

    // 3. Click Install -> Confirm -> plugin installs & status becomes Installed
    await heatmapCard.getByRole("button", { name: "Install" }).click();
    await expect(permWindow).toBeVisible();
    await permWindow.getByRole("button", { name: "Install Add-on" }).click();
    await expect(permWindow).toBeHidden();
    await expect(heatmapCard.getByRole("button", { name: "Installed" })).toBeDisabled();

    // 4. Clear search & install sneaky-no-perm (no permissions -> blocked at runtime)
    await addonsWindow.getByRole("searchbox", { name: "Search Marketplace" }).fill("");
    await expect(sneakyCard).toBeVisible();
    await sneakyCard.getByRole("button", { name: "Install" }).click();
    await expect(permWindow).toBeVisible();
    await expect(permWindow).toContainText("No special permissions requested");
    await permWindow.getByRole("button", { name: "Install Add-on" }).click();
    await expect(permWindow).toBeHidden();

    // Check Installed tab: sneaky-no-perm records runtime permission error
    await addonsWindow.getByRole("tab", { name: /Installed/ }).click();
    const sneakyRow = addonsWindow.locator(".addon-item", { hasText: "Sneaky No Perm" });
    await expect(sneakyRow).toBeVisible();
    await sneakyRow.click();
    await expect(addonsWindow.locator(".addon-details-err")).toContainText(
      'Permission "ui:decks" is required'
    );

    await addonsWindow.getByRole("button", { name: "Close" }).click();

    // Heatmap widget works, sneaky widget was blocked
    await expect(page.locator('[data-plugin-slot="decks:top"] .mp-heatmap-widget')).toHaveText(
      "Heatmap Ready"
    );
    await expect(page.locator(".sneaky-banner")).toHaveCount(0);

    // Clean up both installed plugins
    await page.keyboard.press("Control+Shift+A");
    await expect(addonsWindow).toBeVisible();
    for (const name of ["Review Heatmap Demo", "Sneaky No Perm"]) {
      const row = addonsWindow.locator(".addon-item", { hasText: name });
      await row.click();
      await addonsWindow.getByRole("button", { name: "Delete" }).click();
      const confirmDialog = page.locator(".modal-window", { hasText: "Delete Add-on" });
      await confirmDialog.getByRole("button", { name: "Delete" }).click();
      await expect(row).toHaveCount(0);
    }
    await addonsWindow.getByRole("button", { name: "Close" }).click();

    expect(errors).toEqual([]);
  });

  test("Review Heatmap plugin installs, renders SVG grid & stats, switches palette, and updates live after answering", async ({
    page,
  }) => {
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(e.message));

    const pluginCodePath = path.resolve(__dirname, "../../plugins/review-heatmap/index.js");
    const pluginCode = fs.readFileSync(pluginCodePath, "utf8");

    await login(page);

    // Ensure at least 1 card exists in Default deck to review
    await page.getByRole("button", { name: "Add", exact: true }).click();
    const addModal = page.locator(".modal-window.add-window");
    await expect(addModal).toBeVisible();
    const editorFrame = addModal.frameLocator("iframe.editor-iframe");
    const richFields = editorFrame.locator('.rich-text-editable [contenteditable="true"]');
    await expect(richFields.first()).toBeVisible({ timeout: 20_000 });
    await richFields.nth(0).click();
    await page.keyboard.type("Heatmap E2E Front");
    await richFields.nth(1).click();
    await page.keyboard.type("Heatmap E2E Back");
    await editorFrame.getByRole("button", { name: "Add", exact: true }).click();
    await expect(richFields.nth(0)).toHaveText("", { timeout: 10_000 });
    await addModal.locator(".modal-close").click();
    await expect(addModal).toBeHidden();

    // Install plugins/review-heatmap/index.js via Add-ons -> Install from file...
    await page.keyboard.press("Control+Shift+A");
    const addonsWindow = page.locator(".modal-window.addons-window");
    await expect(addonsWindow).toBeVisible();

    const fileChooserPromise = page.waitForEvent("filechooser");
    await addonsWindow.getByRole("button", { name: "Install from file..." }).click();
    const fileChooser = await fileChooserPromise;
    await fileChooser.setFiles({
      name: "review-heatmap.js",
      mimeType: "text/javascript",
      buffer: Buffer.from(pluginCode, "utf8"),
    });

    const permWindow = page.locator(".modal-window.perm-window");
    await expect(permWindow).toBeVisible();
    await expect(permWindow).toContainText("Review Statistics");
    await permWindow.getByRole("button", { name: "Install Add-on" }).click();
    await expect(permWindow).toBeHidden();
    await addonsWindow.getByRole("button", { name: "Close" }).click();
    await expect(addonsWindow).toBeHidden();

    // Verify heatmap mounted in decks:bottom with 365+ SVG cells and 4 stat items
    const heatmap = page.locator('[data-plugin-slot="decks:bottom"] .rh-root');
    await expect(heatmap).toBeVisible({ timeout: 10_000 });
    const cellCount = await heatmap.locator("svg.rh-svg rect.rh-cell").count();
    expect(cellCount).toBeGreaterThanOrEqual(365);

    await expect(heatmap.locator('[data-stat="daily-average"]')).toBeVisible();
    await expect(heatmap.locator('[data-stat="days-learned"]')).toBeVisible();
    await expect(heatmap.locator('[data-stat="longest-streak"]')).toBeVisible();
    await expect(heatmap.locator('[data-stat="current-streak"]')).toBeVisible();

    const todayCell = heatmap.locator('rect.rh-cell[data-today="true"]');
    await expect(todayCell).toBeVisible();
    const beforeCount = Number((await todayCell.getAttribute("data-count")) ?? "0");

    // Open Settings modal & switch palette to Olive
    await heatmap.locator(".rh-settings-btn").click();
    const settingsDialog = page.locator(".rh-settings-window");
    await expect(settingsDialog).toBeVisible();
    await settingsDialog.locator('.rh-palette-opt[data-scheme="olive"]').click();
    await settingsDialog.getByRole("button", { name: "Save" }).click();
    await expect(settingsDialog).toBeHidden();

    const accentColor = await heatmap
      .locator('[data-stat="current-streak"] .rh-stat-val')
      .evaluate((el) => getComputedStyle(el).color);
    // Olive accent in light (#216e39 -> rgb(33, 110, 57)) or dark (#4ade80 -> rgb(74, 222, 128))
    expect(["rgb(33, 110, 57)", "rgb(74, 222, 128)"]).toContain(accentColor);

    // Review a card and verify heatmap updates live
    await page.locator(".deck-row", { hasText: "Default" }).locator(".deck-link").click();
    await page.getByRole("button", { name: "Study Now" }).click();
    await page.getByRole("button", { name: /Show Answer/ }).click();
    await page.locator("button.ease-btn", { hasText: "Good" }).click();

    await page.getByRole("button", { name: "Decks", exact: true }).click();
    await expect(heatmap).toBeVisible();
    await expect
      .poll(async () => Number((await todayCell.getAttribute("data-count")) ?? "0"))
      .toBe(beforeCount + 1);
    await expect(heatmap.locator('[data-stat="current-streak"]')).not.toContainText("0 days");

    // Clean up Review Heatmap plugin
    await page.keyboard.press("Control+Shift+A");
    await expect(addonsWindow).toBeVisible();
    const row = addonsWindow.locator(".addon-item", { hasText: "Review Heatmap" });
    await row.click();
    await addonsWindow.getByRole("button", { name: "Delete" }).click();
    const confirmDialog = page.locator(".modal-window", { hasText: "Delete Add-on" });
    await confirmDialog.getByRole("button", { name: "Delete" }).click();
    await expect(row).toHaveCount(0);
    await addonsWindow.getByRole("button", { name: "Close" }).click();

    expect(errors).toEqual([]);
  });
});
