"use client";

import { useState } from "react";
import { motion } from "framer-motion";
import {
  BarChart3,
  TrendingUp,
  TrendingDown,
  Activity,
  Target,
  Loader2,
  AlertTriangle,
  DollarSign,
  Percent,
  Hash,
} from "lucide-react";

// ── Types ──────────────────────────────────────────────────────────

interface BacktestResult {
  total_return_pct: number;
  sharpe_ratio: number;
  max_drawdown_pct: number;
  win_rate: number;
  total_trades: number;
  equity_curve: [string, number][];
}

// ── Page Component ─────────────────────────────────────────────────

export default function BacktestPage() {
  const [symbol, setSymbol] = useState("BTCUSDT");
  const [startDate, setStartDate] = useState("2025-01-01");
  const [endDate, setEndDate] = useState("2025-12-31");
  const [initialCapital, setInitialCapital] = useState("10000");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<BacktestResult | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    setResult(null);

    try {
      const res = await fetch("/api/backtest", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          symbol,
          start_date: new Date(startDate).toISOString(),
          end_date: new Date(endDate).toISOString(),
          initial_capital: parseFloat(initialCapital),
          strategy_config: null,
        }),
      });

      const data = await res.json();
      if (data.success) {
        setResult(data.data);
      } else {
        setError(data.error?.message || "Backtest failed");
      }
    } catch {
      setError("Failed to connect to backtest API");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen text-slate-100">
      <div className="p-6 space-y-6">
        {/* Header */}
        <motion.div
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
        >
          <div className="flex items-center gap-3 mb-2">
            <div className="p-2 rounded-xl bg-cyan-500/10 border border-cyan-500/20">
              <BarChart3 className="w-7 h-7 text-cyan-400" />
            </div>
            <div>
              <h1 className="text-2xl font-bold text-white">
                Strategy Backtester
              </h1>
              <p className="text-gray-400 text-sm">
                Test momentum strategies against historical price data
              </p>
            </div>
          </div>
        </motion.div>

        {/* Form */}
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.1 }}
          className="bg-gray-800/50 border border-gray-700/50 rounded-2xl p-6"
        >
          <form onSubmit={handleSubmit} className="space-y-4">
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
              {/* Symbol */}
              <div>
                <label className="block text-sm font-medium text-slate-400 mb-1.5">
                  Symbol
                </label>
                <input
                  type="text"
                  value={symbol}
                  onChange={(e) => setSymbol(e.target.value.toUpperCase())}
                  className="w-full px-4 py-2.5 bg-gray-900/50 border border-gray-700 rounded-xl text-white placeholder-slate-500 focus:outline-none focus:border-cyan-500/50 focus:ring-1 focus:ring-cyan-500/30 transition-colors"
                  placeholder="e.g. BTCUSDT"
                  required
                />
              </div>

              {/* Start Date */}
              <div>
                <label className="block text-sm font-medium text-slate-400 mb-1.5">
                  Start Date
                </label>
                <input
                  type="date"
                  value={startDate}
                  onChange={(e) => setStartDate(e.target.value)}
                  className="w-full px-4 py-2.5 bg-gray-900/50 border border-gray-700 rounded-xl text-white focus:outline-none focus:border-cyan-500/50 focus:ring-1 focus:ring-cyan-500/30 transition-colors"
                  required
                />
              </div>

              {/* End Date */}
              <div>
                <label className="block text-sm font-medium text-slate-400 mb-1.5">
                  End Date
                </label>
                <input
                  type="date"
                  value={endDate}
                  onChange={(e) => setEndDate(e.target.value)}
                  className="w-full px-4 py-2.5 bg-gray-900/50 border border-gray-700 rounded-xl text-white focus:outline-none focus:border-cyan-500/50 focus:ring-1 focus:ring-cyan-500/30 transition-colors"
                  required
                />
              </div>

              {/* Initial Capital */}
              <div>
                <label className="block text-sm font-medium text-slate-400 mb-1.5">
                  Initial Capital (USD)
                </label>
                <input
                  type="number"
                  value={initialCapital}
                  onChange={(e) => setInitialCapital(e.target.value)}
                  className="w-full px-4 py-2.5 bg-gray-900/50 border border-gray-700 rounded-xl text-white placeholder-slate-500 focus:outline-none focus:border-cyan-500/50 focus:ring-1 focus:ring-cyan-500/30 transition-colors"
                  placeholder="10000"
                  min="1"
                  step="any"
                  required
                />
              </div>
            </div>

            <div className="flex items-center gap-3 pt-2">
              <button
                type="submit"
                disabled={loading}
                className="flex items-center gap-2 px-6 py-2.5 bg-cyan-600 hover:bg-cyan-500 disabled:bg-gray-700 disabled:cursor-not-allowed text-white rounded-xl font-medium transition-colors duration-200 cursor-pointer"
              >
                {loading ? (
                  <>
                    <Loader2 className="w-4 h-4 animate-spin" />
                    Running Backtest...
                  </>
                ) : (
                  <>
                    <Activity className="w-4 h-4" />
                    Run Backtest
                  </>
                )}
              </button>
              <span className="text-xs text-slate-500">
                Strategy: Price &gt; 20-SMA (momentum)
              </span>
            </div>
          </form>
        </motion.div>

        {/* Error */}
        {error && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            className="bg-red-500/10 border border-red-500/30 rounded-xl p-4 flex items-center gap-3"
          >
            <AlertTriangle className="w-5 h-5 text-red-400 shrink-0" />
            <span className="text-red-300 text-sm">{error}</span>
          </motion.div>
        )}

        {/* Results */}
        {result && (
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.15 }}
            className="space-y-6"
          >
            {/* Stats Cards */}
            <div className="grid grid-cols-2 md:grid-cols-5 gap-4">
              <StatCard
                label="Total Return"
                value={`${result.total_return_pct >= 0 ? "+" : ""}${result.total_return_pct.toFixed(2)}%`}
                icon={result.total_return_pct >= 0 ? TrendingUp : TrendingDown}
                color={result.total_return_pct >= 0 ? "emerald" : "red"}
              />
              <StatCard
                label="Sharpe Ratio"
                value={result.sharpe_ratio.toFixed(2)}
                icon={Target}
                color={
                  result.sharpe_ratio >= 1
                    ? "emerald"
                    : result.sharpe_ratio >= 0
                      ? "amber"
                      : "red"
                }
              />
              <StatCard
                label="Max Drawdown"
                value={`-${result.max_drawdown_pct.toFixed(2)}%`}
                icon={TrendingDown}
                color={
                  result.max_drawdown_pct <= 10
                    ? "emerald"
                    : result.max_drawdown_pct <= 25
                      ? "amber"
                      : "red"
                }
              />
              <StatCard
                label="Win Rate"
                value={`${result.win_rate.toFixed(1)}%`}
                icon={Percent}
                color={result.win_rate >= 50 ? "emerald" : "amber"}
              />
              <StatCard
                label="Total Trades"
                value={String(result.total_trades)}
                icon={Hash}
                color="cyan"
              />
            </div>

            {/* Summary */}
            <div className="bg-gray-800/50 border border-gray-700/50 rounded-2xl p-6">
              <h2 className="text-lg font-semibold text-white mb-4 flex items-center gap-2">
                <DollarSign className="w-5 h-5 text-cyan-400" />
                Backtest Summary
              </h2>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm">
                <div className="space-y-2">
                  <InfoRow label="Symbol" value={symbol} />
                  <InfoRow
                    label="Period"
                    value={`${startDate} to ${endDate}`}
                  />
                  <InfoRow
                    label="Initial Capital"
                    value={`$${parseFloat(initialCapital).toLocaleString()}`}
                  />
                  <InfoRow
                    label="Final Equity"
                    value={
                      result.equity_curve.length > 0
                        ? `$${result.equity_curve[result.equity_curve.length - 1][1].toLocaleString(undefined, { maximumFractionDigits: 2 })}`
                        : "N/A"
                    }
                  />
                </div>
                <div className="space-y-2">
                  <InfoRow label="Strategy" value="Momentum (20-SMA)" />
                  <InfoRow
                    label="Data Points"
                    value={`${result.equity_curve.length} candles`}
                  />
                  <InfoRow
                    label="P&L"
                    value={
                      result.equity_curve.length > 0
                        ? `$${(result.equity_curve[result.equity_curve.length - 1][1] - parseFloat(initialCapital)).toLocaleString(undefined, { maximumFractionDigits: 2 })}`
                        : "N/A"
                    }
                  />
                  <InfoRow
                    label="Risk/Reward"
                    value={
                      result.max_drawdown_pct > 0
                        ? (
                            result.total_return_pct / result.max_drawdown_pct
                          ).toFixed(2)
                        : "N/A"
                    }
                  />
                </div>
              </div>
            </div>
          </motion.div>
        )}
      </div>
    </div>
  );
}

// ── Sub-components ─────────────────────────────────────────────────

function StatCard({
  label,
  value,
  icon: Icon,
  color,
}: {
  label: string;
  value: string;
  icon: React.ComponentType<{ className?: string }>;
  color: string;
}) {
  const colorMap: Record<string, { bg: string; text: string; border: string }> =
    {
      emerald: {
        bg: "bg-emerald-500/10",
        text: "text-emerald-400",
        border: "border-emerald-500/20",
      },
      red: {
        bg: "bg-red-500/10",
        text: "text-red-400",
        border: "border-red-500/20",
      },
      amber: {
        bg: "bg-amber-500/10",
        text: "text-amber-400",
        border: "border-amber-500/20",
      },
      cyan: {
        bg: "bg-cyan-500/10",
        text: "text-cyan-400",
        border: "border-cyan-500/20",
      },
    };

  const c = colorMap[color] || colorMap.cyan;

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.95 }}
      animate={{ opacity: 1, scale: 1 }}
      className={`${c.bg} border ${c.border} rounded-2xl p-4`}
    >
      <div className="flex items-center gap-2 mb-2">
        <Icon className={`w-4 h-4 ${c.text}`} />
        <span className="text-xs text-slate-400 uppercase tracking-wider">
          {label}
        </span>
      </div>
      <div className={`text-2xl font-bold ${c.text}`}>{value}</div>
    </motion.div>
  );
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between items-center py-1.5 border-b border-gray-700/30">
      <span className="text-slate-400">{label}</span>
      <span className="text-white font-medium">{value}</span>
    </div>
  );
}
