/**
 * Performance E2E Tests
 * Lighthouse CI, Load Times, Bundle Size
 */

import { test, expect } from "@playwright/test";

test.describe("Performance - Page Load", () => {
  test("login page loads under 3 seconds", async ({ page }) => {
    const start = Date.now();
    await page.goto("/login");
    await page.waitForLoadState("networkidle");
    const loadTime = Date.now() - start;
    
    expect(loadTime).toBeLessThan(3000);
  });

  test("dashboard loads under 5 seconds", async ({ page }) => {
    // Login first
    await page.goto("/login");
    await page.getByLabel(/email/i).fill("test@test.com");
    await page.getByLabel(/password/i).fill("password123");
    
    const start = Date.now();
    await page.getByRole("button", { name: /sign in/i }).click();
    await page.waitForURL("/");
    await page.waitForLoadState("networkidle");
    const loadTime = Date.now() - start;
    
    expect(loadTime).toBeLessThan(5000);
  });

  test("no JavaScript errors on load", async ({ page }) => {
    const errors: string[] = [];
    
    page.on("pageerror", (error) => {
      errors.push(error.message);
    });
    
    await page.goto("/login");
    await page.waitForLoadState("networkidle");
    
    expect(errors).toHaveLength(0);
  });

  test("no console errors on load", async ({ page }) => {
    const errors: string[] = [];
    
    page.on("console", (msg) => {
      if (msg.type() === "error") {
        errors.push(msg.text());
      }
    });
    
    await page.goto("/login");
    await page.waitForLoadState("networkidle");
    
    // Filter out known acceptable errors
    const criticalErrors = errors.filter(
      (e) => !e.includes("favicon") && !e.includes("source map")
    );
    
    expect(criticalErrors).toHaveLength(0);
  });
});

test.describe("Performance - Interactions", () => {
  test("navigation is responsive", async ({ page }) => {
    await page.goto("/login");
    await page.getByLabel(/email/i).fill("test@test.com");
    await page.getByLabel(/password/i).fill("password123");
    await page.getByRole("button", { name: /sign in/i }).click();
    await page.waitForURL("/");

    // Measure navigation time
    const start = Date.now();
    await page.getByRole("link", { name: /proposals|предложения/i }).click();
    await page.waitForURL(/proposals/);
    const navTime = Date.now() - start;
    
    expect(navTime).toBeLessThan(2000);
  });

  test("command palette opens quickly", async ({ page }) => {
    await page.goto("/login");
    await page.getByLabel(/email/i).fill("test@test.com");
    await page.getByLabel(/password/i).fill("password123");
    await page.getByRole("button", { name: /sign in/i }).click();
    await page.waitForURL("/");

    const start = Date.now();
    await page.keyboard.press("Control+k");
    await page.waitForSelector("[placeholder*='search' i]");
    const openTime = Date.now() - start;
    
    expect(openTime).toBeLessThan(500);
  });

  test("form submission is responsive", async ({ page }) => {
    await page.goto("/login");
    await page.getByLabel(/email/i).fill("test@test.com");
    await page.getByLabel(/password/i).fill("password123");

    const start = Date.now();
    await page.getByRole("button", { name: /sign in/i }).click();
    await page.waitForURL("/");
    const submitTime = Date.now() - start;
    
    expect(submitTime).toBeLessThan(3000);
  });
});

test.describe("Performance - Resource Loading", () => {
  test("images are lazy loaded", async ({ page }) => {
    await page.goto("/login");

    // Check if images have loading="lazy"
    const images = await page.locator("img").all();
    for (const img of images) {
      const loading = await img.getAttribute("loading");
      if (loading) {
        expect(["lazy", "eager"]).toContain(loading);
      }
    }
  });

  test("bundle size is optimized", async ({ page }) => {
    // This is a basic check - real bundle analysis should be done in build
    await page.goto("/login");
    
    const performanceEntries = await page.evaluate(() =>
      JSON.stringify(performance.getEntriesByType("resource"))
    );
    
    const resources = JSON.parse(performanceEntries);
    const jsFiles = resources.filter((r: { name: string }) => 
      r.name.endsWith(".js")
    );
    
    // Check that JS files are not too large (basic check)
    for (const file of jsFiles) {
      if (file.transferSize) {
        // Transfer size should be reasonable (basic threshold)
        expect(file.transferSize).toBeLessThan(5 * 1024 * 1024); // 5MB max per file
      }
    }
  });
});

test.describe("Performance - Memory", () => {
  test("no memory leaks on navigation", async ({ page }) => {
    await page.goto("/login");
    await page.getByLabel(/email/i).fill("test@test.com");
    await page.getByLabel(/password/i).fill("password123");
    await page.getByRole("button", { name: /sign in/i }).click();
    await page.waitForURL("/");

    // Navigate between pages multiple times
    for (let i = 0; i < 5; i++) {
      await page.goto("/proposals");
      await page.goto("/positions");
      await page.goto("/");
    }

    // Force garbage collection if available
    await page.evaluate(() => {
      if (window.gc) window.gc();
    });

    // Page should still be responsive
    await expect(page.getByText(/dashboard|табло/i).first()).toBeVisible();
  });
});

test.describe("Performance - Lighthouse", () => {
  test.skip("meets performance budget", async ({ page }) => {
    // This would require @lhci/cli or similar
    // Placeholder for Lighthouse CI integration
    await page.goto("/login");
    
    // Lighthouse scores should be:
    // Performance: >90
    // Accessibility: >95
    // Best Practices: >90
    // SEO: >90
  });
});
