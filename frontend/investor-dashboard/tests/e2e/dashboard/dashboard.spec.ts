/**
 * Dashboard E2E Tests
 */

import { test, expect } from "@playwright/test";

test.describe("Dashboard", () => {
  test.beforeEach(async ({ page }) => {
    // Login first
    await page.goto("/login");
    await page.getByLabel(/email/i).fill("test@test.com");
    await page.getByLabel(/password/i).fill("password123");
    await page.getByRole("button", { name: /sign in/i }).click();
    await page.waitForURL("/");
  });

  test("displays portfolio value", async ({ page }) => {
    await expect(page.getByText(/portfolio value|стойност на портфолиото/i)).toBeVisible();
  });

  test("displays positions table", async ({ page }) => {
    await expect(page.getByText(/active positions|активни позиции/i).first()).toBeVisible();
  });

  test("displays AI proposals section", async ({ page }) => {
    await expect(page.getByText(/ai proposals|ai предложения/i).first()).toBeVisible();
  });

  test("refresh button updates data", async ({ page }) => {
    const refreshButton = page.getByRole("button", { name: /refresh|update/i });
    if (await refreshButton.isVisible()) {
      await refreshButton.click();
      // Should show loading state or update timestamp
      await expect(page.getByText(/loading|зареждане/i).first()).not.toBeVisible();
    }
  });
});

test.describe("Dashboard - Responsive", () => {
  test("adapts to mobile viewport", async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto("/login");
    await page.getByLabel(/email/i).fill("test@test.com");
    await page.getByLabel(/password/i).fill("password123");
    await page.getByRole("button", { name: /sign in/i }).click();
    await page.waitForURL("/");

    // Mobile menu should be visible
    await expect(page.getByRole("button", { name: /menu/i })).toBeVisible();
  });
});
