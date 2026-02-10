"use client";

import { useState, useEffect, useRef, useCallback, useMemo } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  ComposedChart,
  Line,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  ReferenceLine,
  ReferenceArea,
  CartesianGrid,
  Cell,
} from "recharts";
import {
  TrendingUp,
  TrendingDown,
  CandlestickChart,
  LineChart as LineChartIcon,
  BarChart3,
  Settings,
  Crosshair,
  Pencil,
  Trash2,
  Maximize2,
  Minimize2,
  Clock,
  Target,
  AlertTriangle,
  Zap,
  Brain,
  Layers,
  Activity,
  ChevronDown,
  ChevronUp,
  GripHorizontal,
  Plus,
  X,
  MousePointer2,
  Move,
} from "lucide-react";
import type { TradingMode } from "./trading-mode";
import { TRADING_MODES } from "./trading-mode";

// ============================================
// TYPES
// ============================================

interface CandleData {
  timestamp: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
  sma20?: number;
  sma50?: number;
  ema12?: number;
  ema26?: number;
  bbUpper?: number;
  bbLower?: number;
  vwap?: number;
}

interface Drawing {
  id: string;
  type: "trendline" | "horizontal" | "fibonacci" | "rectangle";
  points: { x: number; y: number }[];
  color: string;
}

interface RiskLevels {
  entry: number;
  stopLoss: number;
  takeProfit: number;
  position: "long" | "short";
}

interface ChartIndicator {
  id: string;
  type: "sma" | "ema" | "bollinger" | "vwap" | "volume";
  enabled: boolean;
  period?: number;
  color: string;
}

// ============================================
// MOCK DATA GENERATOR
// ============================================

function generateCandleData(count: number = 100): CandleData[] {
  const data: CandleData[] = [];
  let price = 150;
  let timestamp = Date.now() - count * 60000 * 5; // 5-minute intervals

  for (let i = 0; i < count; i++) {
    const volatility = 0.02;
    const trend = Math.sin(i / 20) * 5;
    const change = (Math.random() - 0.5) * volatility * price + trend * 0.1;
    
    const open = price;
    const close = price + change;
    const high = Math.max(open, close) + Math.random() * volatility * price * 0.5;
    const low = Math.min(open, close) - Math.random() * volatility * price * 0.5;
    const volume = Math.floor(Math.random() * 1000000) + 500000;

    data.push({
      timestamp,
      open: Number(open.toFixed(2)),
      high: Number(high.toFixed(2)),
      low: Number(low.toFixed(2)),
      close: Number(close.toFixed(2)),
      volume,
    });

    price = close;
    timestamp += 60000 * 5; // 5 minutes
  }

  // Calculate indicators
  return calculateIndicators(data);
}

function calculateIndicators(data: CandleData[]): CandleData[] {
  return data.map((candle, index) => {
    const result = { ...candle };

    // SMA 20
    if (index >= 19) {
      const sum = data.slice(index - 19, index + 1).reduce((a, b) => a + b.close, 0);
      result.sma20 = Number((sum / 20).toFixed(2));
    }

    // SMA 50
    if (index >= 49) {
      const sum = data.slice(index - 49, index + 1).reduce((a, b) => a + b.close, 0);
      result.sma50 = Number((sum / 50).toFixed(2));
    }

    // EMA 12 & 26 (simplified)
    if (index >= 11) {
      const multiplier = 2 / (12 + 1);
      const prevEma = data[index - 1].ema12 || data[index - 1].close;
      result.ema12 = Number((candle.close * multiplier + prevEma * (1 - multiplier)).toFixed(2));
    }

    // Bollinger Bands (simplified - 20 period, 2 std dev)
    if (index >= 19) {
      const slice = data.slice(index - 19, index + 1);
      const mean = slice.reduce((a, b) => a + b.close, 0) / 20;
      const variance = slice.reduce((a, b) => a + Math.pow(b.close - mean, 2), 0) / 20;
      const stdDev = Math.sqrt(variance);
      result.bbUpper = Number((mean + 2 * stdDev).toFixed(2));
      result.bbLower = Number((mean - 2 * stdDev).toFixed(2));
    }

    // VWAP (simplified)
    if (index > 0) {
      let cumulativeTPV = 0;
      let cumulativeVolume = 0;
      for (let i = 0; i <= index; i++) {
        const tp = (data[i].high + data[i].low + data[i].close) / 3;
        cumulativeTPV += tp * data[i].volume;
        cumulativeVolume += data[i].volume;
      }
      result.vwap = Number((cumulativeTPV / cumulativeVolume).toFixed(2));
    }

    return result;
  });
}

// ============================================
// CUSTOM CANDLESTICK RENDERER
// ============================================

const CandlestickBar = (props: any) => {
  const { x, y, width, height, payload } = props;
  
  if (!payload) return null;
  
  const { open, high, low, close } = payload;
  const isUp = close >= open;
  const color = isUp ? "#10b981" : "#f43f5e";
  
  const candleHeight = Math.abs(height) || 1;
  const wickX = x + width / 2;
  
  // Calculate Y positions based on price scale
  const priceRange = payload.high - payload.low;
  const candleY = y;
  const candleBottom = y + candleHeight;
  
  // Wick positions
  const highY = candleY - ((payload.high - Math.max(open, close)) / priceRange) * candleHeight;
  const lowY = candleBottom + ((Math.min(open, close) - payload.low) / priceRange) * candleHeight;
  
  return (
    <g>
      {/* Upper Wick */}
      <line
        x1={wickX}
        y1={highY}
        x2={wickX}
        y2={candleY}
        stroke={color}
        strokeWidth={1}
      />
      {/* Lower Wick */}
      <line
        x1={wickX}
        y1={candleBottom}
        x2={wickX}
        y2={lowY}
        stroke={color}
        strokeWidth={1}
      />
      {/* Candle Body */}
      <rect
        x={x + 1}
        y={candleY}
        width={width - 2}
        height={candleHeight}
        fill={isUp ? color : color}
        stroke={color}
        strokeWidth={1}
        opacity={isUp ? 0.8 : 0.9}
      />
    </g>
  );
};

// ============================================
// TRADING CHART COMPONENT
// ============================================

interface TradingChartProps {
  symbol?: string;
  mode?: TradingMode;
  onModeChange?: (mode: TradingMode) => void;
  riskLevels?: RiskLevels;
  className?: string;
}

const TIMEFRAMES = [
  { label: "1m", value: "1m", minutes: 1 },
  { label: "5m", value: "5m", minutes: 5 },
  { label: "15m", value: "15m", minutes: 15 },
  { label: "1H", value: "1h", minutes: 60 },
  { label: "4H", value: "4h", minutes: 240 },
  { label: "1D", value: "1d", minutes: 1440 },
  { label: "1W", value: "1w", minutes: 10080 },
];

const CHART_TYPES = [
  { id: "candlestick", name: "Candlestick", icon: CandlestickChart },
  { id: "line", name: "Line", icon: LineChartIcon },
  { id: "area", name: "Area", icon: Activity },
  { id: "heikin", name: "Heikin Ashi", icon: BarChart3 },
];

export function TradingChart({ 
  symbol = "AAPL", 
  mode = "semi_auto",
  onModeChange,
  riskLevels,
  className = "" 
}: TradingChartProps) {
  const [data, setData] = useState<CandleData[]>([]);
  const [timeframe, setTimeframe] = useState("5m");
  const [chartType, setChartType] = useState<"candlestick" | "line" | "area" | "heikin">("candlestick");
  const [indicators, setIndicators] = useState<ChartIndicator[]>([
    { id: "volume", type: "volume", enabled: true, color: "#6b7280" },
    { id: "sma20", type: "sma", enabled: true, period: 20, color: "#3b82f6" },
    { id: "sma50", type: "sma", enabled: false, period: 50, color: "#f59e0b" },
    { id: "ema12", type: "ema", enabled: false, period: 12, color: "#8b5cf6" },
    { id: "bollinger", type: "bollinger", enabled: false, color: "#ec4899" },
    { id: "vwap", type: "vwap", enabled: false, color: "#10b981" },
  ]);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [showIndicators, setShowIndicators] = useState(false);
  const [showDrawings, setShowDrawings] = useState(false);
  const [activeTool, setActiveTool] = useState<"pointer" | "crosshair" | "line" | "horizontal">("pointer");
  const [hoveredData, setHoveredData] = useState<CandleData | null>(null);
  const [lastPrice, setLastPrice] = useState<number | null>(null);
  const [priceChange, setPriceChange] = useState<number>(0);
  
  // Initialize data
  useEffect(() => {
    const initialData = generateCandleData(150);
    setData(initialData);
    setLastPrice(initialData[initialData.length - 1].close);
  }, []);

  // Real-time updates simulation
  useEffect(() => {
    const interval = setInterval(() => {
      setData((prevData) => {
        if (prevData.length === 0) return prevData;
        
        const lastCandle = prevData[prevData.length - 1];
        const newPrice = lastCandle.close + (Math.random() - 0.5) * 0.5;
        
        const updatedCandle: CandleData = {
          ...lastCandle,
          close: Number(newPrice.toFixed(2)),
          high: Math.max(lastCandle.high, newPrice),
          low: Math.min(lastCandle.low, newPrice),
        };
        
        setLastPrice(newPrice);
        setPriceChange(((newPrice - prevData[prevData.length - 2]?.close) / prevData[prevData.length - 2]?.close) * 100);
        
        return [...prevData.slice(0, -1), updatedCandle];
      });
    }, 2000);

    return () => clearInterval(interval);
  }, []);

  const currentMode = TRADING_MODES[mode];

  const toggleIndicator = (id: string) => {
    setIndicators(indicators.map(ind => 
      ind.id === id ? { ...ind, enabled: !ind.enabled } : ind
    ));
  };

  const chartHeight = isFullscreen ? "h-[calc(100vh-200px)]" : "h-[500px]";

  return (
    <div className={`bg-[#0a0f1c] rounded-2xl border border-gray-800 overflow-hidden ${className}`}>
      {/* Header */}
      <div className="px-4 py-3 border-b border-gray-800 flex items-center justify-between flex-wrap gap-3">
        {/* Symbol & Price */}
        <div className="flex items-center gap-4">
          <div>
            <div className="flex items-center gap-2">
              <h2 className="text-xl font-bold text-white">{symbol}</h2>
              <span className="text-sm text-gray-500">NASDAQ</span>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-2xl font-bold text-white">${lastPrice?.toFixed(2)}</span>
              <span className={`text-sm font-medium ${priceChange >= 0 ? "text-emerald-400" : "text-rose-400"}`}>
                {priceChange >= 0 ? "+" : ""}{priceChange.toFixed(2)}%
              </span>
            </div>
          </div>

          {/* Trading Mode Badge */}
          <button
            onClick={() => onModeChange?.(mode === "manual" ? "semi_auto" : mode === "semi_auto" ? "fully_auto" : "manual")}
            className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm font-medium transition-colors
              ${currentMode.bgColor} ${currentMode.borderColor} border`}
          >
            <currentMode.icon className={`w-4 h-4 ${currentMode.color}`} />
            <span className={currentMode.color}>{currentMode.shortName}</span>
            <ChevronDown className={`w-3 h-3 ${currentMode.color}`} />
          </button>
        </div>

        {/* Controls */}
        <div className="flex items-center gap-2">
          {/* Timeframes */}
          <div className="flex items-center bg-gray-800/50 rounded-lg p-1">
            {TIMEFRAMES.map((tf) => (
              <button
                key={tf.value}
                onClick={() => setTimeframe(tf.value)}
                className={`px-3 py-1.5 text-sm font-medium rounded-md transition-colors
                  ${timeframe === tf.value 
                    ? "bg-gray-700 text-white" 
                    : "text-gray-400 hover:text-white"}`}
              >
                {tf.label}
              </button>
            ))}
          </div>

          {/* Chart Type */}
          <div className="flex items-center bg-gray-800/50 rounded-lg p-1">
            {CHART_TYPES.map((type) => (
              <button
                key={type.id}
                onClick={() => setChartType(type.id as any)}
                className={`p-2 rounded-md transition-colors
                  ${chartType === type.id 
                    ? "bg-gray-700 text-white" 
                    : "text-gray-400 hover:text-white"}`}
                title={type.name}
              >
                <type.icon className="w-4 h-4" />
              </button>
            ))}
          </div>

          {/* Indicators Toggle */}
          <button
            onClick={() => setShowIndicators(!showIndicators)}
            className={`p-2 rounded-lg transition-colors ${showIndicators ? "bg-blue-600 text-white" : "text-gray-400 hover:text-white"}`}
            title="Indicators"
          >
            <Layers className="w-5 h-5" />
          </button>

          {/* Drawing Tools */}
          <button
            onClick={() => setShowDrawings(!showDrawings)}
            className={`p-2 rounded-lg transition-colors ${showDrawings ? "bg-blue-600 text-white" : "text-gray-400 hover:text-white"}`}
            title="Drawing Tools"
          >
            <Pencil className="w-5 h-5" />
          </button>

          {/* Fullscreen */}
          <button
            onClick={() => setIsFullscreen(!isFullscreen)}
            className="p-2 text-gray-400 hover:text-white rounded-lg transition-colors"
            title="Fullscreen"
          >
            {isFullscreen ? <Minimize2 className="w-5 h-5" /> : <Maximize2 className="w-5 h-5" />}
          </button>
        </div>
      </div>

      {/* Indicators Panel */}
      <AnimatePresence>
        {showIndicators && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            className="border-b border-gray-800 overflow-hidden"
          >
            <div className="p-3 flex flex-wrap gap-2">
              {indicators.map((ind) => (
                <button
                  key={ind.id}
                  onClick={() => toggleIndicator(ind.id)}
                  className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm transition-colors
                    ${ind.enabled 
                      ? "bg-gray-700 text-white" 
                      : "bg-gray-800/50 text-gray-400 hover:text-white"}`}
                >
                  <div 
                    className="w-3 h-3 rounded-full" 
                    style={{ backgroundColor: ind.enabled ? ind.color : "#6b7280" }}
                  />
                  {ind.type.toUpperCase()}
                  {ind.period && `(${ind.period})`}
                </button>
              ))}
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Drawing Tools Panel */}
      <AnimatePresence>
        {showDrawings && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            className="border-b border-gray-800 overflow-hidden"
          >
            <div className="p-3 flex items-center gap-2">
              {[
                { id: "pointer", icon: MousePointer2, name: "Pointer" },
                { id: "crosshair", icon: Crosshair, name: "Crosshair" },
                { id: "line", icon: TrendingUp, name: "Trend Line" },
                { id: "horizontal", icon: GripHorizontal, name: "Horizontal" },
              ].map((tool) => (
                <button
                  key={tool.id}
                  onClick={() => setActiveTool(tool.id as any)}
                  className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm transition-colors
                    ${activeTool === tool.id 
                      ? "bg-blue-600 text-white" 
                      : "bg-gray-800/50 text-gray-400 hover:text-white"}`}
                >
                  <tool.icon className="w-4 h-4" />
                  {tool.name}
                </button>
              ))}
              <div className="flex-1" />
              <button className="p-2 text-rose-400 hover:bg-rose-500/10 rounded-lg transition-colors">
                <Trash2 className="w-4 h-4" />
              </button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Main Chart Area */}
      <div className={`relative ${chartHeight}`}>
        {/* Mode Overlay */}
        <div className="absolute top-4 left-4 z-10">
          <div className={`px-3 py-2 rounded-lg ${currentMode.bgColor} ${currentMode.borderColor} border backdrop-blur-sm`}>
            <div className="flex items-center gap-2">
              <currentMode.icon className={`w-4 h-4 ${currentMode.color}`} />
              <span className={`text-sm font-medium ${currentMode.color}`}>{currentMode.name}</span>
            </div>
            <p className="text-xs text-gray-400 mt-1">{currentMode.description}</p>
          </div>
        </div>

        {/* Quick Mode Switch */}
        <div className="absolute top-4 right-4 z-10 flex flex-col gap-2">
          {(Object.keys(TRADING_MODES) as TradingMode[]).map((m) => (
            <button
              key={m}
              onClick={() => onModeChange?.(m)}
              className={`w-10 h-10 rounded-lg flex items-center justify-center transition-all
                ${mode === m 
                  ? `${TRADING_MODES[m].bgColor} ${TRADING_MODES[m].color} ring-2 ring-offset-2 ring-offset-[#0a0f1c] ${TRADING_MODES[m].borderColor}` 
                  : "bg-gray-800/50 text-gray-400 hover:text-white"}`}
              title={TRADING_MODES[m].name}
            >
              {m === "manual" && <Brain className="w-5 h-5" />}
              {m === "semi_auto" && <Target className="w-5 h-5" />}
              {m === "fully_auto" && <Zap className="w-5 h-5" />}
            </button>
          ))}
        </div>

        {/* Risk Levels Panel */}
        {riskLevels && (
          <div className="absolute bottom-4 right-4 z-10">
            <div className="p-3 rounded-xl bg-gray-900/90 border border-gray-800 backdrop-blur-sm">
              <h4 className="text-xs font-medium text-gray-400 mb-2">Risk Levels</h4>
              <div className="space-y-1 text-xs">
                <div className="flex items-center justify-between gap-4">
                  <span className="text-gray-500">Entry</span>
                  <span className="text-blue-400 font-mono">${riskLevels.entry.toFixed(2)}</span>
                </div>
                <div className="flex items-center justify-between gap-4">
                  <span className="text-gray-500">Stop Loss</span>
                  <span className="text-rose-400 font-mono">${riskLevels.stopLoss.toFixed(2)}</span>
                </div>
                <div className="flex items-center justify-between gap-4">
                  <span className="text-gray-500">Take Profit</span>
                  <span className="text-emerald-400 font-mono">${riskLevels.takeProfit.toFixed(2)}</span>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Chart */}
        <ResponsiveContainer width="100%" height="100%">
          <ComposedChart
            data={data}
            onMouseMove={(e: any) => {
              if (e.activePayload) {
                setHoveredData(e.activePayload[0].payload);
              }
            }}
            onMouseLeave={() => setHoveredData(null)}
          >
            <CartesianGrid strokeDasharray="3 3" stroke="#1f2937" opacity={0.5} />
            <XAxis 
              dataKey="timestamp" 
              tickFormatter={(value) => {
                const date = new Date(value);
                return `${date.getHours()}:${date.getMinutes().toString().padStart(2, "0")}`;
              }}
              stroke="#374151"
              tick={{ fill: "#6b7280", fontSize: 11 }}
            />
            <YAxis 
              domain={["auto", "auto"]}
              stroke="#374151"
              tick={{ fill: "#6b7280", fontSize: 11 }}
              tickFormatter={(value) => `$${value}`}
              width={60}
            />
            <Tooltip
              content={({ active, payload }) => {
                if (!active || !payload || !payload.length) return null;
                const d = payload[0].payload as CandleData;
                const isUp = d.close >= d.open;
                return (
                  <div className="bg-gray-900 border border-gray-700 rounded-lg p-3 shadow-xl">
                    <p className="text-gray-400 text-xs mb-2">
                      {new Date(d.timestamp).toLocaleString()}
                    </p>
                    <div className="space-y-1 text-sm">
                      <div className="flex justify-between gap-4">
                        <span className="text-gray-500">Open</span>
                        <span className="text-white font-mono">${d.open.toFixed(2)}</span>
                      </div>
                      <div className="flex justify-between gap-4">
                        <span className="text-gray-500">High</span>
                        <span className="text-emerald-400 font-mono">${d.high.toFixed(2)}</span>
                      </div>
                      <div className="flex justify-between gap-4">
                        <span className="text-gray-500">Low</span>
                        <span className="text-rose-400 font-mono">${d.low.toFixed(2)}</span>
                      </div>
                      <div className="flex justify-between gap-4">
                        <span className="text-gray-500">Close</span>
                        <span className={`font-mono ${isUp ? "text-emerald-400" : "text-rose-400"}`}>
                          ${d.close.toFixed(2)}
                        </span>
                      </div>
                      <div className="flex justify-between gap-4 pt-1 border-t border-gray-800">
                        <span className="text-gray-500">Volume</span>
                        <span className="text-gray-300 font-mono">{(d.volume / 1000000).toFixed(2)}M</span>
                      </div>
                    </div>
                  </div>
                );
              }}
            />

            {/* Risk Level Lines */}
            {riskLevels && (
              <>
                <ReferenceLine 
                  y={riskLevels.entry} 
                  stroke="#3b82f6" 
                  strokeDasharray="5 5" 
                  label={{ value: "Entry", fill: "#3b82f6", position: "right" }}
                />
                <ReferenceLine 
                  y={riskLevels.stopLoss} 
                  stroke="#f43f5e" 
                  strokeDasharray="5 5"
                  label={{ value: "SL", fill: "#f43f5e", position: "right" }}
                />
                <ReferenceLine 
                  y={riskLevels.takeProfit} 
                  stroke="#10b981" 
                  strokeDasharray="5 5"
                  label={{ value: "TP", fill: "#10b981", position: "right" }}
                />
                <ReferenceArea 
                  y1={riskLevels.stopLoss} 
                  y2={riskLevels.entry} 
                  fill="#f43f5e" 
                  fillOpacity={0.05}
                />
                <ReferenceArea 
                  y1={riskLevels.entry} 
                  y2={riskLevels.takeProfit} 
                  fill="#10b981" 
                  fillOpacity={0.05}
                />
              </>
            )}

            {/* Volume Bars */}
            {indicators.find(i => i.id === "volume")?.enabled && (
              <Bar 
                dataKey="volume" 
                yAxisId="volume"
                fill="#6b7280" 
                opacity={0.3}
              >
                {data.map((entry, index) => (
                  <Cell 
                    key={`cell-${index}`} 
                    fill={entry.close >= entry.open ? "#10b981" : "#f43f5e"}
                    opacity={0.3}
                  />
                ))}
              </Bar>
            )}

            {/* Price Line (for line/area charts) */}
            {(chartType === "line" || chartType === "area") && (
              <Line 
                type="monotone" 
                dataKey="close" 
                stroke="#3b82f6" 
                strokeWidth={2}
                dot={false}
              />
            )}

            {/* SMA 20 */}
            {indicators.find(i => i.id === "sma20")?.enabled && (
              <Line 
                type="monotone" 
                dataKey="sma20" 
                stroke="#3b82f6" 
                strokeWidth={1.5}
                dot={false}
              />
            )}

            {/* SMA 50 */}
            {indicators.find(i => i.id === "sma50")?.enabled && (
              <Line 
                type="monotone" 
                dataKey="sma50" 
                stroke="#f59e0b" 
                strokeWidth={1.5}
                dot={false}
              />
            )}

            {/* EMA 12 */}
            {indicators.find(i => i.id === "ema12")?.enabled && (
              <Line 
                type="monotone" 
                dataKey="ema12" 
                stroke="#8b5cf6" 
                strokeWidth={1.5}
                dot={false}
              />
            )}

            {/* VWAP */}
            {indicators.find(i => i.id === "vwap")?.enabled && (
              <Line 
                type="monotone" 
                dataKey="vwap" 
                stroke="#10b981" 
                strokeWidth={1.5}
                strokeDasharray="5 5"
                dot={false}
              />
            )}
          </ComposedChart>
        </ResponsiveContainer>
      </div>

      {/* Footer Info */}
      <div className="px-4 py-2 border-t border-gray-800 flex items-center justify-between text-xs text-gray-500">
        <div className="flex items-center gap-4">
          {hoveredData && (
            <>
              <span>O: {hoveredData.open.toFixed(2)}</span>
              <span>H: {hoveredData.high.toFixed(2)}</span>
              <span>L: {hoveredData.low.toFixed(2)}</span>
              <span>C: {hoveredData.close.toFixed(2)}</span>
              <span>Vol: {(hoveredData.volume / 1000000).toFixed(2)}M</span>
            </>
          )}
        </div>
        <div className="flex items-center gap-4">
          <span>{data.length} candles</span>
          <span>Last update: {new Date().toLocaleTimeString()}</span>
        </div>
      </div>
    </div>
  );
}

export default TradingChart;
