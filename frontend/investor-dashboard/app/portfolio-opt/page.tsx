"use client";

import { useEffect, useMemo, useState } from "react";
import { motion } from "framer-motion";
import {
  BarChart3,
  Calculator,
  Download,
  PieChart,
  RefreshCw,
  Target,
  TrendingUp,
} from "lucide-react";
import Sidebar from "@/components/sidebar";
import {
  type EfficientFrontierResponse,
  type PortfolioMethod,
  type PortfolioOptimizeResponse,
  fetchEfficientFrontier,
  fetchPortfolioOptimization,
} from "@/lib/domain-api";

interface MethodOption {
  id: string;
  name: string;
  description: string;
}

function toMethodId(name: string): string {
  return name.toLowerCase().replace(/[^a-z0-9]+/g, "-");
}

function formatPercent(value: number): string {
  return `${(value * 100).toFixed(1)}%`;
}

function parsePercent(raw: string): number {
  const normalized = Number.parseFloat(raw.replace("%", ""));
  if (!Number.isFinite(normalized)) {
    return 0;
  }
  return normalized / 100;
}

function objectiveLabel(value: string): string {
  return value.replace(/([a-z0-9])([A-Z])/g, "$1 $2");
}

function methodOptionsFromPayload(methods: PortfolioMethod[]): MethodOption[] {
  return methods.map((method) => ({
    id: toMethodId(method.name),
    name: method.name,
    description: method.description,
  }));
}

export default function PortfolioOptimizationPage() {
  const [selectedMethod, setSelectedMethod] = useState("");
  const [objective, setObjective] = useState("");
  const [isOptimizing, setIsOptimizing] = useState(false);
  const [showResults, setShowResults] = useState(false);
  const [optimizePayload, setOptimizePayload] = useState<PortfolioOptimizeResponse | null>(null);
  const [frontierPayload, setFrontierPayload] = useState<EfficientFrontierResponse | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const loadPortfolioData = async () => {
    setErrorMessage(null);
    const [optimizeResponse, frontierResponse] = await Promise.all([
      fetchPortfolioOptimization(),
      fetchEfficientFrontier(),
    ]);
    setOptimizePayload(optimizeResponse);
    setFrontierPayload(frontierResponse);

    const options = methodOptionsFromPayload(optimizeResponse.methods);
    if (options.length > 0) {
      setSelectedMethod((current) => current || options[0].id);
    }
    if (optimizeResponse.objectives.length > 0) {
      setObjective((current) => current || optimizeResponse.objectives[0]);
    }
  };

  useEffect(() => {
    let mounted = true;

    const load = async () => {
      setIsLoading(true);
      try {
        await loadPortfolioData();
      } catch (error) {
        if (!mounted) return;
        setErrorMessage(
          error instanceof Error ? error.message : "Failed to load portfolio data",
        );
      } finally {
        if (mounted) {
          setIsLoading(false);
        }
      }
    };

    void load();

    return () => {
      mounted = false;
    };
  }, []);

  const methodOptions = useMemo(
    () => methodOptionsFromPayload(optimizePayload?.methods ?? []),
    [optimizePayload],
  );

  const objectiveOptions = optimizePayload?.objectives ?? [];
  const output = optimizePayload?.example_optimization.output;
  const allocation = output
    ? Object.entries(output.weights).map(([symbol, weight]) => ({
        symbol,
        weight: weight * 100,
      }))
    : [];
  const totalWeight = allocation.reduce((sum, item) => sum + item.weight, 0);
  const portfolioReturn = output ? parsePercent(output.expected_return) : 0;
  const portfolioRisk = output ? parsePercent(output.expected_risk) : 0;
  const sharpeRatio = output?.sharpe_ratio ?? 0;

  const handleOptimize = async () => {
    setIsOptimizing(true);
    setErrorMessage(null);
    try {
      const optimizeResponse = await fetchPortfolioOptimization();
      setOptimizePayload(optimizeResponse);
      setShowResults(true);
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "Optimization failed");
    } finally {
      setIsOptimizing(false);
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0f1c] via-[#111827] to-[#0a0f1c] flex">
      <Sidebar />
      <main className="flex-1 min-h-screen p-6 lg:p-8">
        <div className="max-w-7xl mx-auto space-y-6">
          <motion.div initial={{ opacity: 0, y: -20 }} animate={{ opacity: 1, y: 0 }}>
            <div className="flex items-center gap-3 mb-2">
              <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-blue-500/20 to-blue-600/10 flex items-center justify-center">
                <BarChart3 className="w-5 h-5 text-blue-400" />
              </div>
              <div>
                <h1 className="text-2xl font-bold text-white">Portfolio Optimization</h1>
                <p className="text-gray-400 text-sm">
                  Backend-driven optimization methods, objectives, and frontier data
                </p>
              </div>
            </div>
          </motion.div>

          {errorMessage && (
            <div className="rounded-xl border border-rose-500/40 bg-rose-500/10 p-4 text-sm text-rose-300">
              {errorMessage}
            </div>
          )}

          {isLoading ? (
            <div className="glass-card rounded-2xl p-6 text-gray-300 flex items-center gap-3">
              <RefreshCw className="w-4 h-4 animate-spin" />
              Loading portfolio optimization data...
            </div>
          ) : (
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
              <div className="lg:col-span-2 space-y-6">
                <motion.div
                  initial={{ opacity: 0, y: 20 }}
                  animate={{ opacity: 1, y: 0 }}
                  className="glass-card rounded-2xl p-6"
                >
                  <h3 className="text-lg font-semibold text-white mb-4">Optimization Method</h3>
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                    {methodOptions.map((method) => (
                      <button
                        key={method.id}
                        onClick={() => setSelectedMethod(method.id)}
                        className={`p-4 rounded-xl text-left transition-all border ${
                          selectedMethod === method.id
                            ? "bg-blue-600/20 border-blue-500/50"
                            : "bg-gray-800/30 border-gray-700 hover:border-gray-600"
                        }`}
                      >
                        <div className="flex items-center justify-between">
                          <span
                            className={`font-medium ${
                              selectedMethod === method.id ? "text-blue-400" : "text-white"
                            }`}
                          >
                            {method.name}
                          </span>
                          {selectedMethod === method.id ? (
                            <div className="w-2 h-2 rounded-full bg-blue-400" />
                          ) : null}
                        </div>
                        <p className="text-sm text-gray-400 mt-1">{method.description}</p>
                      </button>
                    ))}
                  </div>
                </motion.div>

                <motion.div
                  initial={{ opacity: 0, y: 20 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: 0.1 }}
                  className="glass-card rounded-2xl p-6"
                >
                  <h3 className="text-lg font-semibold text-white mb-4">
                    Optimization Objective
                  </h3>
                  <div className="flex flex-wrap gap-2">
                    {objectiveOptions.map((value) => (
                      <button
                        key={value}
                        onClick={() => setObjective(value)}
                        className={`flex items-center gap-2 px-4 py-2 rounded-lg transition-all ${
                          objective === value
                            ? "bg-emerald-600/20 text-emerald-400 border border-emerald-500/50"
                            : "bg-gray-800 text-gray-400 border border-gray-700 hover:border-gray-600"
                        }`}
                      >
                        {value.includes("Return") ? (
                          <TrendingUp className="w-4 h-4" />
                        ) : value.includes("Risk") ? (
                          <Target className="w-4 h-4" />
                        ) : value.includes("Sharpe") ? (
                          <BarChart3 className="w-4 h-4" />
                        ) : (
                          <PieChart className="w-4 h-4" />
                        )}
                        <span className="text-sm font-medium">{objectiveLabel(value)}</span>
                      </button>
                    ))}
                  </div>
                </motion.div>

                <motion.div
                  initial={{ opacity: 0, y: 20 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: 0.2 }}
                  className="glass-card rounded-2xl p-6"
                >
                  <h3 className="text-lg font-semibold text-white mb-4">Efficient Frontier</h3>
                  <div className="space-y-3">
                    {(frontierPayload?.frontier_points ?? []).map((point) => (
                      <div
                        key={`${point.portfolio}-${point.risk}`}
                        className="flex items-center justify-between p-3 bg-gray-800/30 rounded-lg"
                      >
                        <div className="flex items-center gap-3">
                          <div className="w-3 h-3 rounded-full bg-blue-500" />
                          <span className="text-white font-medium">{point.portfolio}</span>
                        </div>
                        <div className="flex gap-4 text-sm">
                          <span className="text-gray-400">
                            Risk: <span className="text-amber-400">{point.risk}</span>
                          </span>
                          <span className="text-gray-400">
                            Return: <span className="text-emerald-400">{point.return}</span>
                          </span>
                        </div>
                      </div>
                    ))}
                  </div>
                </motion.div>
              </div>

              <div className="space-y-6">
                <motion.div
                  initial={{ opacity: 0, x: 20 }}
                  animate={{ opacity: 1, x: 0 }}
                  className="glass-card rounded-2xl p-6"
                >
                  <button
                    onClick={handleOptimize}
                    disabled={isOptimizing}
                    className="w-full py-3 bg-blue-600 hover:bg-blue-500 disabled:bg-gray-700 text-white font-medium rounded-lg transition-colors flex items-center justify-center gap-2"
                  >
                    {isOptimizing ? (
                      <>
                        <RefreshCw className="w-5 h-5 animate-spin" />
                        Optimizing...
                      </>
                    ) : (
                      <>
                        <Calculator className="w-5 h-5" />
                        Run Optimization
                      </>
                    )}
                  </button>
                </motion.div>

                {showResults ? (
                  <motion.div
                    initial={{ opacity: 0, scale: 0.95 }}
                    animate={{ opacity: 1, scale: 1 }}
                    className="glass-card rounded-2xl p-6"
                  >
                    <h3 className="text-lg font-semibold text-white mb-4">Optimized Portfolio</h3>

                    <div className="grid grid-cols-2 gap-3 mb-4">
                      <div className="p-3 bg-gray-800/30 rounded-lg text-center">
                        <p className="text-xs text-gray-400">Expected Return</p>
                        <p className="text-lg font-bold text-emerald-400">
                          {formatPercent(portfolioReturn)}
                        </p>
                      </div>
                      <div className="p-3 bg-gray-800/30 rounded-lg text-center">
                        <p className="text-xs text-gray-400">Risk (Std Dev)</p>
                        <p className="text-lg font-bold text-amber-400">
                          {formatPercent(portfolioRisk)}
                        </p>
                      </div>
                      <div className="p-3 bg-gray-800/30 rounded-lg text-center">
                        <p className="text-xs text-gray-400">Sharpe Ratio</p>
                        <p className="text-lg font-bold text-blue-400">
                          {sharpeRatio.toFixed(2)}
                        </p>
                      </div>
                      <div className="p-3 bg-gray-800/30 rounded-lg text-center">
                        <p className="text-xs text-gray-400">Diversification</p>
                        <p className="text-lg font-bold text-purple-400">
                          {output?.diversification_ratio.toFixed(2) ?? "0.00"}
                        </p>
                      </div>
                    </div>

                    <div className="space-y-2">
                      <p className="text-sm text-gray-400 mb-2">
                        Asset Allocation ({totalWeight.toFixed(1)}%)
                      </p>
                      {allocation.map((asset) => (
                        <div key={asset.symbol} className="flex items-center gap-2">
                          <span className="text-sm text-white w-16">{asset.symbol}</span>
                          <div className="flex-1 h-2 bg-gray-700 rounded-full overflow-hidden">
                            <div
                              className="h-full bg-blue-500 rounded-full"
                              style={{ width: `${asset.weight}%` }}
                            />
                          </div>
                          <span className="text-sm text-gray-400 w-12 text-right">
                            {asset.weight.toFixed(1)}%
                          </span>
                        </div>
                      ))}
                    </div>

                    <button className="w-full mt-4 py-2 bg-gray-800 hover:bg-gray-700 text-gray-300 text-sm font-medium rounded-lg transition-colors flex items-center justify-center gap-2">
                      <Download className="w-4 h-4" /> Export Report
                    </button>
                  </motion.div>
                ) : null}
              </div>
            </div>
          )}
        </div>
      </main>
    </div>
  );
}
