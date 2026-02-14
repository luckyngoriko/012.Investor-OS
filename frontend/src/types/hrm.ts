/**
 * HRM TypeScript Types
 * Sprint 44: Frontend Dashboard
 */

/**
 * HRM Inference Request
 */
export interface HRMInferenceRequest {
  pegy: number;      // 0.0 - 1.0
  insider: number;   // 0.0 - 1.0
  sentiment: number; // 0.0 - 1.0
  vix: number;       // typically 10-80
  regime: number;    // 0=Bull, 1=Bear, 2=Sideways, 3=Crisis
  time?: number;     // 0.0 - 1.0 (default: 0.5)
}

/**
 * HRM Inference Response
 */
export interface HRMInferenceResponse {
  conviction: number;           // 0.0 - 1.0
  confidence: number;           // 0.0 - 1.0
  regime: MarketRegime;         // Detected regime
  should_trade: boolean;        // Trading signal
  recommended_strategy: string; // Strategy recommendation
  signal_strength: number;      // conviction * confidence
  source: 'MLModel' | 'Heuristic';
  latency_ms: number;
}

/**
 * Market Regime
 */
export type MarketRegime = 
  | 'Bull' 
  | 'Bear' 
  | 'Sideways' 
  | 'Crisis' 
  | 'StrongUptrend' 
  | 'StrongDowntrend' 
  | 'Trending' 
  | 'Ranging' 
  | 'Volatile';

/**
 * HRM Health Response
 */
export interface HRMHealthResponse {
  status: 'healthy' | 'degraded';
  model_loaded: boolean;
  model_version: string;
  parameters: number;
  backend: string;
}

/**
 * API Response Wrapper
 */
export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

/**
 * Historical Prediction Entry
 */
export interface HistoricalPrediction {
  timestamp: string;
  conviction: number;
  confidence: number;
  regime: MarketRegime;
  inputs: HRMInferenceRequest;
}

/**
 * Trading Signal
 */
export interface TradingSignal {
  timestamp: string;
  conviction: number;
  signal: 'BUY' | 'SELL' | 'HOLD';
  strength: 'STRONG' | 'MODERATE' | 'WEAK';
  strategy: string;
}
