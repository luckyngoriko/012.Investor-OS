/**
 * HRM API Client
 * Sprint 44: Frontend Dashboard
 */

import axios from 'axios';
import {
  HRMInferenceRequest,
  HRMInferenceResponse,
  HRMHealthResponse,
  ApiResponse,
  HistoricalPrediction,
} from '../types/hrm';

const API_BASE_URL = process.env.REACT_APP_API_URL || 'http://localhost:8080';

const api = axios.create({
  baseURL: API_BASE_URL,
  headers: {
    'Content-Type': 'application/json',
  },
  timeout: 5000,
});

/**
 * Run HRM inference
 */
export async function inferHRM(
  request: HRMInferenceRequest
): Promise<HRMInferenceResponse> {
  const response = await api.post<ApiResponse<HRMInferenceResponse>>(
    '/api/v1/hrm/infer',
    request
  );
  
  if (!response.data.success || !response.data.data) {
    throw new Error(response.data.error || 'Inference failed');
  }
  
  return response.data.data;
}

/**
 * Run batch HRM inference
 */
export async function inferHRMBatch(
  requests: HRMInferenceRequest[]
): Promise<HRMInferenceResponse[]> {
  const response = await api.post<ApiResponse<{ results: HRMInferenceResponse[] }>>(
    '/api/v1/hrm/batch',
    { signals: requests }
  );
  
  if (!response.data.success || !response.data.data) {
    throw new Error(response.data.error || 'Batch inference failed');
  }
  
  return response.data.data.results;
}

/**
 * Get HRM health status
 */
export async function getHRMHealth(): Promise<HRMHealthResponse> {
  const response = await api.get<ApiResponse<HRMHealthResponse>>(
    '/api/v1/hrm/health'
  );
  
  if (!response.data.success || !response.data.data) {
    throw new Error(response.data.error || 'Health check failed');
  }
  
  return response.data.data;
}

/**
 * Mock function for historical data (until backend endpoint is ready)
 */
export function generateMockHistory(count: number = 50): HistoricalPrediction[] {
  const history: HistoricalPrediction[] = [];
  const regimes: MarketRegime[] = ['Bull', 'Bear', 'Sideways', 'Crisis'];
  
  for (let i = 0; i < count; i++) {
    const timestamp = new Date();
    timestamp.setMinutes(timestamp.getMinutes() - (count - i) * 5);
    
    history.push({
      timestamp: timestamp.toISOString(),
      conviction: 0.3 + Math.random() * 0.6,
      confidence: 0.7 + Math.random() * 0.3,
      regime: regimes[Math.floor(Math.random() * regimes.length)],
      inputs: {
        pegy: Math.random(),
        insider: Math.random(),
        sentiment: Math.random(),
        vix: 10 + Math.random() * 60,
        regime: Math.floor(Math.random() * 4),
        time: 0.5,
      },
    });
  }
  
  return history;
}

export { API_BASE_URL };
