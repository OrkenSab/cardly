import { test } from "@playwright/test";
import path from "path";

const USERNAME = process.env.CARDLY_TEST_USER ?? "alice";
const PASSWORD = process.env.CARDLY_TEST_PASS ?? "secret123";
const SCREENSHOTS_DIR = path.resolve(__dirname, "../../docs/screenshots");

async function loginMobile(page: import("@playwright/test").Page) {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  if (await page.locator('input[autocomplete="username"]').isVisible({ timeout: 4000 }).catch(() => false)) {
    await page.locator('input[autocomplete="username"]').fill(USERNAME);
    await page.locator('input[autocomplete="current-password"]').fill(PASSWORD);
    await page.getByRole("button", { name: "Sign In" }).click();
    await page.waitForTimeout(1000);
  }
}

test("Generate mobile screenshots gallery", async ({ page }) => {
  await loginMobile(page);

  // 1. Mobile Deck List (Light)
  await page.waitForTimeout(600);
  await page.screenshot({ path: path.join(SCREENSHOTS_DIR, "14-mobile-deck-list-light.png") });

  // Add a test card first so reviewer has content
  await page.getByRole("button", { name: "Add" }).click();
  const addModal = page.locator(".modal-window.add-window");
  await addModal.waitFor({ state: "visible" });
  const editorFrame = addModal.frameLocator("iframe.editor-iframe");
  const richFields = editorFrame.locator('.rich-text-editable [contenteditable="true"]');
  await richFields.first().waitFor({ state: "visible", timeout: 15_000 });
  await richFields.nth(0).click();
  await page.keyboard.type("Mobile Spaced Repetition");
  await richFields.nth(1).click();
  await page.keyboard.type("Cardly works beautifully on phone!");
  await editorFrame.getByRole("button", { name: "Add", exact: true }).click();
  await page.waitForTimeout(800);
  await addModal.locator(".modal-close").click();
  await addModal.waitFor({ state: "hidden" });

  // 2. Mobile Deck Overview (Light)
  const defaultRow = page.locator(".deck-row", { hasText: "Default" });
  await defaultRow.locator(".deck-link").click();
  await page.waitForTimeout(600);
  await page.screenshot({ path: path.join(SCREENSHOTS_DIR, "15-mobile-deck-overview-light.png") });

  // 3. Mobile Reviewer (Question)
  const studyBtn = page.getByRole("button", { name: "Study Now" });
  await studyBtn.click();
  await page.waitForTimeout(800);
  await page.screenshot({ path: path.join(SCREENSHOTS_DIR, "16-mobile-reviewer-question-light.png") });

  // 4. Mobile Reviewer (Answer)
  const showAnsBtn = page.getByRole("button", { name: /Show Answer/ });
  await showAnsBtn.click();
  await page.waitForTimeout(600);
  await page.screenshot({ path: path.join(SCREENSHOTS_DIR, "17-mobile-reviewer-answer-light.png") });

  // 5. Mobile Reviewer (Dark Mode)
  await page.locator("button.theme-toggle").click();
  await page.waitForTimeout(600);
  await page.screenshot({ path: path.join(SCREENSHOTS_DIR, "18-mobile-reviewer-dark.png") });

  // Switch back to light
  await page.locator("button.theme-toggle").click();
  await page.waitForTimeout(300);

  // 6. Mobile Browser
  await page.getByRole("button", { name: "Browse" }).click();
  await page.waitForTimeout(600);
  await page.screenshot({ path: path.join(SCREENSHOTS_DIR, "19-mobile-browser-light.png") });
});
