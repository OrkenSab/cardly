import { expect, test } from "@playwright/test";

test.describe("Cardly Demo Mode E2E Suite", () => {
  test.beforeEach(async ({ request }) => {
    const statusRes = await request.get("/api/demo/status").catch(() => null);
    if (!statusRes || statusRes.status() !== 200) {
      test.skip();
      return;
    }
    const data = await statusRes.json().catch(() => ({}));
    if (!data.demo_mode) {
      test.skip();
    }
  });

  test("Zero-Login Guest Access, Sample Decks, Review, Banner & Limits", async ({ page, request }) => {
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(e.message));

    // 1. Visit root - In Demo Mode, click "Continue as Guest" if login view is presented
    await page.goto("/");
    const demoGuestBtn = page.locator("button.demo-btn");
    if (await demoGuestBtn.isVisible({ timeout: 3_000 }).catch(() => false)) {
      await demoGuestBtn.click();
    }

    // Top banner must be visible with demo notice and AGPL/affiliation links
    const banner = page.locator(".demo-banner-top");
    await expect(banner).toBeVisible({ timeout: 15_000 });
    await expect(banner).toContainText(/Demo Mode.*data resets/i);
    await expect(banner.locator("a")).toHaveAttribute("href", "https://github.com/cardly-anki/cardly");
    await expect(banner).toContainText("Not affiliated with Anki / Ankitects");

    // Decks button and DeckBrowser must be visible
    const decksBtn = page.getByRole("button", { name: "Decks", exact: true });
    await expect(decksBtn).toBeVisible();

    // 2. Preloaded sample decks must be present
    const gkDeck = page.locator(".deck-row", { hasText: "General Knowledge" });
    const langDeck = page.locator(".deck-row", { hasText: "Languages" });
    const sciDeck = page.locator(".deck-row", { hasText: "Science & Mathematics" });

    await expect(gkDeck).toBeVisible();
    await expect(langDeck).toBeVisible();
    await expect(sciDeck).toBeVisible();

    // 3. Complete a card review in General Knowledge
    await gkDeck.locator(".deck-link").click();
    const studyBtn = page.getByRole("button", { name: "Study Now" });
    await expect(studyBtn).toBeVisible();
    await studyBtn.click();

    // Reviewer: Question iframe visible
    const cardFrame = page.frameLocator("iframe.card-iframe");
    await expect(cardFrame.locator("#qa")).toBeVisible({ timeout: 15_000 });

    // Show answer
    const showAnsBtn = page.getByRole("button", { name: /Show Answer/ });
    await expect(showAnsBtn).toBeVisible();
    await showAnsBtn.click();

    // Ease button Good
    const goodBtn = page.locator("button.ease-btn", { hasText: "Good" });
    await expect(goodBtn).toBeVisible();
    await goodBtn.click();

    // Wait briefly for card answering to resolve before returning to decks list
    await page.waitForTimeout(600);
    await page.getByRole("button", { name: "Decks", exact: true }).click();
    await expect(gkDeck).toBeVisible({ timeout: 15_000 });

    // 4. Test Demo Footer AGPL disclaimer
    const footerInfo = page.locator(".demo-footer-info");
    await expect(footerInfo).toBeVisible();
    await expect(footerInfo).toContainText("Not affiliated with Anki / Ankitects");

    // 5. Test Demo Status API Endpoint
    const statusRes = await request.get("/api/demo/status");
    expect(statusRes.status()).toBe(200);
    const statusData = await statusRes.json();
    expect(statusData.demo_mode).toBe(true);
    expect(statusData.reset_interval_minutes).toBeGreaterThan(0);

    // 6. Verify Abuse Limits: AI generation is blocked in demo mode
    const aiRes = await request.post("/api/ai/generate", {
      data: {
        text: "Sample notes to turn into flashcards",
        mode: "basic",
      },
    });
    // In demo mode without demo_ai_enabled, AI endpoint returns 403 Forbidden
    expect([401, 403]).toContain(aiRes.status());

    // 7. Verify Abuse Limits: Full collection (.colpkg) import is blocked
    const importRes = await request.post("/api/import/colpkg", {
      data: { upload_id: "fake-test-id" },
    });
    expect([401, 403]).toContain(importRes.status());

    expect(errors).toEqual([]);
  });
});
