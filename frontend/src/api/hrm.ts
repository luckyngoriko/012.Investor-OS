/**
 * HRM API Client
 * Sprint 44: Frontend Dashboard
 */

import axios from "axios";
import {
  HRMInferenceRequest,
  HRMInferenceResponse,
  HRMHealthResponse,
  ApiResponse,
  HistoricalPrediction,
} from "../types/hrm";

const API_BASE_URL = process.env.REACT_APP_API_URL || "http://localhost:8080";

const api = axios.create({
  baseURL: API_BASE_URL,
  headers: {
    "Content-Type": "application/json",
  },
  timeout: 5000,
});

/**
 * Run HRM inference
 */
export async function inferHRM(
  request: HRMInferenceRequest,
): Promise<HRMInferenceResponse> {
  const response = await api.post<ApiResponse<HRMInferenceResponse>>(
    "/api/v1/hrm/infer",
    request,
  );

  if (!response.data.success || !response.data.data) {
    throw new Error(response.data.error || "Inference failed");
  }

  return response.data.data;
}

/**
 * Run batch HRM inference
 */
export async function inferHRMBatch(
  requests: HRMInferenceRequest[],
): Promise<HRMInferenceResponse[]> {
  const response = await api.post<
    ApiResponse<{ results: HRMInferenceResponse[] }>
  >("/api/v1/hrm/batch", { signals: requests });

  if (!response.data.success || !response.data.data) {
    throw new Error(response.data.error || "Batch inference failed");
  }

  return response.data.data.results;
}

/**
 * Get HRM health status
 */
export async function getHRMHealth(): Promise<HRMHealthResponse> {
  const response =
    await api.get<ApiResponse<HRMHealthResponse>>("/api/v1/hrm/health");

  if (!response.data.success || !response.data.data) {
    throw new Error(response.data.error || "Health check failed");
  }

  return response.data.data;
}

/**
 * Fetch HRM prediction history from backend
 */
export async function fetchHRMHistory(
  limit: number = 50,
): Promise<HistoricalPrediction[]> {
  try {
    const response = await api.get<ApiResponse<HistoricalPrediction[]>>(
      "/api/v1/hrm/history",
      { params: { limit } },
    );

    if (!response.data.success || !response.data.data) {
      return [];
    }

    return response.data.data;
  } catch {
    // Graceful fallback: return empty array when endpoint unavailable
    console.warn("HRM history endpoint unavailable, returning empty history");
    return [];
  }
}

export { API_BASE_URL };
