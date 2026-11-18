import { expect, test } from "@playwright/test";

const USERNAME = process.env.CARDLY_TEST_USER ?? "alice";
const PASSWORD = process.env.CARDLY_TEST_PASS ?? "secret123";

async function login(page: import("@playwright/test").Page) {
  await page.goto("/");
  // Wait for either Login form or Decks toolbar
  const loginHeading = page.locator("h1", { hasText: "Cardly" });
  const decksBtn = page.getByRole("button", { name: "Decks" });
  await expect(loginHeading.or(decksBtn)).toBeVisible();

  if (await page.locator('input[autocomplete="username"]').isVisible()) {
    await page.locator('input[autocomplete="username"]').fill(USERNAME);
    await page.locator('input[autocomplete="current-password"]').fill(PASSWORD);
    await page.getByRole("button", { name: "Sign In" }).click();
    await expect(decksBtn).toBeVisible();
  }
}

test.describe("Cardly Phase 2 E2E", () => {
  test("Flow 1: Login -> Add Card -> Review -> Scheduling & Undo", async ({ page }) => {
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(e.message));

    await login(page);

    // Verify Cardly branding (no "Anki" in top shell)
    await expect(page.locator("nav.menubar")).not.toContainText("Anki");
    await expect(page.locator("header.top-bar")).not.toContainText("Anki");

    // 1. Open Add Cards modal (`Add` button in top toolbar)
    await page.getByRole("button", { name: "Add", exact: true }).click();
    const addModal = page.locator(".modal-window.add-window");
    await expect(addModal).toBeVisible();

    // Interact with the real upstream `ts/routes/editor` inside iframe
    const editorFrame = addModal.frameLocator("iframe.editor-iframe");
    const richFields = editorFrame.locator('.rich-text-editable [contenteditable="true"]');
    await expect(richFields.first()).toBeVisible({ timeout: 20_000 });

    await richFields.nth(0).click();
    await page.keyboard.type("What is the capital of France?");
    await richFields.nth(1).click();
    await page.keyboard.type("Paris");

    // Click upstream Add button inside editor iframe
    await editorFrame.getByRole("button", { name: "Add", exact: true }).click();

    // Wait briefly for addNote RPC to complete, then close Add modal
    await expect(richFields.nth(0)).toHaveText("", { timeout: 10_000 });
    await addModal.locator(".modal-close").click();
    await expect(addModal).toBeHidden();

    // 2. Verify Default deck shows 1 New card in Deck Browser
    const defaultRow = page.locator(".deck-row", { hasText: "Default" });
    await expect(defaultRow).toBeVisible();
    await expect(defaultRow.locator('td[data-col="new"]')).toHaveText("1");

    // 3. Click Default -> Deck Overview -> Study Now
    await defaultRow.locator(".deck-link").click();
    await expect(page.getByRole("button", { name: "Study Now" })).toBeVisible();
    await page.getByRole("button", { name: "Study Now" }).click();

    // 4. Reviewer: Question visible inside isolated card iframe
    const cardFrame = page.frameLocator("iframe.card-iframe");
    await expect(cardFrame.locator("#qa")).toContainText("What is the capital of France?");

    // Click Show Answer to reveal answer
    await page.getByRole("button", { name: /Show Answer/ }).click();
    await expect(cardFrame.locator("#qa")).toContainText("Paris");

    // Verify 4 rating buttons + interval previews exist
    const goodBtn = page.locator("button.ease-btn", { hasText: "Good" });
    await expect(page.locator("button.ease-btn", { hasText: "Again" })).toBeVisible();
    await expect(page.locator("button.ease-btn", { hasText: "Hard" })).toBeVisible();
    await expect(goodBtn).toBeVisible();
    await expect(page.locator("button.ease-btn", { hasText: "Easy" })).toBeVisible();

    // Rate Good
    await goodBtn.click();

    // 5. After answering the only due card, check Deck list
    await page.getByRole("button", { name: "Decks", exact: true }).click();
    await expect(defaultRow).toBeVisible();
    // New count must now be 0 (moved to Learn or Done)
    await expect(defaultRow.locator('td[data-col="new"]')).toHaveText("0");
    // Studied today footer should mention at least 1 card
    await expect(page.locator(".studied-today")).toContainText(/Studied\s+.*1.*card/i);

    // 6. Undo with Ctrl+Z should restore New count to 1
    await page.keyboard.press("Control+z");
    await expect(defaultRow.locator('td[data-col="new"]')).toHaveText("1");

    expect(errors).toEqual([]);
  });

  test("Flow 2: Import CSV -> Browse -> Search -> Bulk Tag/Flag/Suspend", async ({ page }) => {
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(e.message));

    await login(page);

    // 1. Open File -> Import and upload a CSV file with 2 notes
    await page.getByRole("button", { name: "File", exact: true }).click();
    await page.getByRole("menuitem", { name: /Import/ }).click();
    const importModal = page.locator(".modal-window").filter({ hasText: "Import File" });
    await expect(importModal).toBeVisible();

    const csvContent = "#separator:tab\n#tags column:3\nBonjour\tHello\tfrench\nMerci\tThank you\tfrench\n";
    await importModal.locator('input[type="file"]').setInputFiles({
      name: "french-vocab.tsv",
      mimeType: "text/tab-separated-values",
      buffer: Buffer.from(csvContent, "utf-8"),
    });

    // Upstream CSV importer iframe loads inside ImportModal
    const csvFrame = page.locator(".modal-window.frame-window iframe").contentFrame();
    const importBtn = csvFrame.getByRole("button", { name: /^Import$/i });
    await expect(importBtn).toBeVisible({ timeout: 20_000 });
    await importBtn.click();

    // Wait for upstream import completion report and close modal
    await expect(csvFrame.locator("body")).toContainText(/new notes imported|notes.*found in file/i, {
      timeout: 20_000,
    });
    await page.locator(".modal-window.frame-window .modal-close").click();
    await expect(page.locator(".modal-window.frame-window")).toBeHidden();

    // 2. Open Card Browser (`Browse` button in top toolbar)
    await page.getByRole("button", { name: "Browse" }).click();
    const browserWin = page.locator(".browser-window");
    await expect(browserWin).toBeVisible();

    // 3. Search for imported tag `tag:french`
    const searchInput = browserWin.locator('input[placeholder*="Search"]');
    await searchInput.fill("tag:french");
    await searchInput.press("Enter");

    // Expect 2 rows matching Bonjour & Merci
    const rows = browserWin.locator(".grid-row");
    await expect(rows).toHaveCount(2, { timeout: 15_000 });

    // Select first row and set Red Flag via Cards menu
    await rows.first().click();
    await browserWin.getByRole("button", { name: "Cards ▾" }).click();
    await browserWin.locator('.flag-chip[title="Red"]').click();

    // Search for `flag:1` and verify 1 row matches
    await searchInput.fill("flag:1");
    await searchInput.press("Enter");
    await expect(rows).toHaveCount(1, { timeout: 10_000 });

    // Toggle suspend via Cards menu -> verify `is:suspended` finds 1 card
    await rows.first().click();
    await browserWin.getByRole("button", { name: "Cards ▾" }).click();
    await browserWin.getByRole("button", { name: /Toggle Suspend/ }).click();

    await searchInput.fill("is:suspended");
    await searchInput.press("Enter");
    await expect(rows).toHaveCount(1, { timeout: 10_000 });

    await browserWin.locator(".modal-close").click();
    await expect(browserWin).toBeHidden();

    expect(errors).toEqual([]);
  });

  test("Flow 3: Statistics, Deck Options (FSRS), Templates & Dark Mode Preferences", async ({ page }) => {
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(e.message));

    await login(page);

    // 1. Statistics (upstream /graphs)
    await page.getByRole("button", { name: "Stats" }).click();
    const statsModal = page.locator(".modal-window.stats-window");
    await expect(statsModal).toBeVisible();
    await expect(statsModal.frameLocator("iframe").locator("body")).toContainText(/Future Due|Card Counts|Calendar/i, {
      timeout: 20_000,
    });
    await statsModal.locator(".modal-close").click();

    // 2. Deck Options including FSRS (upstream /deck-options/1)
    await page.locator(".deck-row", { hasText: "Default" }).locator(".gear-btn").click();
    await page.locator(".context-menu").getByRole("button", { name: "Options" }).click();
    const optsModal = page.locator(".modal-window.deck-opts-window");
    await expect(optsModal).toBeVisible();
    await expect(optsModal.frameLocator("iframe").locator("body")).toContainText(/FSRS/i, {
      timeout: 20_000,
    });
    await optsModal.locator(".modal-close").click();

    // 3. Manage Note Types -> Card Templates preview
    await page.getByRole("button", { name: "Tools" }).click();
    await page.getByRole("menuitem", { name: /Manage Note Types/ }).click();
    const ntModal = page.locator(".modal-window.nt-window");
    await expect(ntModal).toBeVisible();
    await ntModal.getByRole("button", { name: "Cards..." }).click();
    const tmplModal = page.locator(".modal-window.tmpl-window");
    await expect(tmplModal).toBeVisible();
    await expect(tmplModal.locator("textarea.code-editor")).toHaveValue("{{Front}}");
    await tmplModal.locator(".modal-close").click();
    await ntModal.locator(".modal-close").click();

    // 4. Preferences -> Dark Mode toggle
    await page.getByRole("button", { name: "Tools" }).click();
    await page.getByRole("menuitem", { name: /Preferences/ }).click();
    const prefModal = page.locator(".modal-window.pref-window");
    await expect(prefModal).toBeVisible();
    await prefModal.locator("#pref-theme").selectOption("dark");
    await prefModal.getByRole("button", { name: "Close & Save" }).click();
    await expect(page.locator("html")).toHaveClass(/night-mode/);

    // Switch back to Light mode
    await page.getByRole("button", { name: "Tools" }).click();
    await page.getByRole("menuitem", { name: /Preferences/ }).click();
    await page.locator("#pref-theme").selectOption("light");
    await page.getByRole("button", { name: "Close & Save" }).click();
    await expect(page.locator("html")).not.toHaveClass(/night-mode/);

    expect(errors).toEqual([]);
  });

  test("Flow 4: Mobile Responsive Viewport, PWA Manifest & Touch Review", async ({ page }) => {
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(e.message));

    // 1. Set mobile smartphone viewport (iPhone 14 standard: 390x844)
    await page.setViewportSize({ width: 390, height: 844 });

    await login(page);

    // 2. Verify PWA manifest and icons are present in DOM and accessible
    const manifestLink = page.locator('link[rel="manifest"]');
    await expect(manifestLink).toHaveAttribute("href", "/manifest.webmanifest");
    const manifestRes = await page.request.get("/manifest.webmanifest");
    expect(manifestRes.status()).toBe(200);
    const manifestJson = await manifestRes.json();
    expect(manifestJson.name).toBe("Cardly");
    expect(manifestJson.display).toBe("standalone");

    // Check service worker script and icons
    const swRes = await page.request.get("/sw.js");
    expect(swRes.status()).toBe(200);
    const iconRes = await page.request.get("/icon-192.png");
    expect(iconRes.status()).toBe(200);

    // 3. Verify mobile TopToolbar layout
    const toolbar = page.locator("header.top-bar .toolbar");
    await expect(toolbar).toBeVisible();
    const toolbarBox = await toolbar.boundingBox();
    expect(toolbarBox).not.toBeNull();
    // Toolbar should span almost the entire 390px mobile width
    expect(toolbarBox!.width).toBeGreaterThan(340);

    // 4. Verify Deck list on mobile
    const defaultRow = page.locator(".deck-row", { hasText: "Default" });
    await expect(defaultRow).toBeVisible();

    // 5. Open Deck Overview and start Review
    await defaultRow.locator(".deck-link").click();
    const studyBtn = page.getByRole("button", { name: "Study Now" });
    await expect(studyBtn).toBeVisible();
    await studyBtn.click();

    // 6. Reviewer on mobile: bottom bar should adapt
    const showAnsBtn = page.getByRole("button", { name: /Show Answer/ });
    await expect(showAnsBtn).toBeVisible();
    const showAnsBox = await showAnsBtn.boundingBox();
    expect(showAnsBox).not.toBeNull();
    // Button should be large and full-width on mobile (> 300px)
    expect(showAnsBox!.width).toBeGreaterThan(300);

    // Reveal answer and test ease buttons on mobile
    await showAnsBtn.click();
    const goodBtn = page.locator("button.ease-btn", { hasText: "Good" });
    await expect(goodBtn).toBeVisible();
    const goodBox = await goodBtn.boundingBox();
    expect(goodBox).not.toBeNull();
    // Touch targets should have comfortable height (>= 40px)
    expect(goodBox!.height).toBeGreaterThanOrEqual(40);

    // Click Good button
    await goodBtn.click();

    // Return to decks
    await page.getByRole("button", { name: "Decks", exact: true }).click();
    await expect(defaultRow).toBeVisible();

    expect(errors).toEqual([]);
  });
});
