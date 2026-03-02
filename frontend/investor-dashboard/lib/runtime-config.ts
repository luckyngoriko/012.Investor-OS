const DEFAULT_BACKEND_ORIGIN = "http://127.0.0.1:8080";
const DEFAULT_API_BASE = "/api";

function trimTrailingSlash(value: string): string {
  return value.replace(/\/+$/, "");
}

export function resolveBackendOrigin(): string {
  const configured =
    process.env.NEXT_PUBLIC_BACKEND_ORIGIN ??
    process.env.NEXT_PUBLIC_API_ORIGIN ??
    DEFAULT_BACKEND_ORIGIN;
  return trimTrailingSlash(configured);
}

export function resolveApiBaseUrl(): string {
  const configuredApiBase = process.env.NEXT_PUBLIC_API_BASE_URL;
  if (configuredApiBase && configuredApiBase.trim().length > 0) {
    return trimTrailingSlash(configuredApiBase.trim());
  }

  return DEFAULT_API_BASE;
}
