/**
 * Authentication E2E Tests
 * Critical path: Login → Dashboard → Logout
 */

import { test, expect } from "@playwright/test";
import { factories } from "../../fixtures/factories";

test.describe("Authentication", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/login");
  });

  test("successful login with valid credentials", async ({ page }) => {
    const user = factories.user.create();

    await page.getByLabel(/email/i).fill(user.email);
    await page.getByLabel(/password/i).fill("password123");
    await page.getByRole("button", { name: /sign in|login|вход/i }).click();

    await expect(page).toHaveURL("/");
    await expect(page.getByText(/dashboard|табло/i).first()).toBeVisible();
  });

  test("failed login shows error message", async ({ page }) => {
    await page.getByLabel(/email/i).fill("wrong@email.com");
    await page.getByLabel(/password/i).fill("wrongpassword");
    await page.getByRole("button", { name: /sign in|login/i }).click();

    await expect(page.getByText(/invalid|incorrect|error|грешка/i).first()).toBeVisible();
    await expect(page).toHaveURL(/login/);
  });

  test("protected routes redirect to login", async ({ page }) => {
    await page.goto("/");
    await expect(page).toHaveURL(/login/);
  });
});
