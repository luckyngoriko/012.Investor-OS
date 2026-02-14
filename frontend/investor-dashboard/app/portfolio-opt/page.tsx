"use client";

import { useState } from "react";
import { motion } from "framer-motion";
import { 
  BarChart3, PieChart, TrendingUp, Calculator, Target,
  ChevronRight, Download, RefreshCw
} from "lucide-react";
import Sidebar from "@/components/sidebar";

const optimizationMethods = [
  { id: "markowitz", name: "Markowitz MPT", desc: "Mean-variance optimization", selected: true },
  { id: "black-litterman", name: "Black-Litterman", desc: "Bayesian approach with investor views", selected: false },
  { id: "risk-parity", name: "Risk Parity", desc: "Equal risk contribution", selected: false },
  { id: "max-diversification", name: "Max Diversification", desc: "Maximize diversification ratio", selected: false },
];

const efficientFrontier = [
  { risk: 0.10, return: 0.08, name: "Conservative" },
  { risk: 0.15, return: 0.12, name: "Moderate" },
  { risk: 0.20, return: 0.16, name: "Aggressive" },
  { risk: 0.25, return: 0.19, name: "Very Aggressive" },
];

const mockAllocation = [
  { symbol: "AAPL", weight: 25, expectedReturn: 0.15, risk: 0.22 },
  { symbol: "MSFT", weight: 20, expectedReturn: 0.14, risk: 0.20 },
  { symbol: "GOOGL", weight: 15, expectedReturn: 0.16, risk: 0.24 },
  { symbol: "AMZN", weight: 15, expectedReturn: 0.18, risk: 0.28 },
  { symbol: "NVDA", weight: 10, expectedReturn: 0.22, risk: 0.35 },
  { symbol: "CASH", weight: 15, expectedReturn: 0.04, risk: 0.01 },
];

export default function PortfolioOptimizationPage() {
  const [selectedMethod, setSelectedMethod] = useState("markowitz");
  const [objective, setObjective] = useState("sharpe");
  const [isOptimizing, setIsOptimizing] = useState(false);
  const [showResults, setShowResults] = useState(false);

  const handleOptimize = () => {
    setIsOptimizing(true);
    setTimeout(() => {
      setIsOptimizing(false);
      setShowResults(true);
    }, 1500);
  };

  const totalWeight = mockAllocation.reduce((sum, a) => sum + a.weight, 0);
  const portfolioReturn = mockAllocation.reduce((sum, a) => sum + (a.weight / 100) * a.expectedReturn, 0);
  const portfolioRisk = Math.sqrt(mockAllocation.reduce((sum, a) => sum + Math.pow((a.weight / 100) * a.risk, 2), 0));
  const sharpeRatio = (portfolioReturn - 0.04) / portfolioRisk;

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
                <p className="text-gray-400 text-sm">Sprint 32: Markowitz MPT, Black-Litterman, Risk Parity</p>
              </div>
            </div>
          </motion.div>

          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
            <div className="lg:col-span-2 space-y-6">
              <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="glass-card rounded-2xl p-6">
                <h3 className="text-lg font-semibold text-white mb-4">Optimization Method</h3>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                  {optimizationMethods.map((method) => (
                    <button
                      key={method.id}
                      onClick={() => setSelectedMethod(method.id)}
                      className={`p-4 rounded-xl text-left transition-all border
                        ${selectedMethod === method.id 
                          ? "bg-blue-600/20 border-blue-500/50" 
                          : "bg-gray-800/30 border-gray-700 hover:border-gray-600"}`}
                    >
                      <div className="flex items-center justify-between">
                        <span className={`font-medium ${selectedMethod === method.id ? "text-blue-400" : "text-white"}`}>
                          {method.name}
                        </span>
                        {selectedMethod === method.id && <div className="w-2 h-2 rounded-full bg-blue-400" />}
                      </div>
                      <p className="text-sm text-gray-400 mt-1">{method.desc}</p>
                    </button>
                  ))}
                </div>
              </motion.div>

              <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: 0.1 }} className="glass-card rounded-2xl p-6">
                <h3 className="text-lg font-semibold text-white mb-4">Optimization Objective</h3>
                <div className="flex flex-wrap gap-2">
                  {[
                    { id: "return", label: "Maximize Return", icon: TrendingUp },
                    { id: "risk", label: "Minimize Risk", icon: Target },
                    { id: "sharpe", label: "Maximize Sharpe", icon: BarChart3 },
                    { id: "parity", label: "Risk Parity", icon: PieChart },
                  ].map((obj) => (
                    <button
                      key={obj.id}
                      onClick={() => setObjective(obj.id)}
                      className={`flex items-center gap-2 px-4 py-2 rounded-lg transition-all
                        ${objective === obj.id 
                          ? "bg-emerald-600/20 text-emerald-400 border border-emerald-500/50" 
                          : "bg-gray-800 text-gray-400 border border-gray-700 hover:border-gray-600"}`}
                    >
                      <obj.icon className="w-4 h-4" />
                      <span className="text-sm font-medium">{obj.label}</span>
                    </button>
                  ))}
                </div>
              </motion.div>

              <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: 0.2 }} className="glass-card rounded-2xl p-6">
                <h3 className="text-lg font-semibold text-white mb-4">Efficient Frontier</h3>
                <div className="space-y-3">
                  {efficientFrontier.map((point) => (
                    <div key={point.name} className="flex items-center justify-between p-3 bg-gray-800/30 rounded-lg">
                      <div className="flex items-center gap-3">
                        <div className="w-3 h-3 rounded-full bg-blue-500" />
                        <span className="text-white font-medium">{point.name}</span>
                      </div>
                      <div className="flex gap-4 text-sm">
                        <span className="text-gray-400">Risk: <span className="text-amber-400">{(point.risk * 100).toFixed(0)}%</span></span>
                        <span className="text-gray-400">Return: <span className="text-emerald-400">{(point.return * 100).toFixed(0)}%</span></span>
                      </div>
                    </div>
                  ))}
                </div>
              </motion.div>
            </div>

            <div className="space-y-6">
              <motion.div initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }} className="glass-card rounded-2xl p-6">
                <button
                  onClick={handleOptimize}
                  disabled={isOptimizing}
                  className="w-full py-3 bg-blue-600 hover:bg-blue-500 disabled:bg-gray-700 text-white font-medium rounded-lg transition-colors flex items-center justify-center gap-2"
                >
                  {isOptimizing ? (
                    <><RefreshCw className="w-5 h-5 animate-spin" /> Optimizing...</>
                  ) : (
                    <><Calculator className="w-5 h-5" /> Run Optimization</>
                  )}
                </button>
              </motion.div>

              {showResults && (
                <motion.div initial={{ opacity: 0, scale: 0.95 }} animate={{ opacity: 1, scale: 1 }} className="glass-card rounded-2xl p-6">
                  <h3 className="text-lg font-semibold text-white mb-4">Optimized Portfolio</h3>
                  
                  <div className="grid grid-cols-2 gap-3 mb-4">
                    <div className="p-3 bg-gray-800/30 rounded-lg text-center">
                      <p className="text-xs text-gray-400">Expected Return</p>
                      <p className="text-lg font-bold text-emerald-400">{(portfolioReturn * 100).toFixed(1)}%</p>
                    </div>
                    <div className="p-3 bg-gray-800/30 rounded-lg text-center">
                      <p className="text-xs text-gray-400">Risk (Std Dev)</p>
                      <p className="text-lg font-bold text-amber-400">{(portfolioRisk * 100).toFixed(1)}%</p>
                    </div>
                    <div className="p-3 bg-gray-800/30 rounded-lg text-center">
                      <p className="text-xs text-gray-400">Sharpe Ratio</p>
                      <p className="text-lg font-bold text-blue-400">{sharpeRatio.toFixed(2)}</p>
                    </div>
                    <div className="p-3 bg-gray-800/30 rounded-lg text-center">
                      <p className="text-xs text-gray-400">Diversification</p>
                      <p className="text-lg font-bold text-purple-400">1.45</p>
                    </div>
                  </div>

                  <div className="space-y-2">
                    <p className="text-sm text-gray-400 mb-2">Asset Allocation</p>
                    {mockAllocation.map((asset) => (
                      <div key={asset.symbol} className="flex items-center gap-2">
                        <span className="text-sm text-white w-16">{asset.symbol}</span>
                        <div className="flex-1 h-2 bg-gray-700 rounded-full overflow-hidden">
                          <div className="h-full bg-blue-500 rounded-full" style={{ width: `${asset.weight}%` }} />
                        </div>
                        <span className="text-sm text-gray-400 w-12 text-right">{asset.weight}%</span>
                      </div>
                    ))}
                  </div>

                  <button className="w-full mt-4 py-2 bg-gray-800 hover:bg-gray-700 text-gray-300 text-sm font-medium rounded-lg transition-colors flex items-center justify-center gap-2">
                    <Download className="w-4 h-4" /> Export Report
                  </button>
                </motion.div>
              )}
            </div>
          </div>
        </div>
      </main>
    </div>
  );
}
