import { APIRequestContext, expect, test } from "@playwright/test";
import { loginAsDemo } from "../utils/auth";

const BACKEND_BASE_URL = process.env.BACKEND_BASE_URL ?? "http://127.0.0.1:8080";

async function backendRuntimeAvailable(request: APIRequestContext) {
  try {
    const response = await request.get(`${BACKEND_BASE_URL}/api/health`, { timeout: 3000 });
    return response.ok();
  } catch {
    return false;
  }
}

test.describe("Runtime Contract", () => {
  test("monitoring page consumes healthy runtime endpoints", async ({ page, request }) => {
    const backendAvailable = await backendRuntimeAvailable(request);
    test.skip(
      !backendAvailable,
      `Backend runtime contract unavailable at ${BACKEND_BASE_URL} (set BACKEND_BASE_URL to enable this assertion)`,
    );

    await loginAsDemo(page, "trader");

    await page.goto("/monitoring");
    await expect(page.getByRole("heading", { name: /performance monitoring/i })).toBeVisible();

    const healthResponse = await page.waitForResponse(
      (response) => response.url().includes("/api/health") && response.status() === 200,
      { timeout: 20000 },
    );
    const readyResponse = await page.waitForResponse(
      (response) => response.url().includes("/api/runtime/config") && response.status() === 200,
      { timeout: 20000 },
    );
    const hrmStatusResponse = await page.waitForResponse(
      (response) => response.url().includes("/api/hrm/status") && response.status() === 200,
      { timeout: 20000 },
    );
    const metricsResponse = await page.waitForResponse(
      (response) => response.url().endsWith("/metrics") && response.status() === 200,
      { timeout: 20000 },
    );

    const healthJson = await healthResponse.json();
    expect(healthJson.success).toBe(true);
    expect(healthJson.data?.status).toMatch(/healthy|degraded/);
    expect(healthJson.data?.runtime_contract?.api_base_url).toBeTruthy();
    expect(healthJson.data?.runtime_contract?.ws_hrm_url).toBeTruthy();

    const runtimeJson = await readyResponse.json();
    expect(runtimeJson.success).toBe(true);
    expect(runtimeJson.data?.api_base_url).toBeTruthy();
    expect(runtimeJson.data?.ws_hrm_url).toBeTruthy();
    expect(Array.isArray(runtimeJson.data?.allowed_origins)).toBe(true);

    const hrmStatusJson = await hrmStatusResponse.json();
    expect(hrmStatusJson.success).toBe(true);

    const metricsText = await metricsResponse.text();
    expect(metricsText).toContain("# HELP");
  });
});
