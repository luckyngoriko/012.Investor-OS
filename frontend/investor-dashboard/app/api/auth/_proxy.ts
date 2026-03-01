import { NextRequest, NextResponse } from "next/server";

function trimTrailingSlash(value: string): string {
  return value.replace(/\/+$/, "");
}

function normalizeBackendApiBase(value: string): string {
  const trimmed = trimTrailingSlash(value);
  return trimmed.endsWith("/api") ? trimmed : `${trimmed}/api`;
}

const BACKEND_API_BASE = normalizeBackendApiBase(
  process.env.BACKEND_API_BASE_URL ??
    process.env.BACKEND_BASE_URL ??
    "http://127.0.0.1:8080",
);

export async function proxyAuthToBackend(
  request: NextRequest,
  endpoint: "login" | "refresh" | "me" | "logout",
) {
  const targetUrl = `${BACKEND_API_BASE}/auth/${endpoint}`;
  const bodyText = request.method === "GET" ? "" : await request.text();
  const authorization = request.headers.get("authorization");

  try {
    const response = await fetch(targetUrl, {
      method: request.method,
      headers: {
        ...(bodyText ? { "Content-Type": "application/json" } : {}),
        ...(authorization ? { Authorization: authorization } : {}),
      },
      body: bodyText || undefined,
      cache: "no-store",
    });

    const rawBody = await response.text();
    const contentType = response.headers.get("content-type") ?? "application/json";
    return new NextResponse(rawBody, {
      status: response.status,
      headers: {
        "Content-Type": contentType,
      },
    });
  } catch {
    return NextResponse.json(
      {
        success: false,
        error: "Auth backend unavailable",
      },
      { status: 503 },
    );
  }
}

