const API_BASE = "/api";
const ACCESS_TOKEN_KEY = "auth.accessToken";

class DomainApiError extends Error {
  status: number;

  constructor(message: string, status: number) {
    super(message);
    this.name = "DomainApiError";
    this.status = status;
  }
}

function resolveAccessToken(): string | null {
  if (typeof window === "undefined") {
    return null;
  }
  return window.localStorage.getItem(ACCESS_TOKEN_KEY);
}

/** Retry configuration for API requests. */
const MAX_RETRIES = 3;
const RETRY_BASE_MS = 1000;
/** Status codes that should NOT be retried (client errors). */
const NON_RETRYABLE = new Set([400, 401, 403, 404, 409, 415, 422]);

async function requestJson<T>(path: string, init?: RequestInit): Promise<T> {
  let lastError: DomainApiError | null = null;

  for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
    if (attempt > 0) {
      const delay = RETRY_BASE_MS * Math.pow(2, attempt - 1);
      await new Promise((r) => setTimeout(r, delay));
    }

    const token = resolveAccessToken();
    const headers = new Headers(init?.headers);

    if (token) {
      headers.set("Authorization", `Bearer ${token}`);
    }
    if (init?.body && !headers.has("Content-Type")) {
      headers.set("Content-Type", "application/json");
    }

    try {
      const response = await fetch(`${API_BASE}${path}`, {
        ...init,
        headers,
        cache: "no-store",
      });

      const raw = await response.text();
      const data = raw ? (JSON.parse(raw) as unknown) : null;

      if (!response.ok) {
        let message = `Request failed with status ${response.status}`;
        if (data && typeof data === "object") {
          const errObj = data as Record<string, unknown>;
          const maybeError = errObj.error;
          if (typeof maybeError === "string" && maybeError.length > 0) {
            message = maybeError;
          } else if (
            typeof maybeError === "object" &&
            maybeError !== null &&
            "message" in maybeError
          ) {
            message = String((maybeError as Record<string, unknown>).message);
          }
        }
        const err = new DomainApiError(message, response.status);
        if (NON_RETRYABLE.has(response.status)) {
          throw err;
        }
        lastError = err;
        continue;
      }

      return data as T;
    } catch (e) {
      if (e instanceof DomainApiError) {
        throw e;
      }
      lastError = new DomainApiError(
        e instanceof Error ? e.message : "Network error",
        0,
      );
    }
  }

  throw lastError ?? new DomainApiError("Request failed after retries", 0);
}

export interface SecurityFeature {
  name: string;
  description: string;
  rotation_interval?: string;
  methods?: string[];
  event_types?: number;
  levels?: string[];
}

export interface SecurityStatusResponse {
  module: string;
  status: string;
  features: SecurityFeature[];
  stats?: {
    unit_tests: number;
    integration_tests: number;
    total_tests: number;
  };
}

export interface ClearanceLevel {
  name: string;
  value: number;
  description: string;
  "2fa_required": boolean;
}

export interface ClearanceLevelsResponse {
  hierarchy: string;
  levels: ClearanceLevel[];
  example: string;
}

export interface GenerateApiKeyResponse {
  key_id: string;
  api_key: string;
  user_id: string;
  clearance: string;
  expires_in_days: number;
  algorithm: string;
  note: string;
  usage: string;
}

export interface PortfolioMethod {
  name: string;
  description: string;
}

export interface PortfolioOptimizeResponse {
  module: string;
  methods: PortfolioMethod[];
  objectives: string[];
  example_optimization: {
    input: {
      assets: string[];
      objective: string;
    };
    output: {
      weights: Record<string, number>;
      expected_return: string;
      expected_risk: string;
      sharpe_ratio: number;
      diversification_ratio: number;
    };
  };
  tests?: number;
}

export interface EfficientFrontierPoint {
  risk: string;
  return: string;
  portfolio: string;
  allocation: string;
}

export interface EfficientFrontierResponse {
  description: string;
  frontier_points: EfficientFrontierPoint[];
  tangency_portfolio?: {
    description: string;
    risk: string;
    return: string;
    sharpe_ratio: number;
  };
  calculations_performed: string;
}

export interface TaxFeature {
  name: string;
  description: string;
  min_loss_threshold?: string;
  max_harvests_per_month?: number;
  replacement_securities?: string;
  formats?: string[];
}

export interface TaxStatusResponse {
  module: string;
  jurisdiction: string;
  features: TaxFeature[];
  tests?: number;
}

export interface TaxCalculationResponse {
  tax_year: number;
  calculations: {
    short_term_gains: number;
    long_term_gains: number;
    short_term_rate: string;
    long_term_rate: string;
    estimated_tax: {
      short_term: number;
      long_term: number;
      total: number;
    };
  };
  optimization_opportunities: Array<{
    action: string;
    tax_savings?: string;
    replacement?: string;
    reason?: string;
    potential_savings?: string;
  }>;
  harvesting_status: string;
}

export async function fetchSecurityStatus(): Promise<SecurityStatusResponse> {
  return requestJson<SecurityStatusResponse>("/security/status", {
    method: "GET",
  });
}

export async function fetchClearanceLevels(): Promise<ClearanceLevelsResponse> {
  return requestJson<ClearanceLevelsResponse>("/security/clearance-levels", {
    method: "GET",
  });
}

export async function generateSecurityApiKey(): Promise<GenerateApiKeyResponse> {
  return requestJson<GenerateApiKeyResponse>("/security/generate-key", {
    method: "POST",
    body: JSON.stringify({}),
  });
}

export async function fetchPortfolioOptimization(): Promise<PortfolioOptimizeResponse> {
  return requestJson<PortfolioOptimizeResponse>("/portfolio/optimize", {
    method: "POST",
    body: JSON.stringify({}),
  });
}

export async function fetchEfficientFrontier(): Promise<EfficientFrontierResponse> {
  return requestJson<EfficientFrontierResponse>(
    "/portfolio/efficient-frontier",
    {
      method: "GET",
    },
  );
}

export async function fetchTaxStatus(): Promise<TaxStatusResponse> {
  return requestJson<TaxStatusResponse>("/tax/status", { method: "GET" });
}

export async function fetchTaxCalculation(): Promise<TaxCalculationResponse> {
  return requestJson<TaxCalculationResponse>("/tax/calculate", {
    method: "POST",
    body: JSON.stringify({}),
  });
}

export { DomainApiError };
