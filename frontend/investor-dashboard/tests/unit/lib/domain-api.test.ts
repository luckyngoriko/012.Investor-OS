import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  fetchPortfolioOptimization,
  fetchTaxStatus,
  generateSecurityApiKey,
} from "@/lib/domain-api";

describe("domain-api", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("sends bearer token when requesting tax status", async () => {
    vi.mocked(window.localStorage.getItem).mockReturnValueOnce("atk_live_token");
    vi.mocked(global.fetch).mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          module: "Tax & Compliance Engine (Sprint 30)",
          jurisdiction: "US",
          features: [],
        }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    await fetchTaxStatus();

    expect(global.fetch).toHaveBeenCalledWith(
      "/api/tax/status",
      expect.objectContaining({
        method: "GET",
        headers: expect.any(Headers),
      }),
    );

    const fetchCall = vi.mocked(global.fetch).mock.calls[0];
    const headers = fetchCall[1]?.headers as Headers;
    expect(headers.get("Authorization")).toBe("Bearer atk_live_token");
  });

  it("uses POST for portfolio optimization endpoint", async () => {
    vi.mocked(window.localStorage.getItem).mockReturnValueOnce("atk_live_token");
    vi.mocked(global.fetch).mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          module: "Portfolio Optimization (Sprint 32)",
          methods: [],
          objectives: [],
          example_optimization: {
            input: { assets: [], objective: "MaximizeSharpe" },
            output: {
              weights: {},
              expected_return: "0%",
              expected_risk: "0%",
              sharpe_ratio: 0,
              diversification_ratio: 0,
            },
          },
        }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    await fetchPortfolioOptimization();

    expect(global.fetch).toHaveBeenCalledWith(
      "/api/portfolio/optimize",
      expect.objectContaining({
        method: "POST",
      }),
    );
  });

  it("surfaces backend error for api key generation", async () => {
    vi.mocked(window.localStorage.getItem).mockReturnValueOnce("atk_live_token");
    vi.mocked(global.fetch).mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          error: "Security backend unavailable",
        }),
        { status: 503, headers: { "Content-Type": "application/json" } },
      ),
    );

    await expect(generateSecurityApiKey()).rejects.toThrow("Security backend unavailable");
  });
});
