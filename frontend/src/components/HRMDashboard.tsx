/**
 * HRM Dashboard - Main Component
 * Complete dashboard for HRM visualization
 */

import React, { useState, useEffect, useCallback } from "react";
import {
  HRMInferenceRequest,
  HRMInferenceResponse,
  HistoricalPrediction,
} from "../types/hrm";
import { inferHRM, getHRMHealth, fetchHRMHistory } from "../api/hrm";
import { ConvictionGauge } from "./ConvictionGauge";
import { RegimeIndicator, RegimeCard } from "./RegimeIndicator";
import { HRMInputForm } from "./HRMInputForm";
import { SignalStrengthChart } from "./SignalStrengthChart";

export const HRMDashboard: React.FC = () => {
  const [result, setResult] = useState<HRMInferenceResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [health, setHealth] = useState<{
    status: string;
    model_loaded: boolean;
  } | null>(null);
  const [history, setHistory] = useState<HistoricalPrediction[]>([]);

  // Load initial data
  useEffect(() => {
    checkHealth();
    // Load history from backend
    fetchHRMHistory(30).then(setHistory);

    // Run initial analysis with default values
    handleAnalyze({
      pegy: 0.8,
      insider: 0.9,
      sentiment: 0.7,
      vix: 15,
      regime: 0,
      time: 0.5,
    });
  }, []);

  const checkHealth = async () => {
    try {
      const healthData = await getHRMHealth();
      setHealth({
        status: healthData.status,
        model_loaded: healthData.model_loaded,
      });
    } catch (err) {
      setHealth({ status: "unhealthy", model_loaded: false });
    }
  };

  const handleAnalyze = useCallback(async (request: HRMInferenceRequest) => {
    setLoading(true);
    setError(null);

    try {
      const response = await inferHRM(request);
      setResult(response);

      // Add to history
      const newEntry: HistoricalPrediction = {
        timestamp: new Date().toISOString(),
        conviction: response.conviction,
        confidence: response.confidence,
        regime: response.regime,
        inputs: request,
      };
      setHistory((prev) => [newEntry, ...prev].slice(0, 50));
    } catch (err) {
      setError(err instanceof Error ? err.message : "Analysis failed");
    } finally {
      setLoading(false);
    }
  }, []);

  return (
    <div className="min-h-screen bg-gray-50 p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-gray-900">
            🤖 HRM Trading Dashboard
          </h1>
          <p className="text-gray-600 mt-2">
            Hierarchical Reasoning Model - ML-powered trading conviction
          </p>

          {/* Health Status */}
          {health && (
            <div className="flex items-center gap-2 mt-4">
              <span className="text-sm text-gray-500">Model Status:</span>
              <span
                className={`inline-flex items-center px-3 py-1 rounded-full text-sm font-medium ${
                  health.status === "healthy"
                    ? "bg-green-100 text-green-800"
                    : "bg-red-100 text-red-800"
                }`}
              >
                <span
                  className={`w-2 h-2 rounded-full mr-2 ${
                    health.status === "healthy" ? "bg-green-500" : "bg-red-500"
                  }`}
                />
                {health.status === "healthy" ? "Online" : "Offline"}
                {health.model_loaded && " (ML Model Active)"}
              </span>
            </div>
          )}
        </div>

        {/* Error Display */}
        {error && (
          <div className="mb-6 p-4 bg-red-50 border border-red-200 rounded-lg text-red-700">
            <strong>Error:</strong> {error}
          </div>
        )}

        {/* Main Grid */}
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          {/* Left Column - Input Form */}
          <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
            <h2 className="text-lg font-semibold text-gray-900 mb-4">
              Market Signals
            </h2>
            <HRMInputForm onSubmit={handleAnalyze} loading={loading} />
          </div>

          {/* Middle Column - Results */}
          <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
            <h2 className="text-lg font-semibold text-gray-900 mb-4">
              HRM Analysis
            </h2>

            {result ? (
              <div className="space-y-6">
                {/* Conviction Gauge */}
                <div className="flex justify-center">
                  <ConvictionGauge value={result.conviction} size={220} />
                </div>

                {/* Regime */}
                <div className="flex justify-center">
                  <RegimeIndicator
                    regime={result.regime}
                    confidence={result.confidence}
                    size="lg"
                  />
                </div>

                {/* Signal Details */}
                <div className="grid grid-cols-2 gap-4 pt-4 border-t border-gray-100">
                  <div className="text-center">
                    <p className="text-sm text-gray-500">Confidence</p>
                    <p className="text-2xl font-bold text-blue-600">
                      {(result.confidence * 100).toFixed(1)}%
                    </p>
                  </div>
                  <div className="text-center">
                    <p className="text-sm text-gray-500">Signal Strength</p>
                    <p className="text-2xl font-bold text-purple-600">
                      {(result.signal_strength * 100).toFixed(1)}%
                    </p>
                  </div>
                </div>

                {/* Trading Signal */}
                <div
                  className={`p-4 rounded-lg text-center ${
                    result.should_trade
                      ? "bg-green-100 text-green-800"
                      : "bg-red-100 text-red-800"
                  }`}
                >
                  <p className="text-sm opacity-75">Trading Signal</p>
                  <p className="text-xl font-bold">
                    {result.should_trade ? "🚀 TRADE" : "⛔ HOLD"}
                  </p>
                </div>

                {/* Latency */}
                <p className="text-xs text-gray-400 text-center">
                  Inference latency: {result.latency_ms.toFixed(2)}ms
                </p>
              </div>
            ) : (
              <div className="text-center text-gray-400 py-12">
                Enter market signals to see HRM analysis
              </div>
            )}
          </div>

          {/* Right Column - History & Strategy */}
          <div className="space-y-6">
            {/* Strategy Card */}
            {result && (
              <RegimeCard
                regime={result.regime}
                recommendedStrategy={result.recommended_strategy}
              />
            )}

            {/* Source Info */}
            {result && (
              <div className="bg-blue-50 rounded-lg p-4 border border-blue-200">
                <p className="text-sm text-blue-800">
                  <span className="font-medium">Source:</span>{" "}
                  {result.source === "MLModel"
                    ? "🧠 Neural Network (9,347 params)"
                    : "📊 Heuristic Formula"}
                </p>
              </div>
            )}

            {/* History Chart */}
            <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
              <h3 className="text-sm font-semibold text-gray-700 mb-4">
                Conviction History
              </h3>
              <SignalStrengthChart data={history} />
            </div>
          </div>
        </div>

        {/* Footer */}
        <footer className="mt-12 text-center text-sm text-gray-500">
          <p>HRM v1.0 | 9,347 parameters | burn-ndarray backend</p>
          <p className="mt-1">Sprint 44: Frontend Dashboard ✅</p>
        </footer>
      </div>
    </div>
  );
};

export default HRMDashboard;
