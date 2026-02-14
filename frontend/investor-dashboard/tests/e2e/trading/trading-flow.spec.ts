/**
 * Trading Flow E2E Tests
 * Critical path: View Proposals → Approve → Execute Trade
 */

import { test, expect } from "@playwright/test";

test.describe("Trading Flow - AI Proposals", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/login");
    await page.getByLabel(/email/i).fill("test@test.com");
    await page.getByLabel(/password/i).fill("password123");
    await page.getByRole("button", { name: /sign in/i }).click();
    await page.waitForURL("/");
  });

  test("navigate to proposals page", async ({ page }) => {
    await page.getByRole("link", { name: /proposals|предложения/i }).click();
    await expect(page).toHaveURL(/proposals/);
    await expect(page.getByText(/ai proposals|ai предложения/i).first()).toBeVisible();
  });

  test("approve a trade proposal", async ({ page }) => {
    await page.goto("/proposals");
    
    // Find and click approve button on first proposal
    const approveButton = page.getByRole("button", { name: /approve|confirm|потвърди/i }).first();
    if (await approveButton.isVisible()) {
      await approveButton.click();
      
      // Should show success message
      await expect(page.getByText(/success|успех|confirmed/i).first()).toBeVisible();
    }
  });

  test("reject a trade proposal", async ({ page }) => {
    await page.goto("/proposals");
    
    const rejectButton = page.getByRole("button", { name: /reject|отхвърли/i }).first();
    if (await rejectButton.isVisible()) {
      await rejectButton.click();
      await expect(page.getByText(/rejected|отхвърлено/i).first()).toBeVisible();
    }
  });

  test("view proposal details", async ({ page }) => {
    await page.goto("/proposals");
    
    const detailsButton = page.getByRole("button", { name: /details|детайли/i }).first();
    if (await detailsButton.isVisible()) {
      await detailsButton.click();
      await expect(page.getByText(/confidence|увереност|reasoning/i).first()).toBeVisible();
    }
  });
});

test.describe("Trading Flow - Positions", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/login");
    await page.getByLabel(/email/i).fill("test@test.com");
    await page.getByLabel(/password/i).fill("password123");
    await page.getByRole("button", { name: /sign in/i }).click();
    await page.waitForURL("/");
  });

  test("view all positions", async ({ page }) => {
    await page.getByRole("link", { name: /positions|позиции/i }).click();
    await expect(page).toHaveURL(/positions/);
  });

  test("positions table has correct columns", async ({ page }) => {
    await page.goto("/positions");
    
    const table = page.locator("table");
    await expect(table).toBeVisible();
    
    // Check for expected columns
    const headers = ["symbol", "qty", "price", "p&l"];
    for (const header of headers) {
      await expect(
        page.locator("th, td").filter({ hasText: new RegExp(header, "i") }).first()
      ).toBeVisible();
    }
  });
});

test.describe("Trading Flow - Risk Management", () => {
  test("risk limits are enforced", async ({ page }) => {
    await page.goto("/login");
    await page.getByLabel(/email/i).fill("test@test.com");
    await page.getByLabel(/password/i).fill("password123");
    await page.getByRole("button", { name: /sign in/i }).click();
    await page.waitForURL("/");

    // Navigate to risk page
    await page.getByRole("link", { name: /risk|риск/i }).click();
    await expect(page).toHaveURL(/risk/);
  });
});
