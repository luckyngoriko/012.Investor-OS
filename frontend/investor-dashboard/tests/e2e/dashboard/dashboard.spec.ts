import { expect, test } from "../fixtures/warning-budget";
import { loginAsUser } from "../utils/auth";

test.describe("Dashboard", () => {
  test.beforeEach(async ({ page }) => {
    await loginAsUser(page, "trader");
  });

  test("renders core dashboard sections", async ({ page }) => {
    await expect(page.getByRole("heading", { name: /dashboard/i })).toBeVisible();
    await expect(page.getByText(/portfolio value/i)).toBeVisible();
    await expect(page.getByText(/market regime/i)).toBeVisible();
    await expect(page.getByText(/ai trade proposals/i)).toBeVisible();
    await expect(page.getByText(/active positions/i)).toBeVisible();
  });

  test("shows a sync status badge", async ({ page }) => {
    await expect(page.getByText(/not synced|\d{1,2}:\d{2}:\d{2}/i).first()).toBeVisible();
  });

  test("allows navigation to positions from dashboard CTA", async ({ page }) => {
    const cta = page.getByRole("button", { name: /view all positions/i });
    await cta.scrollIntoViewIfNeeded();
    await cta.click({ timeout: 10000 });
    await expect(page).toHaveURL(/\/positions$/);
    await expect(page.getByRole("heading", { name: /portfolio positions/i })).toBeVisible();
  });
});

test.describe("Dashboard - Responsive", () => {
  test("dashboard is usable on mobile viewport", async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await loginAsUser(page, "trader");

    await expect(page.getByRole("heading", { name: /dashboard/i })).toBeVisible();

    const hasHorizontalOverflow = await page.evaluate(() => {
      return document.documentElement.scrollWidth > window.innerWidth;
    });
    expect(hasHorizontalOverflow).toBe(false);
  });
});
