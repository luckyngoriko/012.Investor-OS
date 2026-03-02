import { expect, test } from "../fixtures/warning-budget";
import { loginAsUser } from "../utils/auth";

test.describe("Authentication", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/login");
    await expect(page.getByRole("heading", { name: /welcome back/i })).toBeVisible();
  });

  test("successful login with valid credentials", async ({ page }) => {
    await loginAsUser(page, "trader");

    await expect(page).toHaveURL("/");
    await expect(page.getByRole("heading", { name: /dashboard/i })).toBeVisible();
  });

  test("failed login shows invalid credentials message", async ({ page }) => {
    await page.locator('input[type="email"]').first().fill("admin@investor-os.com");
    await page.locator('input[type="password"]').first().fill("wrong-password");
    await page.getByRole("button", { name: /sign in/i }).click();

    await expect(page.getByText(/invalid credentials/i)).toBeVisible();
    await expect(page).toHaveURL(/\/login$/);
  });

  test("protected route redirects to login when unauthenticated", async ({ page }) => {
    await page.goto("/");
    await expect(page).toHaveURL(/\/login$/);
  });
});
