import { resolveApiBaseUrl } from "@/lib/runtime-config";

export type UserRole = "admin" | "trader" | "viewer";

export interface AuthUser {
  id: string;
  email: string;
  name: string;
  role: UserRole;
  avatar?: string;
  permissions: string[];
}

export interface AuthSession {
  user: AuthUser;
  access_token: string;
  refresh_token: string;
  expires_at: string;
  refresh_expires_at: string;
}

interface ApiEnvelope<T> {
  success: boolean;
  data?: T;
  error?: string;
}

const AUTH_BASE = resolveApiBaseUrl();

async function decodeEnvelope<T>(response: Response): Promise<T> {
  const body = (await response.json().catch(() => null)) as ApiEnvelope<T> | null;

  if (!response.ok || !body?.success || body.data === undefined) {
    const message = body?.error || `Request failed with status ${response.status}`;
    throw new Error(message);
  }

  return body.data;
}

export async function loginWithPassword(
  email: string,
  password: string,
): Promise<AuthSession> {
  const response = await fetch(`${AUTH_BASE}/auth/login`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ email, password }),
  });

  return decodeEnvelope<AuthSession>(response);
}

export async function refreshAuthSession(refreshToken: string): Promise<AuthSession> {
  const response = await fetch(`${AUTH_BASE}/auth/refresh`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ refresh_token: refreshToken }),
  });

  return decodeEnvelope<AuthSession>(response);
}

export async function fetchCurrentUser(accessToken: string): Promise<AuthUser> {
  const response = await fetch(`${AUTH_BASE}/auth/me`, {
    method: "GET",
    headers: {
      Authorization: `Bearer ${accessToken}`,
    },
  });

  const payload = await decodeEnvelope<{ user: AuthUser }>(response);
  return payload.user;
}

export async function logoutCurrentSession(
  accessToken?: string | null,
  refreshToken?: string | null,
): Promise<void> {
  if (!accessToken && !refreshToken) {
    return;
  }

  await fetch(`${AUTH_BASE}/auth/logout`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...(accessToken ? { Authorization: `Bearer ${accessToken}` } : {}),
    },
    body: JSON.stringify({
      refresh_token: refreshToken ?? null,
    }),
  }).catch(() => {
    // Local logout should continue even if backend call fails.
  });
}

