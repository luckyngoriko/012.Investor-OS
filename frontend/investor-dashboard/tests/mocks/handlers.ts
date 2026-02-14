/**
 * MSW Handlers - Mock API Endpoints
 */

import { http, HttpResponse, delay } from "msw";
import { factories } from "../fixtures/factories";

// Simulate network delay
const simulateDelay = async (min: number = 100, max: number = 500) => {
  await delay(faker.number.int({ min, max }));
};

import { faker } from "@faker-js/faker";

// Auth handlers
export const authHandlers = [
  http.post("/api/auth/login", async ({ request }) => {
    await simulateDelay();
    const body = (await request.json()) as { email: string; password: string };

    if (body.password === "wrong") {
      return HttpResponse.json(
        { error: "Invalid credentials" },
        { status: 401 }
      );
    }

    return HttpResponse.json({
      user: factories.user.create({ email: body.email }),
      token: "mock-jwt-token",
    });
  }),

  http.post("/api/auth/logout", async () => {
    await simulateDelay();
    return HttpResponse.json({ success: true });
  }),

  http.get("/api/auth/me", async () => {
    await simulateDelay();
    return HttpResponse.json({ user: factories.user.create() });
  }),
];

// Portfolio handlers
export const portfolioHandlers = [
  http.get("/api/portfolio", async () => {
    await simulateDelay(200, 800);
    return HttpResponse.json(factories.portfolio.create());
  }),

  http.get("/api/positions", async () => {
    await simulateDelay(100, 500);
    return HttpResponse.json({
      positions: factories.position.createMany(5),
    });
  }),

  http.post("/api/positions", async ({ request }) => {
    await simulateDelay();
    const body = await request.json() as Record<string, unknown>;
    return HttpResponse.json({
      position: factories.position.create(body as Partial<import("../fixtures/factories").Position>),
    });
  }),

  http.delete("/api/positions/:id", async () => {
    await simulateDelay();
    return HttpResponse.json({ success: true });
  }),
];

// AI Proposal handlers
export const proposalHandlers = [
  http.get("/api/proposals", async () => {
    await simulateDelay(300, 1000);
    return HttpResponse.json({
      proposals: factories.proposal.createMany(3),
    });
  }),

  http.post("/api/proposals/:id/confirm", async () => {
    await simulateDelay(500, 1500);
    return HttpResponse.json({
      trade: factories.position.create(),
      success: true,
    });
  }),

  http.post("/api/proposals/:id/reject", async () => {
    await simulateDelay();
    return HttpResponse.json({ success: true });
  }),

  http.post("/api/proposals/generate", async () => {
    await simulateDelay(1000, 3000);
    return HttpResponse.json({
      proposals: factories.proposal.createMany(2),
    });
  }),
];

// Market data handlers
export const marketHandlers = [
  http.get("/api/market/data/:symbol", async ({ params }) => {
    await simulateDelay(50, 200);
    return HttpResponse.json(
      factories.marketData.create({ symbol: params.symbol as string })
    );
  }),

  http.get("/api/market/chart/:symbol", async () => {
    await simulateDelay(200, 600);
    return HttpResponse.json({
      data: factories.chartData.createTimeSeries(30),
    });
  }),
];

// Settings handlers
export const settingsHandlers = [
  http.get("/api/settings", async () => {
    await simulateDelay();
    return HttpResponse.json(factories.settings.create());
  }),

  http.patch("/api/settings", async ({ request }) => {
    await simulateDelay();
    const body = await request.json() as Record<string, unknown>;
    return HttpResponse.json({
      settings: factories.settings.create(body as Partial<import("../fixtures/factories").UserSettings>),
    });
  }),
];

// Notification handlers
export const notificationHandlers = [
  http.get("/api/notifications", async () => {
    await simulateDelay();
    return HttpResponse.json({
      notifications: factories.notification.createMany(10),
    });
  }),

  http.patch("/api/notifications/:id/read", async () => {
    await simulateDelay();
    return HttpResponse.json({ success: true });
  }),

  http.post("/api/notifications/mark-all-read", async () => {
    await simulateDelay();
    return HttpResponse.json({ success: true });
  }),
];

// Error simulation handlers
export const errorHandlers = [
  http.get("/api/error/500", () => {
    return HttpResponse.json(
      { error: "Internal Server Error" },
      { status: 500 }
    );
  }),

  http.get("/api/error/timeout", async () => {
    await delay(30000);
    return HttpResponse.json({ error: "Timeout" }, { status: 408 });
  }),

  http.get("/api/error/network", () => {
    return new Response(null, { status: 0 });
  }),
];

// WebSocket mock (for testing real-time updates)
export const wsHandlers = [
  http.get("/ws/market", () => {
    return new Response("WebSocket not supported in tests");
  }),
];

// Combine all handlers
export const handlers = [
  ...authHandlers,
  ...portfolioHandlers,
  ...proposalHandlers,
  ...marketHandlers,
  ...settingsHandlers,
  ...notificationHandlers,
  ...errorHandlers,
  ...wsHandlers,
];

// Scenario-specific handler sets
export const scenarios = {
  // Empty state scenario
  empty: [
    http.get("/api/portfolio", () => HttpResponse.json(null)),
    http.get("/api/positions", () => HttpResponse.json({ positions: [] })),
    http.get("/api/proposals", () => HttpResponse.json({ proposals: [] })),
  ],

  // Loading scenario (slow responses)
  slow: [
    http.get("/api/portfolio", async () => {
      await delay(10000);
      return HttpResponse.json(factories.portfolio.create());
    }),
  ],

  // Error scenario
  error: [
    http.get("/api/portfolio", () =>
      HttpResponse.json({ error: "Failed to load" }, { status: 500 })
    ),
  ],

  // High volume scenario (many positions)
  highVolume: [
    http.get("/api/positions", () =>
      HttpResponse.json({
        positions: factories.position.createMany(100),
      })
    ),
  ],
};
