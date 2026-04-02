"use client";

import { useState } from "react";
import { motion } from "framer-motion";
import {
  Settings,
  Brain,
  Shield,
  Cpu,
  Save,
  Loader2,
  AlertTriangle,
  CheckCircle2,
} from "lucide-react";

// ── Constants ─────────────────────────────────────────────────────

const MODELS = [
  "catboost",
  "garch",
  "finbert",
  "chronos",
  "hrm",
  "ets",
  "skfolio",
  "consensus",
] as const;

const SYMBOLS = [
  "BTCUSDT",
  "ETHUSDT",
  "BNBUSDT",
  "SOLUSDT",
  "XRPUSDT",
  "ADAUSDT",
  "DOGEUSDT",
  "AVAXUSDT",
];

const MODES = ["Signal", "Semi-Auto", "Full-Auto"] as const;

// ── Page Component ────────────────────────────────────────────────

export default function StrategyBuilderPage() {
  const [name, setName] = useState("");
  const [selectedModels, setSelectedModels] = useState<string[]>(["consensus"]);
  const [selectedSymbols, setSelectedSymbols] = useState<string[]>(["BTCUSDT"]);
  const [mode, setMode] = useState<string>("Signal");
  const [maxPosition, setMaxPosition] = useState(5);
  const [maxDailyLoss, setMaxDailyLoss] = useState(2);
  const [maxDrawdown, setMaxDrawdown] = useState(10);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);

  const toggleModel = (m: string) =>
    setSelectedModels((prev) =>
      prev.includes(m) ? prev.filter((x) => x !== m) : [...prev, m],
    );

  const toggleSymbol = (s: string) =>
    setSelectedSymbols((prev) =>
      prev.includes(s) ? prev.filter((x) => x !== s) : [...prev, s],
    );

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    setSuccess(false);

    try {
      const res = await fetch("/api/strategies", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name,
          symbols: selectedSymbols,
          models: selectedModels,
          mode: mode.toLowerCase().replace("-", "_"),
          risk_limits: {
            max_position_pct: maxPosition,
            max_daily_loss_pct: maxDailyLoss,
            max_drawdown_pct: maxDrawdown,
          },
        }),
      });

      const data = await res.json();
      if (data.success) {
        setSuccess(true);
      } else {
        setError(data.error?.message || "Failed to save strategy");
      }
    } catch {
      setError("Failed to connect to API");
    } finally {
      setLoading(false);
    }
  };

  const inputClass =
    "w-full px-4 py-2.5 bg-gray-900/50 border border-gray-700 rounded-xl text-white placeholder-slate-500 focus:outline-none focus:border-cyan-500/50 focus:ring-1 focus:ring-cyan-500/30 transition-colors";

  return (
    <div className="min-h-screen text-slate-100">
      <div className="p-6 space-y-6">
        {/* Header */}
        <motion.div
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
        >
          <div className="flex items-center gap-3 mb-2">
            <div className="p-2 rounded-xl bg-purple-500/10 border border-purple-500/20">
              <Settings className="w-7 h-7 text-purple-400" />
            </div>
            <div>
              <h1 className="text-2xl font-bold text-white">
                Strategy Builder
              </h1>
              <p className="text-gray-400 text-sm">
                Configure a trading strategy with model selection, symbols, and
                risk limits
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
          <form onSubmit={handleSave} className="space-y-6">
            {/* Strategy Name */}
            <div>
              <label className="block text-sm font-medium text-slate-400 mb-1.5">
                Strategy Name
              </label>
              <input
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                className={inputClass}
                placeholder="e.g. BTC Momentum Alpha"
                required
              />
            </div>

            {/* Model Selection */}
            <div>
              <label className="flex items-center gap-2 text-sm font-medium text-slate-400 mb-2">
                <Brain className="w-4 h-4" /> Models
              </label>
              <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
                {MODELS.map((m) => (
                  <label
                    key={m}
                    className={`flex items-center gap-2 px-3 py-2.5 rounded-xl border cursor-pointer transition-colors ${selectedModels.includes(m) ? "bg-purple-500/15 border-purple-500/40 text-purple-300" : "bg-gray-900/30 border-gray-700 text-slate-400 hover:border-gray-600"}`}
                  >
                    <input
                      type="checkbox"
                      checked={selectedModels.includes(m)}
                      onChange={() => toggleModel(m)}
                      className="accent-purple-500"
                    />
                    <span className="text-sm capitalize">{m}</span>
                  </label>
                ))}
              </div>
            </div>

            {/* Symbol Selection */}
            <div>
              <label className="flex items-center gap-2 text-sm font-medium text-slate-400 mb-2">
                <Cpu className="w-4 h-4" /> Symbols
              </label>
              <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
                {SYMBOLS.map((s) => (
                  <label
                    key={s}
                    className={`flex items-center gap-2 px-3 py-2.5 rounded-xl border cursor-pointer transition-colors ${selectedSymbols.includes(s) ? "bg-cyan-500/15 border-cyan-500/40 text-cyan-300" : "bg-gray-900/30 border-gray-700 text-slate-400 hover:border-gray-600"}`}
                  >
                    <input
                      type="checkbox"
                      checked={selectedSymbols.includes(s)}
                      onChange={() => toggleSymbol(s)}
                      className="accent-cyan-500"
                    />
                    <span className="text-sm">{s}</span>
                  </label>
                ))}
              </div>
            </div>

            {/* Trading Mode */}
            <div>
              <label className="flex items-center gap-2 text-sm font-medium text-slate-400 mb-2">
                <Cpu className="w-4 h-4" /> Trading Mode
              </label>
              <div className="flex gap-3">
                {MODES.map((m) => (
                  <label
                    key={m}
                    className={`flex items-center gap-2 px-4 py-2.5 rounded-xl border cursor-pointer transition-colors ${mode === m ? "bg-cyan-500/15 border-cyan-500/40 text-cyan-300" : "bg-gray-900/30 border-gray-700 text-slate-400 hover:border-gray-600"}`}
                  >
                    <input
                      type="radio"
                      name="mode"
                      value={m}
                      checked={mode === m}
                      onChange={() => setMode(m)}
                      className="accent-cyan-500"
                    />
                    <span className="text-sm">{m}</span>
                  </label>
                ))}
              </div>
            </div>

            {/* Risk Limits */}
            <div>
              <label className="flex items-center gap-2 text-sm font-medium text-slate-400 mb-2">
                <Shield className="w-4 h-4" /> Risk Limits
              </label>
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
                <div>
                  <label className="block text-xs text-slate-500 mb-1">
                    Max Position %
                  </label>
                  <input
                    type="number"
                    value={maxPosition}
                    onChange={(e) => setMaxPosition(Number(e.target.value))}
                    className={inputClass}
                    min={1}
                    max={100}
                    required
                  />
                </div>
                <div>
                  <label className="block text-xs text-slate-500 mb-1">
                    Max Daily Loss %
                  </label>
                  <input
                    type="number"
                    value={maxDailyLoss}
                    onChange={(e) => setMaxDailyLoss(Number(e.target.value))}
                    className={inputClass}
                    min={0.1}
                    max={100}
                    step={0.1}
                    required
                  />
                </div>
                <div>
                  <label className="block text-xs text-slate-500 mb-1">
                    Max Drawdown %
                  </label>
                  <input
                    type="number"
                    value={maxDrawdown}
                    onChange={(e) => setMaxDrawdown(Number(e.target.value))}
                    className={inputClass}
                    min={1}
                    max={100}
                    required
                  />
                </div>
              </div>
            </div>

            {/* Save Button */}
            <div className="pt-2">
              <button
                type="submit"
                disabled={
                  loading ||
                  selectedModels.length === 0 ||
                  selectedSymbols.length === 0
                }
                className="flex items-center gap-2 px-6 py-2.5 bg-purple-600 hover:bg-purple-500 disabled:bg-gray-700 disabled:cursor-not-allowed text-white rounded-xl font-medium transition-colors duration-200 cursor-pointer"
              >
                {loading ? (
                  <>
                    <Loader2 className="w-4 h-4 animate-spin" /> Saving...
                  </>
                ) : (
                  <>
                    <Save className="w-4 h-4" /> Save Strategy
                  </>
                )}
              </button>
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

        {/* Success */}
        {success && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            className="bg-emerald-500/10 border border-emerald-500/30 rounded-xl p-4 flex items-center gap-3"
          >
            <CheckCircle2 className="w-5 h-5 text-emerald-400 shrink-0" />
            <span className="text-emerald-300 text-sm">
              Strategy saved successfully!
            </span>
          </motion.div>
        )}
      </div>
    </div>
  );
}
