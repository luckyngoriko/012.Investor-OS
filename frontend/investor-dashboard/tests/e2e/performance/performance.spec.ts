import { expect, test } from "../fixtures/warning-budget";
import { loginAsUser } from "../utils/auth";

function thresholdForProject(
  projectName: string,
  desktopMs: number,
  tabletMs: number,
  mobileMs: number,
): number {
  if (projectName.includes("mobile")) {
    return mobileMs;
  }
  if (projectName.includes("tablet")) {
    return tabletMs;
  }
  return desktopMs;
}

test.describe("Performance - Baseline", () => {
  test("login page loads under baseline threshold", async ({ page }) => {
    const projectName = test.info().project.name;
    const start = Date.now();
    await page.goto("/login");
    await page.waitForLoadState("domcontentloaded");
    const loadTimeMs = Date.now() - start;
    const thresholdMs = thresholdForProject(projectName, 6000, 9000, 12000);

    expect(loadTimeMs).toBeLessThan(thresholdMs);
  });

  test("post-login dashboard transition stays under threshold", async ({ page }) => {
    const projectName = test.info().project.name;
    const start = Date.now();
    await loginAsUser(page, "trader");
    await page.waitForLoadState("domcontentloaded");
    const transitionMs = Date.now() - start;
    const thresholdMs = thresholdForProject(projectName, 10000, 12000, 15000);

    expect(transitionMs).toBeLessThan(thresholdMs);
  });

  test("page does not throw unhandled runtime errors on login route", async ({ page }) => {
    const pageErrors: string[] = [];
    page.on("pageerror", (error) => pageErrors.push(error.message));

    await page.goto("/login");
    await page.waitForLoadState("domcontentloaded");

    expect(pageErrors).toHaveLength(0);
  });
});

test.describe("Performance - Interaction Latency", () => {
  test.beforeEach(async ({ page }) => {
    await loginAsUser(page, "trader");
  });

  test("navigation to proposals stays responsive", async ({ page }) => {
    const projectName = test.info().project.name;
    const start = Date.now();
    await page.goto("/proposals");
    await page.waitForLoadState("domcontentloaded");
    const navigationMs = Date.now() - start;
    const thresholdMs = thresholdForProject(projectName, 5000, 7000, 9000);

    expect(navigationMs).toBeLessThan(thresholdMs);
    await expect(page.getByRole("heading", { name: /trade proposals/i })).toBeVisible();
  });

  test("command palette opens with keyboard shortcut under threshold", async ({ page }) => {
    const projectName = test.info().project.name;
    test.skip(
      projectName.includes("mobile") || projectName.includes("tablet"),
      "Keyboard shortcut latency is not applicable to touch-only mobile/tablet projects.",
    );

    const search = page.getByPlaceholder(/search commands, pages, or actions/i);
    const start = Date.now();

    for (const shortcut of ["Control+k", "Meta+k"]) {
      await page.keyboard.press(shortcut);
      if (await search.isVisible().catch(() => false)) {
        break;
      }
    }

    await expect(search).toBeVisible();
    const openMs = Date.now() - start;

    expect(openMs).toBeLessThan(2000);
  });
});
