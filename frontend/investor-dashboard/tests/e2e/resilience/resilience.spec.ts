import { expect, test } from "../fixtures/warning-budget";
import { loginAsDemo } from "../utils/auth";

test.describe("Resilience - Frontend Degradation", () => {
  test("dashboard stays usable when proposal API fails", async ({ page }) => {
    await page.route("**/api/proposals", async (route) => {
      await route.fulfill({
        status: 500,
        contentType: "application/json",
        body: JSON.stringify({ success: false, error: "Proposals unavailable" }),
      });
    });

    await loginAsDemo(page, "trader");

    await expect(page.getByRole("heading", { name: /dashboard/i })).toBeVisible();
    await expect(page.getByText(/ai trade proposals/i)).toBeVisible();
  });

  test("monitoring page shows warning banner when health API degrades", async ({ page }) => {
    await loginAsDemo(page, "trader");

    await page.route("**/api/health", async (route) => {
      await route.fulfill({
        status: 503,
        contentType: "application/json",
        body: JSON.stringify({ success: false, error: "Health endpoint unavailable" }),
      });
    });

    await page.goto("/monitoring");
    await expect(page.getByRole("heading", { name: /performance monitoring/i })).toBeVisible();
    await expect(page.getByText(/health endpoint unavailable|failed to load monitoring data/i)).toBeVisible();
  });

  test("settings page surfaces API failures without crashing", async ({ page }) => {
    await loginAsDemo(page, "trader");

    await page.route("**/api/runtime/config", async (route) => {
      await route.fulfill({
        status: 500,
        contentType: "application/json",
        body: JSON.stringify({ success: false, error: "Runtime config unavailable" }),
      });
    });

    await page.goto("/settings");
    await expect(page.getByRole("heading", { level: 1, name: /settings/i })).toBeVisible();
    await expect(page.getByText(/kill switch/i).first()).toBeVisible();
  });
});
