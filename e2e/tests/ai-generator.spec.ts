import { test, expect } from "@playwright/test";

const USERNAME = process.env.CARDLY_TEST_USER ?? "alice";
const PASSWORD = process.env.CARDLY_TEST_PASS ?? "secret123";

async function login(page: import("@playwright/test").Page) {
  await page.goto("/");
  const loginHeading = page.locator("h1", { hasText: "Cardly" });
  const decksBtn = page.getByRole("button", { name: "Decks" });
  await expect(loginHeading.or(decksBtn)).toBeVisible({ timeout: 15_000 });

  if (await page.locator('input[autocomplete="username"]').isVisible()) {
    await page.locator('input[autocomplete="username"]').fill(USERNAME);
    await page.locator('input[autocomplete="current-password"]').fill(PASSWORD);
    await page.getByRole("button", { name: "Sign In" }).click();
    await expect(decksBtn).toBeVisible({ timeout: 15_000 });
  }
}

test.describe("AI Card Generator E2E Suite", () => {
  test("Open AI Generator -> Configure -> Generate -> Review/Edit -> Batch Add", async ({ page }) => {
    // Intercept /api/ai/generate with mock responses for hermetic testing
    await page.route("**/api/ai/generate", async (route) => {
      const requestData = JSON.parse(route.request().postData() || "{}");
      const mode = requestData.mode || "basic";
      if (mode === "cloze") {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            cards: [
              {
                cloze: "The {{c1::hippocampus}} is essential for the consolidation of short-term memory into long-term memory.",
                tags: ["neuroscience", "memory"],
              },
              {
                cloze: "Spaced repetition intervals are computed using the {{c1::FSRS}} algorithm.",
                tags: ["fsrs", "learning"],
              },
            ],
          }),
        });
      } else {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            cards: [
              {
                front: "What is the primary function of the hippocampus?",
                back: "Memory consolidation (converting short-term memory to long-term memory).",
                tags: ["neuroscience", "biology"],
              },
              {
                front: "What does FSRS stand for in spaced repetition?",
                back: "Free Spaced Repetition Scheduler.",
                tags: ["anki", "fsrs"],
              },
              {
                front: "Which neurotransmitter is most closely linked to reward-motivated behavior?",
                back: "Dopamine.",
                tags: ["neuroscience"],
              },
            ],
          }),
        });
      }
    });

    // Intercept /api/ai/batch-add
    await page.route("**/api/ai/batch-add", async (route) => {
      const requestData = JSON.parse(route.request().postData() || "{}");
      const count = Array.isArray(requestData.cards) ? requestData.cards.length : 0;
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          added_count: count,
        }),
      });
    });

    await login(page);

    // 1. Open AI Generator via toolbar button
    const aiBtn = page.getByRole("button", { name: /AI/ });
    await expect(aiBtn).toBeVisible({ timeout: 10_000 });
    await aiBtn.click();

    const aiModal = page.locator(".ai-generator-window");
    await expect(aiModal).toBeVisible();
    await expect(aiModal.locator("#ai-modal-title")).toContainText("AI Card Generator");

    // 2. Switch between Text and Image OCR tabs
    const imageTab = page.getByRole("tab", { name: /Photo/i });
    await imageTab.click();
    await expect(page.locator(".image-dropzone")).toBeVisible();

    const textTab = page.getByRole("tab", { name: /Text/i });
    await textTab.click();
    await expect(page.locator("#ai-source-text")).toBeVisible();

    // 3. Fill in notes text
    await page.locator("#ai-source-text").fill(
      "The hippocampus is a major component of the brains of humans and other vertebrates. Humans and other mammals have two hippocampi, one in each side of the brain. The hippocampus is part of the limbic system, and plays important roles in the consolidation of information from short-term memory to long-term memory."
    );

    // 4. Configure settings
    await page.locator("#ai-card-type").selectOption("basic");

    // 5. Toggle AI Provider settings
    const providerToggle = page.locator(".provider-toggle-btn");
    await providerToggle.click();
    await expect(page.locator(".provider-settings-panel")).toBeVisible();
    await page.locator("#ai-api-key").fill("test-mock-key");

    // 6. Click Generate Flashcards
    const generateBtn = page.getByRole("button", { name: /Generate Flashcards/i });
    await generateBtn.click();

    // 7. Verify Review & Edit View
    await expect(page.locator(".cards-review-list")).toBeVisible({ timeout: 15_000 });
    const cardItems = page.locator(".review-card-item");
    await expect(cardItems).toHaveCount(3);

    // Verify first card fields are populated and editable
    const firstFront = cardItems.first().locator(".card-front-input");
    await expect(firstFront).toHaveValue("What is the primary function of the hippocampus?");
    await firstFront.fill("What is the main function of the human hippocampus?");

    // 8. Delete the 3rd card
    const deleteBtn = cardItems.nth(2).locator(".btn-delete-card");
    await deleteBtn.click();
    await expect(cardItems).toHaveCount(2);

    // 9. Batch Add Cards to Deck
    const addBatchBtn = page.getByRole("button", { name: /Add 2 Cards to Deck/i });
    await expect(addBatchBtn).toBeVisible();
    await addBatchBtn.click();

    // 10. Verify modal closed and success toast appeared
    await expect(aiModal).toBeHidden();
    const toast = page.locator(".toast-pill").first();
    await expect(toast).toContainText(/Added 2 AI flashcards/i);
  });
});
