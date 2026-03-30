"use client";

import { useEffect, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  Activity,
  BarChart3,
  Brain,
  CheckCircle2,
  Clock,
  Cpu,
  Layers,
  RefreshCw,
  Target,
  TrendingDown,
  TrendingUp,
  Zap,
  AlertTriangle,
  Gauge,
  ArrowUpRight,
  ArrowDownRight,
  Minus,
} from "lucide-react";
// ── Types ──────────────────────────────────────────────────────────

interface Prediction {
  id: string;
  model_name: string;
  model_version: string;
  symbol: string;
  prediction_type: string;
  horizon: string | null;
  predicted_value: Record<string, unknown>;
  confidence: number;
  is_fallback: boolean;
  predicted_at: string;
  error_metric: number | null;
}

interface ModelInfo {
  model_name: string;
  model_version: string;
  model_type: string;
  tier: number;
  status: string;
  metrics: Record<string, unknown>;
}

// ── Helpers ────────────────────────────────────────────────────────

function confidenceBg(c: number): string {
  if (c >= 0.7)
    return "bg-emerald-500/20 text-emerald-400 border-emerald-500/30";
  if (c >= 0.4) return "bg-amber-500/20 text-amber-400 border-amber-500/30";
  return "bg-red-500/20 text-red-400 border-red-500/30";
}

function tierLabel(tier: number): string {
  return tier === 1 ? "CPU" : tier === 2 ? "GPU" : "Research";
}

function tierStyle(tier: number): string {
  if (tier === 1) return "bg-blue-500/10 text-blue-400 border-blue-500/20";
  if (tier === 2)
    return "bg-purple-500/10 text-purple-400 border-purple-500/20";
  return "bg-orange-500/10 text-orange-400 border-orange-500/20";
}

function directionIcon(dir: string) {
  if (dir === "long")
    return <ArrowUpRight className="w-4 h-4 text-emerald-400" />;
  if (dir === "short")
    return <ArrowDownRight className="w-4 h-4 text-red-400" />;
  return <Minus className="w-4 h-4 text-slate-400" />;
}

function modelIcon(model: string) {
  switch (model) {
    case "catboost":
      return <BarChart3 className="w-4 h-4 text-amber-400" />;
    case "garch":
      return <Activity className="w-4 h-4 text-cyan-400" />;
    case "finbert":
      return <Brain className="w-4 h-4 text-pink-400" />;
    case "kronos":
      return <Zap className="w-4 h-4 text-purple-400" />;
    case "chronos":
      return <Target className="w-4 h-4 text-blue-400" />;
    case "ets":
      return <TrendingUp className="w-4 h-4 text-green-400" />;
    default:
      return <Cpu className="w-4 h-4 text-slate-400" />;
  }
}

// ── Confidence Gauge ───────────────────────────────────────────────

function ConfidenceGauge({ value }: { value: number }) {
  const pct = Math.round(value * 100);
  const color = value >= 0.7 ? "#10b981" : value >= 0.4 ? "#f59e0b" : "#ef4444";
  const circumference = 2 * Math.PI * 40;
  const dashoffset = circumference * (1 - value);

  return (
    <div className="relative w-28 h-28">
      <svg viewBox="0 0 100 100" className="w-full h-full -rotate-90">
        <circle
          cx="50"
          cy="50"
          r="40"
          fill="none"
          stroke="#1e293b"
          strokeWidth="8"
        />
        <motion.circle
          cx="50"
          cy="50"
          r="40"
          fill="none"
          stroke={color}
          strokeWidth="8"
          strokeLinecap="round"
          strokeDasharray={circumference}
          initial={{ strokeDashoffset: circumference }}
          animate={{ strokeDashoffset: dashoffset }}
          transition={{ duration: 1.2, ease: "easeOut" }}
        />
      </svg>
      <div className="absolute inset-0 flex flex-col items-center justify-center">
        <span className="text-2xl font-bold" style={{ color }}>
          {pct}%
        </span>
        <span className="text-[10px] text-slate-500 uppercase tracking-wider">
          confidence
        </span>
      </div>
    </div>
  );
}

// ── Page Component ─────────────────────────────────────────────────

export default function PredictionsPage() {
  const [predictions, setPredictions] = useState<Prediction[]>([]);
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchData = async () => {
    setLoading(true);
    setError(null);
    try {
      const [predRes, modelRes] = await Promise.all([
        fetch("/api/predictions/history?limit=50"),
        fetch("/api/models/registry"),
      ]);
      if (predRes.ok) {
        const predData = await predRes.json();
        setPredictions(predData.data || []);
      }
      if (modelRes.ok) {
        const modelData = await modelRes.json();
        setModels(modelData.data || []);
      }
    } catch {
      setError("Failed to load prediction data");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, 30000);
    return () => clearInterval(interval);
  }, []);

  const avgConfidence =
    predictions.length > 0
      ? predictions.reduce((s, p) => s + p.confidence, 0) / predictions.length
      : 0;
  const fallbackCount = predictions.filter((p) => p.is_fallback).length;
  const activeModels = models.filter((m) => m.status === "active").length;
  const resolvedCount = predictions.filter(
    (p) => p.error_metric !== null,
  ).length;

  const stats = [
    {
      label: "Predictions",
      value: predictions.length,
      icon: Zap,
      color: "text-amber-400",
      bg: "bg-amber-500/10",
    },
    {
      label: "Avg Confidence",
      value: `${(avgConfidence * 100).toFixed(0)}%`,
      icon: Gauge,
      color: avgConfidence >= 0.7 ? "text-emerald-400" : "text-amber-400",
      bg: avgConfidence >= 0.7 ? "bg-emerald-500/10" : "bg-amber-500/10",
    },
    {
      label: "Active Models",
      value: activeModels,
      icon: Layers,
      color: "text-purple-400",
      bg: "bg-purple-500/10",
    },
    {
      label: "Resolved",
      value: resolvedCount,
      icon: CheckCircle2,
      color: "text-cyan-400",
      bg: "bg-cyan-500/10",
    },
  ];

  return (
    <div className="min-h-screen text-slate-100">
      <div className="p-6 space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-bold tracking-tight flex items-center gap-3">
              <div className="p-2 rounded-xl bg-purple-500/10 border border-purple-500/20">
                <Brain className="w-7 h-7 text-purple-400" />
              </div>
              ML Predictions
            </h1>
            <p className="text-slate-500 mt-2 text-sm">
              Real-time AI predictions from {models.length} models across 3
              tiers
            </p>
          </div>
          <button
            onClick={fetchData}
            className="flex items-center gap-2 px-4 py-2.5 bg-slate-800/60 backdrop-blur-sm border border-slate-700/50 rounded-xl hover:bg-slate-700/60 transition-colors duration-200 cursor-pointer"
          >
            <RefreshCw className={`w-4 h-4 ${loading ? "animate-spin" : ""}`} />
            <span className="text-sm">Refresh</span>
          </button>
        </div>

        {error && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            className="bg-red-500/10 backdrop-blur-sm border border-red-500/20 rounded-xl p-4 mb-6 flex items-center gap-3"
          >
            <AlertTriangle className="w-5 h-5 text-red-400 shrink-0" />
            <span className="text-red-300 text-sm">{error}</span>
          </motion.div>
        )}

        {/* Stats Row */}
        <div className="grid grid-cols-4 gap-4">
          {stats.map((stat, i) => (
            <motion.div
              key={stat.label}
              initial={{ opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: i * 0.08, duration: 0.4 }}
              className="bg-slate-900/80 rounded-xl p-5 border border-slate-700/50 hover:border-slate-600/60 transition-colors duration-200"
            >
              <div className="flex items-center gap-3 mb-3">
                <div className={`p-2 rounded-lg ${stat.bg}`}>
                  <stat.icon className={`w-4 h-4 ${stat.color}`} />
                </div>
                <span className="text-slate-500 text-xs uppercase tracking-wider">
                  {stat.label}
                </span>
              </div>
              <span className={`text-2xl font-bold ${stat.color}`}>
                {stat.value}
              </span>
            </motion.div>
          ))}
        </div>

        <div className="grid grid-cols-3 gap-6">
          {/* Consensus Gauge */}
          <motion.div
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ delay: 0.3 }}
            className="bg-slate-900/80 rounded-xl border border-slate-700/50 p-6 flex flex-col items-center justify-center"
          >
            <h3 className="text-sm text-slate-500 uppercase tracking-wider mb-4">
              Consensus Confidence
            </h3>
            <ConfidenceGauge value={avgConfidence} />
            {fallbackCount > 0 && (
              <p className="text-orange-400/80 text-xs mt-3 flex items-center gap-1">
                <AlertTriangle className="w-3 h-3" />
                {fallbackCount} fallback predictions
              </p>
            )}
          </motion.div>

          {/* Model Registry Grid */}
          <div className="col-span-2 bg-slate-900/80 rounded-xl border border-slate-700/50 p-6">
            <h3 className="text-sm text-slate-500 uppercase tracking-wider mb-4 flex items-center gap-2">
              <Layers className="w-4 h-4" />
              Model Registry
            </h3>
            {models.length === 0 ? (
              <div className="text-slate-600 text-sm py-8 text-center">
                No models registered. Deploy models to start generating
                predictions.
              </div>
            ) : (
              <div className="grid grid-cols-2 gap-3">
                {models.map((m) => (
                  <div
                    key={`${m.model_name}-${m.model_version}`}
                    className="bg-slate-800/40 rounded-lg p-3 border border-slate-700/30 hover:border-slate-600/50 transition-colors duration-200 cursor-pointer"
                  >
                    <div className="flex items-center justify-between mb-1.5">
                      <div className="flex items-center gap-2">
                        {modelIcon(m.model_name)}
                        <span className="font-medium text-sm">
                          {m.model_name}
                        </span>
                      </div>
                      <span
                        className={`text-[10px] px-2 py-0.5 rounded-full border ${tierStyle(m.tier)}`}
                      >
                        {tierLabel(m.tier)}
                      </span>
                    </div>
                    <div className="flex items-center gap-2 text-xs text-slate-500">
                      <span>v{m.model_version}</span>
                      <span className="w-1 h-1 rounded-full bg-slate-600" />
                      <span
                        className={
                          m.status === "active"
                            ? "text-emerald-400"
                            : "text-slate-600"
                        }
                      >
                        {m.status}
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>

        {/* Recent Predictions */}
        <div className="bg-slate-900/80 rounded-xl border border-slate-700/50 p-6">
          <h3 className="text-sm text-slate-500 uppercase tracking-wider mb-4 flex items-center gap-2">
            <Activity className="w-4 h-4" />
            Recent Predictions
          </h3>
          {predictions.length === 0 ? (
            <div className="text-slate-600 text-sm py-12 text-center">
              <Brain className="w-12 h-12 text-slate-800 mx-auto mb-3" />
              No predictions yet. Models will generate predictions when market
              data flows in.
            </div>
          ) : (
            <div className="space-y-2">
              <AnimatePresence>
                {predictions.slice(0, 25).map((p, i) => {
                  const direction = String(
                    p.predicted_value?.direction || "neutral",
                  );

                  return (
                    <motion.div
                      key={p.id}
                      initial={{ opacity: 0, x: -8 }}
                      animate={{ opacity: 1, x: 0 }}
                      transition={{ delay: i * 0.03, duration: 0.3 }}
                      className="flex items-center justify-between bg-slate-800/30 rounded-lg px-4 py-3 border border-slate-700/20 hover:border-slate-600/40 transition-colors duration-200"
                    >
                      <div className="flex items-center gap-3">
                        {directionIcon(direction)}
                        <div>
                          <span className="font-medium text-sm">
                            {p.symbol}
                          </span>
                          <span className="text-slate-500 text-xs ml-2 inline-flex items-center gap-1">
                            {modelIcon(p.model_name)}
                            {p.model_name}
                          </span>
                          {p.is_fallback && (
                            <span className="text-orange-400/70 text-[10px] ml-2 px-1.5 py-0.5 rounded bg-orange-500/10">
                              fallback
                            </span>
                          )}
                        </div>
                      </div>

                      <div className="flex items-center gap-5 text-xs">
                        <span
                          className={`px-2 py-0.5 rounded-full border ${confidenceBg(p.confidence)}`}
                        >
                          {(p.confidence * 100).toFixed(0)}%
                        </span>
                        <span className="text-slate-500 w-16">
                          {p.prediction_type}
                        </span>
                        {p.horizon && (
                          <span className="text-slate-600 w-8">
                            {p.horizon}
                          </span>
                        )}
                        <span className="text-slate-700 text-[10px] w-16 text-right">
                          {new Date(p.predicted_at).toLocaleTimeString([], {
                            hour: "2-digit",
                            minute: "2-digit",
                          })}
                        </span>
                        {p.error_metric !== null && (
                          <CheckCircle2 className="w-3.5 h-3.5 text-emerald-500/60" />
                        )}
                      </div>
                    </motion.div>
                  );
                })}
              </AnimatePresence>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
