import { expect, test } from "../fixtures/warning-budget";
import { loginAsUser } from "../utils/auth";

const ADMIN_PASSWORD = process.env.E2E_AUTH_PASSWORD_ADMIN ?? "Admin#2026!";

test.describe("Accessibility - Login", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/login");
  });

  test("login form inputs are present and editable", async ({ page }) => {
    const emailInput = page.locator('input[type="email"]').first();
    const passwordInput = page.locator('input[type="password"]').first();
    const submitButton = page.getByRole("button", { name: /sign in/i });

    await expect(emailInput).toBeVisible();
    await expect(passwordInput).toBeVisible();
    await expect(submitButton).toBeVisible();

    await emailInput.fill("admin@investor-os.com");
    await expect(emailInput).toHaveValue(/@investor-os\.com$/i);
    await passwordInput.fill(ADMIN_PASSWORD);
    await expect(passwordInput).toHaveValue(/.+/);
  });

  test("submit control has clear accessible name", async ({ page }) => {
    await expect(page.getByRole("button", { name: /sign in/i })).toBeVisible();
  });
});

test.describe("Accessibility - Dashboard", () => {
  test.beforeEach(async ({ page }) => {
    await loginAsUser(page, "trader");
  });

  test("main heading exists exactly once", async ({ page }) => {
    await expect(page.locator("h1")).toHaveCount(1);
    await expect(page.getByRole("heading", { level: 1, name: /dashboard/i })).toBeVisible();
  });

  test("tabular content includes headers", async ({ page }) => {
    const tables = page.locator("table");
    const count = await tables.count();

    for (let i = 0; i < count; i += 1) {
      const headerCount = await tables.nth(i).locator("th").count();
      expect(headerCount).toBeGreaterThan(0);
    }
  });

  test("keyboard shortcut opens and closes command palette", async ({ page }) => {
    const projectName = test.info().project.name;
    test.skip(
      projectName.includes("mobile") || projectName.includes("tablet"),
      "Keyboard shortcut coverage is not applicable to touch-only mobile/tablet projects.",
    );

    const search = page.getByPlaceholder(/search commands, pages, or actions/i);

    for (const shortcut of ["Control+k", "Meta+k"]) {
      await page.keyboard.press(shortcut);
      if (await search.isVisible().catch(() => false)) {
        break;
      }
    }

    await expect(search).toBeVisible();

    await page.keyboard.press("Escape");
    await expect(search).not.toBeVisible();
  });
});
