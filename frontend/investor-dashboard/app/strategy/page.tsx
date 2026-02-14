"use client";

import { useState } from "react";
import { motion } from "framer-motion";
import { 
  Brain, TrendingUp, Activity, BarChart3, Target,
  RefreshCw, CheckCircle2, AlertCircle, Zap
} from "lucide-react";
import Sidebar from "@/components/sidebar";

const marketRegimes = [
  { id: "trending", name: "Trending", confidence: 0.85, color: "emerald", trend: "Bullish" },
  { id: "ranging", name: "Ranging", confidence: 0.72, color: "blue", trend: "Neutral" },
  { id: "volatile", name: "Volatile", confidence: 0.68, color: "amber", trend: "Choppy" },
  { id: "breakout", name: "Breakout", confidence: 0.55, color: "purple", trend: "Strong" },
];

const strategies = [
  { id: "momentum", name: "Momentum", allocation: 40, performance: "+12.5%", fit: 0.92 },
  { id: "trend", name: "Trend Following", allocation: 35, performance: "+8.3%", fit: 0.88 },
  { id: "mean", name: "Mean Reversion", allocation: 15, performance: "+5.2%", fit: 0.65 },
  { id: "breakout", name: "Breakout", allocation: 10, performance: "+15.1%", fit: 0.78 },
];

const performanceAttribution = [
  { factor: "Market Timing", contribution: "+1.2%", impact: "positive" },
  { factor: "Security Selection", contribution: "+0.8%", impact: "positive" },
  { factor: "Risk Management", contribution: "+0.3%", impact: "positive" },
  { factor: "Transaction Costs", contribution: "-0.1%", impact: "negative" },
];

export default function StrategySelectorPage() {
  const [selectedRegime, setSelectedRegime] = useState("trending");
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [showRecommendation, setShowRecommendation] = useState(true);

  const handleAnalyze = () => {
    setIsAnalyzing(true);
    setTimeout(() => {
      setIsAnalyzing(false);
      setShowRecommendation(true);
    }, 1000);
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0f1c] via-[#111827] to-[#0a0f1c] flex">
      <Sidebar />
      <main className="flex-1 min-h-screen p-6 lg:p-8">
        <div className="max-w-7xl mx-auto space-y-6">
          <motion.div initial={{ opacity: 0, y: -20 }} animate={{ opacity: 1, y: 0 }}>
            <div className="flex items-center gap-3 mb-2">
              <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-purple-500/20 to-purple-600/10 flex items-center justify-center">
                <Brain className="w-5 h-5 text-purple-400" />
              </div>
              <div>
                <h1 className="text-2xl font-bold text-white">ML Strategy Selector</h1>
                <p className="text-gray-400 text-sm">Sprint 31: Regime detection and automatic strategy selection</p>
              </div>
            </div>
          </motion.div>

          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
            <div className="lg:col-span-2 space-y-6">
              <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="glass-card rounded-2xl p-6">
                <div className="flex items-center justify-between mb-4">
                  <h3 className="text-lg font-semibold text-white">Current Market Regime</h3>
                  <button 
                    onClick={handleAnalyze}
                    disabled={isAnalyzing}
                    className="flex items-center gap-2 px-3 py-1.5 bg-blue-600/20 text-blue-400 rounded-lg text-sm hover:bg-blue-600/30 transition-colors"
                  >
                    <RefreshCw className={`w-4 h-4 ${isAnalyzing ? "animate-spin" : ""}`} />
                    {isAnalyzing ? "Analyzing..." : "Refresh"}
                  </button>
                </div>

                <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
                  {marketRegimes.map((regime) => (
                    <button
                      key={regime.id}
                      onClick={() => setSelectedRegime(regime.id)}
                      className={`p-4 rounded-xl text-center transition-all border
                        ${selectedRegime === regime.id 
                          ? `bg-${regime.color}-600/20 border-${regime.color}-500/50` 
                          : "bg-gray-800/30 border-gray-700 hover:border-gray-600"}`}
                    >
                      <p className={`font-bold ${selectedRegime === regime.id ? `text-${regime.color}-400` : "text-white"}`}>
                        {regime.name}
                      </p>
                      <p className="text-xs text-gray-400 mt-1">{(regime.confidence * 100).toFixed(0)}% confidence</p>
                      <p className={`text-xs mt-1 ${regime.trend === "Bullish" ? "text-emerald-400" : regime.trend === "Bearish" ? "text-rose-400" : "text-amber-400"}`}>
                        {regime.trend}
                      </p>
                    </button>
                  ))}
                </div>

                <div className="mt-4 p-4 bg-gray-800/30 rounded-xl">
                  <div className="flex items-center gap-2 mb-2">
                    <Activity className="w-5 h-5 text-blue-400" />
                    <span className="text-white font-medium">Market Indicators</span>
                  </div>
                  <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                    <div>
                      <span className="text-gray-400">VIX:</span>
                      <span className="text-emerald-400 ml-2">14.2</span>
                    </div>
                    <div>
                      <span className="text-gray-400">Breadth:</span>
                      <span className="text-emerald-400 ml-2">75%</span>
                    </div>
                    <div>
                      <span className="text-gray-400">RSI:</span>
                      <span className="text-amber-400 ml-2">68</span>
                    </div>
                    <div>
                      <span className="text-gray-400">MACD:</span>
                      <span className="text-emerald-400 ml-2">Positive</span>
                    </div>
                  </div>
                </div>
              </motion.div>

              <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: 0.1 }} className="glass-card rounded-2xl p-6">
                <h3 className="text-lg font-semibold text-white mb-4">Strategy Recommendations</h3>
                <div className="space-y-3">
                  {strategies.map((strategy) => (
                    <div key={strategy.id} className="flex items-center justify-between p-4 bg-gray-800/30 rounded-xl">
                      <div className="flex items-center gap-4">
                        <div className="w-10 h-10 rounded-lg bg-blue-500/10 flex items-center justify-center">
                          <Zap className="w-5 h-5 text-blue-400" />
                        </div>
                        <div>
                          <p className="text-white font-medium">{strategy.name}</p>
                          <p className="text-xs text-gray-400">Regime fit: {(strategy.fit * 100).toFixed(0)}%</p>
                        </div>
                      </div>
                      <div className="flex items-center gap-4">
                        <div className="text-right">
                          <p className="text-emerald-400 font-bold">{strategy.performance}</p>
                          <p className="text-xs text-gray-400">YTD Return</p>
                        </div>
                        <div className="w-16 text-right">
                          <span className="text-white font-bold">{strategy.allocation}%</span>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              </motion.div>
            </div>

            <div className="space-y-6">
              <motion.div initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }} className="glass-card rounded-2xl p-6">
                <h3 className="text-lg font-semibold text-white mb-4">Selection Criteria</h3>
                <div className="space-y-3">
                  {[
                    { label: "Regime Fit", weight: 35, value: 92 },
                    { label: "Recent Performance", weight: 30, value: 78 },
                    { label: "Risk-Adjusted Return", weight: 25, value: 85 },
                    { label: "Diversification", weight: 10, value: 70 },
                  ].map((criteria) => (
                    <div key={criteria.label}>
                      <div className="flex justify-between text-sm mb-1">
                        <span className="text-gray-400">{criteria.label}</span>
                        <span className="text-gray-500">{criteria.weight}%</span>
                      </div>
                      <div className="h-2 bg-gray-700 rounded-full overflow-hidden">
                        <div className="h-full bg-blue-500 rounded-full" style={{ width: `${criteria.value}%` }} />
                      </div>
                    </div>
                  ))}
                </div>
              </motion.div>

              <motion.div initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }} transition={{ delay: 0.1 }} className="glass-card rounded-2xl p-6">
                <h3 className="text-lg font-semibold text-white mb-4">Performance Attribution</h3>
                <div className="space-y-2">
                  {performanceAttribution.map((item) => (
                    <div key={item.factor} className="flex items-center justify-between p-3 bg-gray-800/30 rounded-lg">
                      <span className="text-sm text-gray-300">{item.factor}</span>
                      <span className={`text-sm font-medium ${item.impact === "positive" ? "text-emerald-400" : "text-rose-400"}`}>
                        {item.contribution}
                      </span>
                    </div>
                  ))}
                </div>
                <div className="mt-3 p-3 bg-emerald-500/10 rounded-lg">
                  <div className="flex items-center justify-between">
                    <span className="text-white font-medium">Total Alpha</span>
                    <span className="text-emerald-400 font-bold">+2.3%</span>
                  </div>
                </div>
              </motion.div>

              <motion.div initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }} transition={{ delay: 0.2 }} className="glass-card rounded-2xl p-6">
                <h3 className="text-lg font-semibold text-white mb-4">Switch Limits</h3>
                <div className="space-y-2 text-sm">
                  <div className="flex justify-between">
                    <span className="text-gray-400">Min Hold Period</span>
                    <span className="text-white">24 hours</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Min Improvement</span>
                    <span className="text-white">5%</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Max Switches/Day</span>
                    <span className="text-white">3</span>
                  </div>
                </div>
              </motion.div>
            </div>
          </div>
        </div>
      </main>
    </div>
  );
}
