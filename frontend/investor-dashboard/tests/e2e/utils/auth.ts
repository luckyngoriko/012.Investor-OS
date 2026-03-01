import { expect, Page } from "@playwright/test";

type DemoRole = "admin" | "trader" | "viewer";

const DEMO_EMAIL_BY_ROLE: Record<DemoRole, string> = {
  admin: "admin@investor-os.com",
  trader: "trader@investor-os.com",
  viewer: "viewer@investor-os.com",
};

const DEMO_LABEL_BY_ROLE: Record<DemoRole, RegExp> = {
  admin: /administrator/i,
  trader: /trader/i,
  viewer: /viewer/i,
};

export async function loginAsDemo(page: Page, role: DemoRole = "trader") {
  await page.addInitScript(() => {
    localStorage.setItem("investor-os-welcome-seen", "true");
    localStorage.setItem("investor-os-tour-seen", "true");
  });

  await page.goto("/login");
  await expect(page.getByRole("heading", { name: /welcome back/i })).toBeVisible();

  const quickLoginButton = page.getByRole("button", { name: DEMO_LABEL_BY_ROLE[role] }).first();
  if (await quickLoginButton.isVisible().catch(() => false)) {
    await quickLoginButton.click();
  }

  const emailInput = page.locator('input[type="email"]').first();
  const passwordInput = page.locator('input[type="password"]').first();

  await emailInput.fill(DEMO_EMAIL_BY_ROLE[role]);
  await passwordInput.fill("demo123");
  await page.getByRole("button", { name: /sign in/i }).click();

  await expect(page).toHaveURL("/", { timeout: 15000 });
}
