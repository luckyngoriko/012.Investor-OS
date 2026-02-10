"use client";

import { useState } from "react";
import { motion } from "framer-motion";
import { 
  ArrowLeft, 
  Settings, 
  Maximize2, 
  Target, 
  TrendingUp, 
  TrendingDown,
  Shield,
  Zap,
  Brain,
  Info,
} from "lucide-react";
import Link from "next/link";
import { TradingChart } from "@/components/trading-chart";
import { TradingMode, TRADING_MODES } from "@/components/trading-mode";

// ============================================
// RISK CALCULATOR COMPONENT
// ============================================

function RiskCalculator({ 
  entry, 
  stopLoss, 
  takeProfit,
  onChange 
}: { 
  entry: number;
  stopLoss: number;
  takeProfit: number;
  onChange: (values: { entry: number; stopLoss: number; takeProfit: number; position: "long" | "short" }) => void;
}) {
  const risk = Math.abs(entry - stopLoss);
  const reward = Math.abs(takeProfit - entry);
  const riskReward = reward / risk;
  const position = entry > stopLoss ? "long" : "short";

  return (
    <div className="p-4 rounded-xl glass-card">
      <h3 className="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4 flex items-center gap-2">
        <Target className="w-4 h-4" />
        Risk Calculator
      </h3>
      
      <div className="space-y-4">
        <div>
          <label className="block text-xs text-gray-500 mb-1">Entry Price</label>
          <input
            type="number"
            value={entry}
            onChange={(e) => onChange({ entry: Number(e.target.value), stopLoss, takeProfit, position })}
            className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded-lg text-white text-sm"
            step="0.01"
          />
        </div>
        
        <div>
          <label className="block text-xs text-gray-500 mb-1">Stop Loss</label>
          <input
            type="number"
            value={stopLoss}
            onChange={(e) => onChange({ entry, stopLoss: Number(e.target.value), takeProfit, position })}
            className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded-lg text-white text-sm"
            step="0.01"
          />
        </div>
        
        <div>
          <label className="block text-xs text-gray-500 mb-1">Take Profit</label>
          <input
            type="number"
            value={takeProfit}
            onChange={(e) => onChange({ entry, stopLoss, takeProfit: Number(e.target.value), position })}
            className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded-lg text-white text-sm"
            step="0.01"
          />
        </div>

        <div className="pt-4 border-t border-gray-800">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-gray-400">Position</span>
            <span className={`text-sm font-medium ${position === "long" ? "text-emerald-400" : "text-rose-400"}`}>
              {position === "long" ? "LONG" : "SHORT"}
            </span>
          </div>
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-gray-400">Risk</span>
            <span className="text-sm text-rose-400 font-mono">${risk.toFixed(2)}</span>
          </div>
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-gray-400">Reward</span>
            <span className="text-sm text-emerald-400 font-mono">${reward.toFixed(2)}</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-sm text-gray-400">R:R Ratio</span>
            <span className={`text-lg font-bold ${riskReward >= 2 ? "text-emerald-400" : riskReward >= 1 ? "text-amber-400" : "text-rose-400"}`}>
              1:{riskReward.toFixed(1)}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}

// ============================================
// ORDER PANEL COMPONENT
// ============================================

function OrderPanel({ 
  mode,
  currentPrice 
}: { 
  mode: TradingMode;
  currentPrice: number;
}) {
  const [orderType, setOrderType] = useState<"market" | "limit" | "stop">("market");
  const [side, setSide] = useState<"buy" | "sell">("buy");
  const [quantity, setQuantity] = useState(100);

  const modeConfig = TRADING_MODES[mode];

  return (
    <div className="p-4 rounded-xl glass-card">
      <h3 className="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4">
        Place Order
      </h3>

      {/* Mode Indicator */}
      <div className={`mb-4 p-3 rounded-lg ${modeConfig.bgColor} ${modeConfig.borderColor} border`}>
        <div className="flex items-center gap-2">
          <modeConfig.icon className={`w-4 h-4 ${modeConfig.color}`} />
          <span className={`text-sm font-medium ${modeConfig.color}`}>{modeConfig.name}</span>
        </div>
        <p className="text-xs text-gray-500 mt-1">
          {mode === "manual" 
            ? "You will manually execute this trade"
            : mode === "semi_auto"
            ? "AI will execute after your confirmation"
            : "AI will auto-execute if CQ >= threshold"}
        </p>
      </div>

      {/* Side Selection */}
      <div className="grid grid-cols-2 gap-2 mb-4">
        <button
          onClick={() => setSide("buy")}
          className={`py-3 rounded-lg font-medium transition-colors
            ${side === "buy" 
              ? "bg-emerald-600 text-white" 
              : "bg-gray-800 text-gray-400 hover:text-white"}`}
        >
          Buy
        </button>
        <button
          onClick={() => setSide("sell")}
          className={`py-3 rounded-lg font-medium transition-colors
            ${side === "sell" 
              ? "bg-rose-600 text-white" 
              : "bg-gray-800 text-gray-400 hover:text-white"}`}
        >
          Sell
        </button>
      </div>

      {/* Order Type */}
      <div className="flex gap-2 mb-4">
        {["market", "limit", "stop"].map((type) => (
          <button
            key={type}
            onClick={() => setOrderType(type as any)}
            className={`flex-1 py-2 text-sm font-medium rounded-lg transition-colors capitalize
              ${orderType === type 
                ? "bg-blue-600 text-white" 
                : "bg-gray-800 text-gray-400 hover:text-white"}`}
          >
            {type}
          </button>
        ))}
      </div>

      {/* Quantity */}
      <div className="mb-4">
        <label className="block text-xs text-gray-500 mb-1">Quantity</label>
        <input
          type="number"
          value={quantity}
          onChange={(e) => setQuantity(Number(e.target.value))}
          className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded-lg text-white"
        />
      </div>

      {/* Price */}
      {orderType !== "market" && (
        <div className="mb-4">
          <label className="block text-xs text-gray-500 mb-1">Price</label>
          <input
            type="number"
            defaultValue={currentPrice}
            className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded-lg text-white"
            step="0.01"
          />
        </div>
      )}

      {/* Summary */}
      <div className="p-3 bg-gray-800/50 rounded-lg mb-4 space-y-2 text-sm">
        <div className="flex justify-between">
          <span className="text-gray-500">Est. Price</span>
          <span className="text-white">${currentPrice.toFixed(2)}</span>
        </div>
        <div className="flex justify-between">
          <span className="text-gray-500">Est. Total</span>
          <span className="text-white">${(quantity * currentPrice).toFixed(2)}</span>
        </div>
      </div>

      {/* Submit Button */}
      <button
        className={`w-full py-3 rounded-lg font-medium transition-colors
          ${side === "buy" 
            ? "bg-emerald-600 hover:bg-emerald-500" 
            : "bg-rose-600 hover:bg-rose-500"} text-white`}
      >
        {side === "buy" ? "Buy" : "Sell"} {quantity} Shares
      </button>
    </div>
  );
}

// ============================================
// MAIN CHART PAGE
// ============================================

export default function ChartPage() {
  const [mode, setMode] = useState<TradingMode>("semi_auto");
  const [riskLevels, setRiskLevels] = useState({
    entry: 155.50,
    stopLoss: 152.00,
    takeProfit: 162.00,
    position: "long" as "long" | "short",
  });

  const currentPrice = 155.50;

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0f1c] via-[#111827] to-[#0a0f1c]">
      {/* Header */}
      <header className="border-b border-gray-800 bg-gray-900/50 backdrop-blur-lg">
        <div className="max-w-7xl mx-auto px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <Link 
                href="/"
                className="p-2 text-gray-400 hover:text-white hover:bg-gray-800 rounded-lg transition-colors"
              >
                <ArrowLeft className="w-5 h-5" />
              </Link>
              <div>
                <h1 className="text-xl font-bold text-white">Advanced Chart</h1>
                <p className="text-sm text-gray-400">Professional trading interface with real-time data</p>
              </div>
            </div>
            <div className="flex items-center gap-3">
              <Link
                href="/admin"
                className="flex items-center gap-2 px-4 py-2 bg-gray-800 hover:bg-gray-700 text-white 
                  rounded-lg transition-colors"
              >
                <Settings className="w-4 h-4" />
                Settings
              </Link>
            </div>
          </div>
        </div>
      </header>

      <div className="max-w-7xl mx-auto px-6 py-6">
        <div className="grid grid-cols-1 lg:grid-cols-4 gap-6">
          {/* Main Chart Area */}
          <div className="lg:col-span-3">
            <TradingChart
              symbol="AAPL"
              mode={mode}
              onModeChange={setMode}
              riskLevels={riskLevels}
              className="h-[600px]"
            />

            {/* Info Cards */}
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mt-6">
              <div className="p-4 rounded-xl glass-card">
                <div className="flex items-center gap-3 mb-2">
                  <div className="w-10 h-10 rounded-lg bg-blue-500/20 flex items-center justify-center">
                    <Brain className="w-5 h-5 text-blue-400" />
                  </div>
                  <h3 className="font-medium text-white">AI Analysis</h3>
                </div>
                <p className="text-sm text-gray-400">
                  Current CQ Score: <span className="text-emerald-400 font-bold">87%</span>
                </p>
                <p className="text-xs text-gray-500 mt-1">
                  Strong buy signal detected based on PEGY and technical factors
                </p>
              </div>

              <div className="p-4 rounded-xl glass-card">
                <div className="flex items-center gap-3 mb-2">
                  <div className="w-10 h-10 rounded-lg bg-emerald-500/20 flex items-center justify-center">
                    <TrendingUp className="w-5 h-5 text-emerald-400" />
                  </div>
                  <h3 className="font-medium text-white">Trend</h3>
                </div>
                <p className="text-sm text-gray-400">
                  Direction: <span className="text-emerald-400 font-bold">Bullish</span>
                </p>
                <p className="text-xs text-gray-500 mt-1">
                  Price above SMA 20 & 50. Momentum increasing
                </p>
              </div>

              <div className="p-4 rounded-xl glass-card">
                <div className="flex items-center gap-3 mb-2">
                  <div className="w-10 h-10 rounded-lg bg-amber-500/20 flex items-center justify-center">
                    <Shield className="w-5 h-5 text-amber-400" />
                  </div>
                  <h3 className="font-medium text-white">Risk Status</h3>
                </div>
                <p className="text-sm text-gray-400">
                  Level: <span className="text-amber-400 font-bold">Medium</span>
                </p>
                <p className="text-xs text-gray-500 mt-1">
                  Within normal parameters. VaR: 1.2%
                </p>
              </div>
            </div>
          </div>

          {/* Sidebar */}
          <div className="lg:col-span-1 space-y-4">
            {/* Mode Quick Switch */}
            <div className="p-4 rounded-xl glass-card">
              <h3 className="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3">
                Trading Mode
              </h3>
              <div className="space-y-2">
                {(Object.keys(TRADING_MODES) as TradingMode[]).map((m) => {
                  const ModeIcon = TRADING_MODES[m].icon;
                  return (
                  <button
                    key={m}
                    onClick={() => setMode(m)}
                    className={`w-full flex items-center gap-3 p-3 rounded-lg transition-all
                      ${mode === m 
                        ? `${TRADING_MODES[m].bgColor} ${TRADING_MODES[m].borderColor} border` 
                        : "bg-gray-800/30 hover:bg-gray-800/50"}`}
                  >
                    <ModeIcon className={`w-5 h-5 ${TRADING_MODES[m].color}`} />
                    <div className="text-left">
                      <p className={`font-medium ${mode === m ? "text-white" : "text-gray-300"}`}>
                        {TRADING_MODES[m].shortName}
                      </p>
                      <p className="text-xs text-gray-500">{TRADING_MODES[m].description}</p>
                    </div>
                    {mode === m && (
                      <div className={`ml-auto w-2 h-2 rounded-full ${TRADING_MODES[m].color.replace("text-", "bg-")}`} />
                    )}
                  </button>
                  );
                })}
              </div>
            </div>

            {/* Risk Calculator */}
            <RiskCalculator
              entry={riskLevels.entry}
              stopLoss={riskLevels.stopLoss}
              takeProfit={riskLevels.takeProfit}
              onChange={setRiskLevels}
            />

            {/* Order Panel */}
            <OrderPanel mode={mode} currentPrice={currentPrice} />

            {/* Keyboard Shortcuts */}
            <div className="p-4 rounded-xl glass-card">
              <h3 className="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3 flex items-center gap-2">
                <Info className="w-4 h-4" />
                Shortcuts
              </h3>
              <div className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <span className="text-gray-500">Buy Market</span>
                  <kbd className="px-2 py-1 bg-gray-800 rounded text-xs">B</kbd>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-500">Sell Market</span>
                  <kbd className="px-2 py-1 bg-gray-800 rounded text-xs">S</kbd>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-500">Crosshair</span>
                  <kbd className="px-2 py-1 bg-gray-800 rounded text-xs">C</kbd>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-500">Fullscreen</span>
                  <kbd className="px-2 py-1 bg-gray-800 rounded text-xs">F</kbd>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
