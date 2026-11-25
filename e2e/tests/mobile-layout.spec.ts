import { expect, test, type Page } from "@playwright/test";

const USERNAME = process.env.CARDLY_TEST_USER ?? "alice";
const PASSWORD = process.env.CARDLY_TEST_PASS ?? "secret123";

const LONG_DECK = "550+ Phrasal Verbs Parte 1 [Inglés - español]";
const VERY_LONG_DECK = "A".repeat(200);
const NESTED_DECK = "Languages::Spanish::Verbs";
const CJK_DECK = "日本語テスト";
const CYRILLIC_DECK = "Русский язык";
const RTL_DECK = "العربية اختبار";

const DEVICE_MATRIX = [
  { name: "iphone-se", width: 320, height: 568, isPhone: true, hasTouch: true },
  { name: "android-360", width: 360, height: 800, isPhone: true, hasTouch: true },
  { name: "iphone-14", width: 390, height: 844, isPhone: true, hasTouch: true },
  { name: "iphone-15-pro-max", width: 430, height: 932, isPhone: true, hasTouch: true },
  { name: "ipad-portrait", width: 810, height: 1080, isPhone: false, hasTouch: true },
  { name: "ipad-landscape", width: 1080, height: 810, isPhone: false, hasTouch: true },
];

async function login(page: Page) {
  await page.goto("/");
  const loginInput = page.locator('input[autocomplete="username"]');
  const decksNav = page.locator("button.hitem", { hasText: "Decks" });
  await expect(loginInput.or(decksNav)).toBeVisible({ timeout: 15_000 });
  if (await loginInput.isVisible()) {
    await loginInput.fill(USERNAME);
    await page.locator('input[autocomplete="current-password"]').fill(PASSWORD);
    await page.getByRole("button", { name: "Sign In" }).click();
    await expect(decksNav).toBeVisible({ timeout: 15_000 });
  }
}

async function ensureDeckExists(page: Page, deckName: string) {
  const shortName = deckName.split("::").pop() ?? deckName;
  const topName = deckName.split("::")[0];
  const row = page.locator(".deck-row", { hasText: topName.slice(0, 24) });
  const targetRow = page.locator(".deck-row", { hasText: shortName.slice(0, 24) });
  if ((await targetRow.count()) === 0 && (await row.count()) === 0) {
    await page.getByRole("button", { name: "Create Deck" }).click();
    const promptInput = page.locator("input.prompt-input");
    await expect(promptInput).toBeVisible();
    await promptInput.fill(deckName);
    await page.locator("form.dialog-box button[type='submit']").click();
    await expect(row.first()).toBeVisible({ timeout: 10_000 });
  }
}

async function assertNoHorizontalOverflow(page: Page, contextLabel: string) {
  const { scrollW, innerW } = await page.evaluate(() => ({
    scrollW: document.documentElement.scrollWidth,
    innerW: window.innerWidth,
  }));
  expect(scrollW, `${contextLabel}: horizontal overflow (${scrollW} > ${innerW})`).toBeLessThanOrEqual(innerW);
}

test.describe("Mobile & Tablet Responsive Layout", () => {
  test.beforeAll(async ({ browser }) => {
    const seedCtx = await browser.newContext({ viewport: { width: 1280, height: 840 } });
    const seedPage = await seedCtx.newPage();
    await login(seedPage);
    for (const name of [LONG_DECK, VERY_LONG_DECK, NESTED_DECK, CJK_DECK, CYRILLIC_DECK, RTL_DECK]) {
      await ensureDeckExists(seedPage, name);
    }
    await seedCtx.close();
  });

  for (const dev of DEVICE_MATRIX) {
    test(`Deck list, navigation & modals on ${dev.name} (${dev.width}x${dev.height})`, async ({ browser }) => {
      const ctx = await browser.newContext({
        viewport: { width: dev.width, height: dev.height },
        hasTouch: dev.hasTouch,
        isMobile: dev.isPhone,
      });
      const page = await ctx.newPage();
      await login(page);
      await page.waitForTimeout(300);

      // 1. No horizontal overflow on Decks screen
      await assertNoHorizontalOverflow(page, `${dev.name} decks`);

      // 2. Long deck row: button width > 100px, inner text visible, elementFromPoint hits button, gear >= 44px on phone
      const longRow = page.locator(".deck-row", { hasText: "550+ Phrasal Verbs" });
      await expect(longRow).toBeVisible();

      const rowMetrics = await longRow.evaluate((rowEl) => {
        const btn = rowEl.querySelector("button.deck-link") as HTMLElement | null;
        const nameSpan = rowEl.querySelector(".deck-name-text") as HTMLElement | null;
        const gear = rowEl.querySelector("button.gear-btn") as HTMLElement | null;
        if (!btn || !nameSpan || !gear) return null;
        const btnRect = btn.getBoundingClientRect();
        const spanRect = nameSpan.getBoundingClientRect();
        const gearRect = gear.getBoundingClientRect();
        const cx = btnRect.left + btnRect.width / 2;
        const cy = btnRect.top + btnRect.height / 2;
        const hitEl = document.elementFromPoint(cx, cy);
        const hitInsideBtn = hitEl ? btn.contains(hitEl) : false;
        return {
          btnW: btnRect.width,
          btnH: btnRect.height,
          spanW: spanRect.width,
          spanH: spanRect.height,
          gearRight: gearRect.right,
          gearW: gearRect.width,
          gearH: gearRect.height,
          hitInsideBtn,
          innerW: window.innerWidth,
        };
      });

      expect(rowMetrics).not.toBeNull();
      expect(rowMetrics!.btnW).toBeGreaterThan(100);
      expect(rowMetrics!.spanW).toBeGreaterThan(50);
      expect(rowMetrics!.spanH).toBeGreaterThan(10);
      expect(rowMetrics!.hitInsideBtn).toBe(true);
      expect(rowMetrics!.gearRight).toBeLessThanOrEqual(rowMetrics!.innerW);
      if (dev.isPhone) {
        expect(rowMetrics!.btnH).toBeGreaterThanOrEqual(44);
        expect(rowMetrics!.gearW).toBeGreaterThanOrEqual(44);
        expect(rowMetrics!.gearH).toBeGreaterThanOrEqual(44);
      }

      // 3. Mobile bottom tab bar visible on phones, hidden when Add modal opens or input focused
      if (dev.isPhone) {
        const tabBar = page.locator("header.top-bar");
        await expect(tabBar).toBeVisible();
        const tabBox = await tabBar.boundingBox();
        expect(tabBox!.y + tabBox!.height).toBeLessThanOrEqual(dev.height + 1);

        // Open mobile menu sheet
        await page.locator("button.mobile-menu-btn").click();
        const sheet = page.locator(".mobile-sheet");
        await expect(sheet).toBeVisible();
        await assertNoHorizontalOverflow(page, `${dev.name} mobile menu sheet`);
        await page.locator("button.mobile-sheet-close").click();
        await expect(sheet).toBeHidden();
      }

      // 4. Tap the long deck row -> opens Deck Overview
      await longRow.locator("button.deck-link").click();
      await expect(page.locator("h2.deck-heading").or(page.locator(".congrats-wrap"))).toBeVisible({ timeout: 10_000 });
      await assertNoHorizontalOverflow(page, `${dev.name} overview`);

      // 5. Open Preferences modal & verify no overflow + tab bar hidden on phone
      await page.keyboard.press("Control+p");
      const prefModal = page.locator(".modal-window.pref-window");
      await expect(prefModal).toBeVisible();
      await assertNoHorizontalOverflow(page, `${dev.name} preferences`);
      if (dev.isPhone) {
        await expect(page.locator("header.top-bar")).toBeHidden();
      }
      await prefModal.locator(".modal-close").click();
      await expect(prefModal).toBeHidden();

      // 6. Open Browser & verify no document overflow
      await page.locator("button.hitem", { hasText: "Browse" }).click();
      const browserWin = page.locator(".modal-window.browser-window");
      await expect(browserWin).toBeVisible();
      await assertNoHorizontalOverflow(page, `${dev.name} browser`);
      await browserWin.locator(".modal-close").click();
      await expect(browserWin).toBeHidden();

      await ctx.close();
    });
  }

  test("Reviewer More sheet and card layout on phone (390x844)", async ({ browser }) => {
    const ctx = await browser.newContext({
      viewport: { width: 390, height: 844 },
      hasTouch: true,
      isMobile: true,
    });
    const page = await ctx.newPage();
    await login(page);

    // Ensure a card exists in Default deck via Add modal
    await page.locator("button.hitem", { hasText: "Add" }).click();
    const addModal = page.locator(".modal-window.add-window");
    await expect(addModal).toBeVisible();
    await expect(page.locator("header.top-bar")).toBeHidden();
    const editorFrame = addModal.frameLocator("iframe.editor-iframe");
    const richFields = editorFrame.locator('.rich-text-editable [contenteditable="true"]');
    if (await richFields.first().isVisible({ timeout: 10_000 }).catch(() => false)) {
      await richFields.nth(0).click();
      await page.keyboard.type("Mobile Test Q");
      await richFields.nth(1).click();
      await page.keyboard.type("Mobile Test A");
      await editorFrame.getByRole("button", { name: "Add", exact: true }).click();
      await page.waitForTimeout(500);
    }
    await addModal.locator(".modal-close").click();

    const defaultRow = page.locator(".deck-row", { hasText: "Default" }).first();
    await defaultRow.locator("button.deck-link").click();
    const studyBtn = page.getByRole("button", { name: "Study Now" });
    if (await studyBtn.isVisible({ timeout: 3_000 }).catch(() => false)) {
      await studyBtn.click();
      await expect(page.getByRole("button", { name: /Show Answer/ })).toBeVisible();
      await assertNoHorizontalOverflow(page, "iphone-14 reviewer");

      // Bottom tab bar must be hidden in Reviewer
      await expect(page.locator("header.top-bar")).toBeHidden();

      // Open More sheet
      await page.locator("button.more-trigger-btn").click();
      const moreMenu = page.locator(".more-menu");
      await expect(moreMenu).toBeVisible();

      // Verify labels are inside viewport and not clipped offscreen
      const menuBox = await moreMenu.boundingBox();
      expect(menuBox).not.toBeNull();
      expect(menuBox!.x).toBeGreaterThanOrEqual(0);
      expect(menuBox!.x + menuBox!.width).toBeLessThanOrEqual(390);
      await expect(moreMenu).toContainText("Forget...");
      await expect(moreMenu).toContainText("Set Due Date...");
      await expect(moreMenu).toContainText("Delete Note");

      // Close More sheet via close button
      await page.locator("button.more-sheet-close").click();
      await expect(moreMenu).toBeHidden();
    }

    await ctx.close();
  });
});
