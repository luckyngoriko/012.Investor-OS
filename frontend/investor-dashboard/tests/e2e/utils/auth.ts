import { expect, Page } from "@playwright/test";

type UserRole = "admin" | "trader" | "viewer";

const USER_EMAIL_BY_ROLE: Record<UserRole, string> = {
  admin: "admin@investor-os.com",
  trader: "trader@investor-os.com",
  viewer: "viewer@investor-os.com",
};

const USER_PASSWORD_BY_ROLE: Record<UserRole, string> = {
  admin: process.env.E2E_AUTH_PASSWORD_ADMIN ?? "Admin#2026!",
  trader: process.env.E2E_AUTH_PASSWORD_TRADER ?? "Trader#2026!",
  viewer: process.env.E2E_AUTH_PASSWORD_VIEWER ?? "Viewer#2026!",
};

const USER_NAME_BY_ROLE: Record<UserRole, string> = {
  admin: "Admin User",
  trader: "Trader User",
  viewer: "Viewer User",
};

const USER_PERMISSIONS_BY_ROLE: Record<UserRole, string[]> = {
  admin: ["*"],
  trader: [
    "dashboard.read",
    "portfolio.read",
    "portfolio.trade",
    "positions.read",
    "proposals.read",
    "proposals.execute",
    "risk.read",
    "backtest.read",
    "backtest.run",
    "journal.read",
    "journal.write",
    "settings.read",
    "settings.update",
  ],
  viewer: ["dashboard.read", "portfolio.read", "positions.read", "proposals.read", "risk.read", "journal.read"],
};

function roleFromEmail(email: string): UserRole | null {
  return (
    (Object.keys(USER_EMAIL_BY_ROLE) as UserRole[]).find(
      (role) => USER_EMAIL_BY_ROLE[role] === email.toLowerCase(),
    ) ?? null
  );
}

function buildSession(role: UserRole) {
  const now = new Date();
  const expiresAt = new Date(now.getTime() + 15 * 60 * 1000).toISOString();
  const refreshExpiresAt = new Date(now.getTime() + 7 * 24 * 60 * 60 * 1000).toISOString();

  return {
    user: {
      id: role === "admin" ? "1" : role === "trader" ? "2" : "3",
      email: USER_EMAIL_BY_ROLE[role],
      name: USER_NAME_BY_ROLE[role],
      role,
      permissions: USER_PERMISSIONS_BY_ROLE[role],
    },
    access_token: `atk_${role}_e2e`,
    refresh_token: `rtk_${role}_e2e`,
    expires_at: expiresAt,
    refresh_expires_at: refreshExpiresAt,
  };
}

export async function loginAsUser(page: Page, role: UserRole = "trader") {
  await page.route("**/api/auth/login", async (route) => {
    const body = (route.request().postDataJSON() as { email?: string; password?: string }) ?? {};
    const email = (body.email ?? "").toLowerCase();
    const requestedRole = roleFromEmail(email);

    if (!requestedRole || body.password !== USER_PASSWORD_BY_ROLE[requestedRole]) {
      await route.fulfill({
        status: 401,
        contentType: "application/json",
        body: JSON.stringify({
          success: false,
          error: "Invalid credentials",
        }),
      });
      return;
    }

    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        success: true,
        data: buildSession(requestedRole),
      }),
    });
  });

  await page.route("**/api/auth/refresh", async (route) => {
    const body = (route.request().postDataJSON() as { refresh_token?: string }) ?? {};
    const token = body.refresh_token ?? "";
    const refreshedRole = (Object.keys(USER_EMAIL_BY_ROLE) as UserRole[]).find((candidateRole) =>
      token.includes(candidateRole),
    );

    if (!refreshedRole) {
      await route.fulfill({
        status: 401,
        contentType: "application/json",
        body: JSON.stringify({
          success: false,
          error: "Invalid refresh token",
        }),
      });
      return;
    }

    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        success: true,
        data: buildSession(refreshedRole),
      }),
    });
  });

  await page.route("**/api/auth/me", async (route) => {
    const authHeader = route.request().headers()["authorization"] ?? "";
    const meRole = (Object.keys(USER_EMAIL_BY_ROLE) as UserRole[]).find((candidateRole) =>
      authHeader.includes(candidateRole),
    );

    if (!meRole) {
      await route.fulfill({
        status: 401,
        contentType: "application/json",
        body: JSON.stringify({
          success: false,
          error: "Invalid or expired session",
        }),
      });
      return;
    }

    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        success: true,
        data: {
          user: buildSession(meRole).user,
        },
      }),
    });
  });

  await page.route("**/api/auth/logout", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        success: true,
        data: { logged_out: true },
      }),
    });
  });

  await page.addInitScript(() => {
    localStorage.setItem("investor-os-welcome-seen", "true");
    localStorage.setItem("investor-os-tour-seen", "true");
  });

  await page.goto("/login");
  await expect(page.getByRole("heading", { name: /welcome back/i })).toBeVisible();

  const emailInput = page.locator('input[type="email"]').first();
  const passwordInput = page.locator('input[type="password"]').first();

  await emailInput.fill(USER_EMAIL_BY_ROLE[role]);
  await passwordInput.fill(USER_PASSWORD_BY_ROLE[role]);
  await page.getByRole("button", { name: /sign in/i }).click();

  await expect(page).toHaveURL("/", { timeout: 15000 });
}
