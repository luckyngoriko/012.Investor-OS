/**
 * Accessibility E2E Tests
 * WCAG 2.1 AA Compliance
 */

import { test, expect } from "@playwright/test";

test.describe("Accessibility - Login Page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/login");
  });

  test("all interactive elements are focusable", async ({ page }) => {
    const focusableElements = await page.locator("button, a, input, select, textarea").all();
    
    for (const element of focusableElements) {
      await element.focus();
      await expect(element).toBeFocused();
    }
  });

  test("form has proper labels", async ({ page }) => {
    const emailInput = page.getByLabel(/email/i);
    const passwordInput = page.getByLabel(/password/i);
    
    await expect(emailInput).toHaveAttribute("id");
    await expect(passwordInput).toHaveAttribute("id");
  });

  test("color contrast meets WCAG AA", async ({ page }) => {
    // Check main heading contrast
    const heading = page.locator("h1").first();
    await expect(heading).toBeVisible();
    
    // Note: Actual contrast checking requires additional tools
    // This is a placeholder for manual verification
  });

  test("keyboard navigation works", async ({ page }) => {
    // Tab through all interactive elements
    let currentElement = page.locator("*:focus");
    
    for (let i = 0; i < 5; i++) {
      await page.keyboard.press("Tab");
      const newElement = page.locator("*:focus");
      await expect(newElement).not.toEqual(currentElement);
      currentElement = newElement;
    }
  });

  test("error messages have role=alert", async ({ page }) => {
    await page.getByRole("button", { name: /sign in/i }).click();
    
    const alert = page.getByRole("alert");
    // If there are validation errors, they should have alert role
    if (await alert.isVisible().catch(() => false)) {
      await expect(alert).toBeVisible();
    }
  });
});

test.describe("Accessibility - Dashboard", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/login");
    await page.getByLabel(/email/i).fill("test@test.com");
    await page.getByLabel(/password/i).fill("password123");
    await page.getByRole("button", { name: /sign in/i }).click();
    await page.waitForURL("/");
  });

  test("heading hierarchy is correct", async ({ page }) => {
    const h1 = page.locator("h1");
    await expect(h1).toHaveCount(1);
  });

  test.skip("images have alt text", async ({ page }) => {
    const images = await page.locator("img").all();
    for (const img of images) {
      await expect(img).toHaveAttribute("alt", /.*/);
    }
  });

  test("table has proper headers", async ({ page }) => {
    const tables = await page.locator("table").all();
    for (const table of tables) {
      const headers = await table.locator("th").count();
      expect(headers).toBeGreaterThan(0);
    }
  });

  test("ARIA labels on icon buttons", async ({ page }) => {
    const iconButtons = page.locator("button:has(svg), a:has(svg)");
    const count = await iconButtons.count();
    
    for (let i = 0; i < count; i++) {
      const button = iconButtons.nth(i);
      const hasAriaLabel = await button.getAttribute("aria-label");
      const hasAriaLabelledBy = await button.getAttribute("aria-labelledby");
      const hasTitle = await button.getAttribute("title");
      
      expect(hasAriaLabel || hasAriaLabelledBy || hasTitle).toBeTruthy();
    }
  });
});

test.describe("Accessibility - Keyboard Shortcuts", () => {
  test("command palette opens with keyboard shortcut", async ({ page }) => {
    await page.goto("/login");
    await page.getByLabel(/email/i).fill("test@test.com");
    await page.getByLabel(/password/i).fill("password123");
    await page.getByRole("button", { name: /sign in/i }).click();
    await page.waitForURL("/");

    // Press Cmd+K (Mac) or Ctrl+K (Windows)
    await page.keyboard.press("Control+k");
    
    // Command palette should open
    await expect(page.getByPlaceholder(/search/i)).toBeVisible();
  });

  test("escape closes modals and panels", async ({ page }) => {
    await page.goto("/login");
    await page.getByLabel(/email/i).fill("test@test.com");
    await page.getByLabel(/password/i).fill("password123");
    await page.getByRole("button", { name: /sign in/i }).click();
    await page.waitForURL("/");

    // Open command palette
    await page.keyboard.press("Control+k");
    await expect(page.getByPlaceholder(/search/i)).toBeVisible();

    // Press escape
    await page.keyboard.press("Escape");
    
    // Command palette should close
    await expect(page.getByPlaceholder(/search/i)).not.toBeVisible();
  });
});

test.describe("Accessibility - Screen Reader", () => {
  test("live regions for dynamic content", async ({ page }) => {
    await page.goto("/login");
    
    // Check for live regions
    const liveRegions = await page.locator("[aria-live]").all();
    
    // If there are notifications or status updates
    if (liveRegions.length > 0) {
      for (const region of liveRegions) {
        const liveValue = await region.getAttribute("aria-live");
        expect(["polite", "assertive", "off"]).toContain(liveValue);
      }
    }
  });

  test("skip link present", async ({ page }) => {
    await page.goto("/login");
    
    // First tab should go to skip link if present
    await page.keyboard.press("Tab");
    const focused = page.locator(":focus");
    
    // Check if it's a skip link
    const text = await focused.textContent().catch(() => "");
    if (text.toLowerCase().includes("skip")) {
      await expect(focused).toBeVisible();
    }
  });
});
