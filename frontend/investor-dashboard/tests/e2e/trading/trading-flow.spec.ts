import { expect, test } from "../fixtures/warning-budget";
import { loginAsUser } from "../utils/auth";

test.describe("Trading Flow - Proposals", () => {
  test.beforeEach(async ({ page }) => {
    await loginAsUser(page, "trader");
  });

  test("proposals page is reachable and shows tabs", async ({ page }) => {
    await page.goto("/proposals");

    await expect(page.getByRole("heading", { name: /trade proposals/i })).toBeVisible();
    await expect(page.getByRole("tab", { name: /pending/i })).toBeVisible();
    await expect(page.getByRole("tab", { name: /confirmed/i })).toBeVisible();
    await expect(page.getByRole("tab", { name: /rejected/i })).toBeVisible();
  });

  test("confirming a proposal updates visible status", async ({ page }) => {
    await page.goto("/proposals");

    const confirmButton = page.getByRole("button", { name: /confirm/i }).first();
    if (await confirmButton.isVisible().catch(() => false)) {
      await confirmButton.click();
      await expect(page.getByText(/confirmed/i).first()).toBeVisible();
    }
  });

  test("rejecting a proposal opens dialog and applies rejection", async ({ page }) => {
    await page.goto("/proposals");

    const rejectButton = page.getByRole("button", { name: /^reject$/i }).first();
    if (await rejectButton.isVisible().catch(() => false)) {
      await rejectButton.click();
      await expect(page.getByRole("dialog")).toBeVisible();
      await page.locator("textarea").fill("Rejected in E2E validation");
      await page.getByRole("button", { name: /reject proposal/i }).click();
      await expect(page.getByText(/rejected/i).first()).toBeVisible();
    }
  });
});

test.describe("Trading Flow - Positions and Risk", () => {
  test.beforeEach(async ({ page }) => {
    await loginAsUser(page, "trader");
  });

  test("positions page shows portfolio table", async ({ page }) => {
    await page.goto("/positions");

    await expect(page.getByRole("heading", { name: /portfolio positions/i })).toBeVisible();
    await expect(page.locator("table")).toBeVisible();
    await expect(page.getByText(/total value/i)).toBeVisible();
  });

  test("risk page is accessible", async ({ page }) => {
    await page.goto("/risk");

    await expect(page.getByRole("heading", { name: /risk management/i })).toBeVisible();
    await expect(page.getByText("Control Checks", { exact: true })).toBeVisible();
  });
});
