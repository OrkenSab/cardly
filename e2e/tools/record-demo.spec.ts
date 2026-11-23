import { test, expect } from "@playwright/test";
import path from "path";
import fs from "fs";

const USERNAME = process.env.CARDLY_TEST_USER ?? "alice";
const PASSWORD = process.env.CARDLY_TEST_PASS ?? "secret123";

test("Record 30-45s showcase demo video", async ({ browser }) => {
  test.setTimeout(90_000);

  const rawVideoDir = path.resolve(process.cwd(), "docs/demo/raw");
  if (!fs.existsSync(rawVideoDir)) {
    fs.mkdirSync(rawVideoDir, { recursive: true });
  }

  const context = await browser.newContext({
    viewport: { width: 1280, height: 800 },
    recordVideo: {
      dir: rawVideoDir,
      size: { width: 1280, height: 800 },
    },
  });

  const page = await context.newPage();
  const sleep = (ms: number) => page.waitForTimeout(ms);

  // 1. Navigation & Login
  await page.goto("/");
  await sleep(1000);

  const usernameInput = page.locator('input[autocomplete="username"]');
  if (await usernameInput.isVisible()) {
    await usernameInput.click();
    await usernameInput.pressSequentially(USERNAME, { delay: 60 });
    await sleep(400);

    const passwordInput = page.locator('input[autocomplete="current-password"]');
    await passwordInput.click();
    await passwordInput.pressSequentially(PASSWORD, { delay: 60 });
    await sleep(500);

    await page.getByRole("button", { name: "Sign In" }).click();
  }

  // Verify and pause on Deck Browser (Main Screen)
  const decksBtn = page.getByRole("button", { name: "Decks" });
  await expect(decksBtn).toBeVisible({ timeout: 10_000 });
  await sleep(1800);

  // 2. Add Card Dialog (Rich text & Cloze)
  const addBtn = page.getByRole("button", { name: "Add", exact: true });
  await addBtn.click();

  const addModal = page.locator(".modal-window.add-window");
  await expect(addModal).toBeVisible();
  await sleep(1000);

  // Interact with upstream editor iframe
  const editorFrame = addModal.frameLocator("iframe.editor-iframe");
  const richFields = editorFrame.locator('.rich-text-editable [contenteditable="true"]');
  await expect(richFields.first()).toBeVisible({ timeout: 15_000 });

  await richFields.nth(0).click();
  await sleep(400);
  await page.keyboard.type("What is the capital of France?", { delay: 45 });
  await sleep(700);

  if (await richFields.nth(1).isVisible()) {
    await richFields.nth(1).click();
    await sleep(400);
    await page.keyboard.type("Paris (The City of Light)", { delay: 45 });
    await sleep(1000);
  }

  // Click Add button inside editor
  const submitAddBtn = editorFrame.getByRole("button", { name: "Add", exact: true });
  await submitAddBtn.click();
  await sleep(1200);

  // Close Add modal
  await addModal.locator(".modal-close").click();
  await expect(addModal).toBeHidden();
  await sleep(1500);

  // 3. Reviewer Flow (Question -> Show Answer -> Rate Good)
  const defaultRow = page.locator(".deck-row", { hasText: "Default" });
  if (await defaultRow.isVisible()) {
    await defaultRow.locator(".deck-link").click();
    await sleep(1500);

    const studyNowBtn = page.getByRole("button", { name: "Study Now" });
    if (await studyNowBtn.isVisible()) {
      await studyNowBtn.click();
      await sleep(2500);

      // Question is visible in reviewer frame
      const showAnswerBtn = page.getByRole("button", { name: /Show Answer/i });
      if (await showAnswerBtn.isVisible()) {
        await showAnswerBtn.click();
        await sleep(2500);

        // Rating buttons
        const goodBtn = page.locator("button.ease-btn", { hasText: "Good" });
        if (await goodBtn.isVisible()) {
          await goodBtn.click();
          await sleep(1800);
        }
      }
    }
  }

  // Return to Deck Browser
  await decksBtn.click();
  await sleep(1200);

  // 4. Card Browser (3-pane layout, search, filter, selection)
  const browseBtn = page.getByRole("button", { name: "Browse" });
  await browseBtn.click();

  const browserWin = page.locator(".browser-window");
  await expect(browserWin).toBeVisible({ timeout: 10_000 });
  await sleep(1800);

  const searchInput = browserWin.locator('input[placeholder*="Search"]');
  if (await searchInput.isVisible()) {
    await searchInput.click();
    await searchInput.fill("");
    await searchInput.pressSequentially("France", { delay: 70 });
    await sleep(600);
    await searchInput.press("Enter");
    await sleep(1800);
  }

  // Select first row to display note editor pane
  const rows = browserWin.locator(".grid-row");
  const rowCount = await rows.count();
  if (rowCount > 0) {
    await rows.first().click();
    await sleep(2500);
  }

  // Close browser modal
  await browserWin.locator(".modal-close").click();
  await expect(browserWin).toBeHidden();
  await sleep(1200);

  // 5. Statistics (upstream /graphs)
  const statsBtn = page.getByRole("button", { name: "Stats" });
  await statsBtn.click();

  const statsModal = page.locator(".modal-window.stats-window");
  await expect(statsModal).toBeVisible({ timeout: 10_000 });
  await sleep(4000);

  // Close stats modal
  await statsModal.locator(".modal-close").click();
  await expect(statsModal).toBeHidden();
  await sleep(1800);

  // Close page and context to ensure video recording is saved to disk
  const video = page.video();
  await page.close();
  await context.close();

  if (video) {
    const videoPath = await video.path();
    console.log(`Video recorded successfully at: ${videoPath}`);
  }
});
