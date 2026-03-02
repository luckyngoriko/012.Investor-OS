import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  fetchCurrentUser,
  loginWithPassword,
  logoutCurrentSession,
  refreshAuthSession,
} from "@/lib/auth-api";

describe("auth-api", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("logs in through backend auth endpoint", async () => {
    vi.mocked(global.fetch).mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          success: true,
          data: {
            user: {
              id: "2",
              email: "trader@investor-os.com",
              name: "Trader User",
              role: "trader",
              permissions: ["dashboard.read"],
            },
            access_token: "atk_test",
            refresh_token: "rtk_test",
            expires_at: "2026-01-01T00:00:00Z",
            refresh_expires_at: "2026-01-08T00:00:00Z",
          },
        }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    const session = await loginWithPassword("trader@investor-os.com", "Trader#2026!");

    expect(session.access_token).toBe("atk_test");
    expect(session.refresh_token).toBe("rtk_test");
    expect(global.fetch).toHaveBeenCalledWith(
      "/api/auth/login",
      expect.objectContaining({
        method: "POST",
      }),
    );
  });

  it("throws descriptive error for invalid credentials", async () => {
    vi.mocked(global.fetch).mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          success: false,
          error: "Invalid credentials",
        }),
        { status: 401, headers: { "Content-Type": "application/json" } },
      ),
    );

    await expect(loginWithPassword("trader@investor-os.com", "wrong")).rejects.toThrow(
      "Invalid credentials",
    );
  });

  it("refreshes and retrieves current user with bearer token", async () => {
    vi.mocked(global.fetch)
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            success: true,
            data: {
              user: {
                id: "2",
                email: "trader@investor-os.com",
                name: "Trader User",
                role: "trader",
                permissions: ["dashboard.read"],
              },
              access_token: "atk_refreshed",
              refresh_token: "rtk_refreshed",
              expires_at: "2026-01-01T00:00:00Z",
              refresh_expires_at: "2026-01-08T00:00:00Z",
            },
          }),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            success: true,
            data: {
              user: {
                id: "2",
                email: "trader@investor-os.com",
                name: "Trader User",
                role: "trader",
                permissions: ["dashboard.read"],
              },
            },
          }),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ),
      );

    const refreshed = await refreshAuthSession("rtk_old");
    const user = await fetchCurrentUser(refreshed.access_token);

    expect(refreshed.access_token).toBe("atk_refreshed");
    expect(user.email).toBe("trader@investor-os.com");
    expect(global.fetch).toHaveBeenNthCalledWith(
      2,
      "/api/auth/me",
      expect.objectContaining({
        headers: expect.objectContaining({
          Authorization: "Bearer atk_refreshed",
        }),
      }),
    );
  });

  it("skips logout API call when no tokens are present", async () => {
    await logoutCurrentSession(undefined, undefined);
    expect(global.fetch).not.toHaveBeenCalled();
  });
});
