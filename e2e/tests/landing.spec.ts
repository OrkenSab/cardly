import { test, expect } from "@playwright/test";
import path from "path";

test("Landing Page Visual Test & Interaction Suite", async ({ page }) => {
  const landingPath = path.resolve(process.cwd(), "../landing/index.html");
  const landingUrl = `file://${landingPath}`;

  // 1. Desktop Viewport
  await page.setViewportSize({ width: 1280, height: 840 });
  await page.goto(landingUrl);

  // Check branding & Hero elements
  await expect(page.locator(".brand-name")).toHaveText("Cardly");
  await expect(page.locator(".hero-title")).toBeVisible();
  await expect(page.locator(".badge-row")).toBeVisible();

  // Verify default language is English
  await expect(page.locator(".nav-menu")).toContainText("Features");
  await expect(page.locator(".hero-title")).toContainText("The Open-Source Spaced Repetition App");

  // 2. Multilingual Switcher: Russian
  await page.locator('.lang-btn[data-lang="ru"]').click();
  await expect(page.locator(".nav-menu")).toContainText("Возможности");
  await expect(page.locator(".hero-title")).toContainText("Веб-приложение интервального повторения");
  await expect(page.locator(".cta-btn").first()).toContainText("Открыть демо-версию");

  // Multilingual Switcher: Kazakh
  await page.locator('.lang-btn[data-lang="kk"]').click();
  await expect(page.locator(".nav-menu")).toContainText("Мүмкіндіктер");
  await expect(page.locator(".hero-title")).toContainText("интервалды қайталау жүйесі");
  await expect(page.locator(".cta-btn").first()).toContainText("Демо нұсқасын көру");

  // Switch back to English
  await page.locator('.lang-btn[data-lang="en"]').click();
  await expect(page.locator(".nav-menu")).toContainText("Features");

  // 3. Theme Toggle: Dark -> Light -> Dark
  const html = page.locator("html");
  await expect(html).toHaveAttribute("data-theme", "dark");
  await page.locator("#themeToggle").click();
  await expect(html).toHaveAttribute("data-theme", "light");
  await page.locator("#themeToggle").click();
  await expect(html).toHaveAttribute("data-theme", "dark");

  // 4. Copy Button
  await page.locator("#copyBtn").click();
  await expect(page.locator("#toast")).toHaveClass(/show/);

  // 5. Interactive FAQ Accordion
  const firstAccordion = page.locator(".accordion-trigger").first();
  await expect(firstAccordion).toHaveAttribute("aria-expanded", "false");
  await firstAccordion.click();
  await expect(firstAccordion).toHaveAttribute("aria-expanded", "true");
  await expect(page.locator(".accordion-panel").first()).toBeVisible();

  // 6. Visual Screenshot Capture
  const rootScreenshotsDir = path.resolve(__dirname, "../../docs/screenshots");
  await page.screenshot({ path: path.join(rootScreenshotsDir, "20-landing-desktop.png"), fullPage: false });

  // 7. Mobile Viewport (iPhone 390x844)
  await page.setViewportSize({ width: 390, height: 844 });
  await page.waitForTimeout(400);
  await expect(page.locator(".brand-name")).toBeVisible();
  await expect(page.locator(".hero-title")).toBeVisible();
  await page.screenshot({ path: path.join(rootScreenshotsDir, "21-landing-mobile.png"), fullPage: false });
});
