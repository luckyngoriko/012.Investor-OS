"use client";

import { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { 
  TrendingUp, 
  TrendingDown, 
  Activity, 
  DollarSign, 
  Target, 
  BarChart3,
  PieChart,
  Clock,
  RefreshCw,
  CheckCircle2,
  XCircle,
  AlertTriangle,
  ArrowRight,
  Sparkles,
  Zap,
  Shield,
  Brain,
  ChevronRight,
  Info,
  Play,
  HelpCircle,
  FileText,
  Settings,
  Bot,
  User,
  UserCog,
} from "lucide-react";
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  PieChart as RePieChart,
  Pie,
  Cell,
  BarChart,
  Bar,
} from "recharts";
import Sidebar from "@/components/sidebar";
import { FeatureCard, QuickActionsPanel, WhatYouCanDo, SystemStatusOverview } from "@/components/features";
import { FeatureTour, TourTriggerButton, FirstTimeWelcome } from "@/components/feature-tour";
import { features } from "@/components/features";
import { 
  TradingModeIndicator, 
  TradingModeSelector,
  TradingModeWizard,
  ModeStatusBar,
  TRADING_MODES,
  type TradingMode,
  DEFAULT_MODE_CONFIG,
  type TradingModeConfig,
} from "@/components/trading-mode";
import { TradingFlowDiagram } from "@/components/trading-flow";
import { TradingChart } from "@/components/trading-chart";
import { useAuth } from "@/lib/auth-context";
import { useRouter } from "next/navigation";

// ============================================
// DASHBOARD DATA
// ============================================

const portfolioData = [
  { date: "Jan 1", value: 100000 },
  { date: "Jan 5", value: 102500 },
  { date: "Jan 10", value: 101800 },
  { date: "Jan 15", value: 105200 },
  { date: "Jan 20", value: 104000 },
  { date: "Jan 25", value: 108500 },
  { date: "Jan 30", value: 112342 },
];

const sectorData = [
  { name: "Technology", value: 35, color: "#3b82f6" },
  { name: "Healthcare", value: 25, color: "#10b981" },
  { name: "Finance", value: 20, color: "#8b5cf6" },
  { name: "Energy", value: 15, color: "#f59e0b" },
  { name: "Other", value: 5, color: "#6b7280" },
];

const initialPositions = [
  { symbol: "AAPL", name: "Apple Inc.", qty: 150, avgPrice: 175.50, currentPrice: 185.32, sector: "Technology", beta: 1.2 },
  { symbol: "MSFT", name: "Microsoft Corp.", qty: 100, avgPrice: 380.25, currentPrice: 412.88, sector: "Technology", beta: 0.9 },
  { symbol: "NVDA", name: "NVIDIA Corp.", qty: 50, avgPrice: 480.00, currentPrice: 725.50, sector: "Technology", beta: 1.8 },
  { symbol: "JPM", name: "JPMorgan Chase", qty: 75, avgPrice: 165.80, currentPrice: 172.45, sector: "Finance", beta: 1.1 },
  { symbol: "JNJ", name: "Johnson & Johnson", qty: 80, avgPrice: 155.20, currentPrice: 158.90, sector: "Healthcare", beta: 0.7 },
];

const tradeProposals = [
  { 
    symbol: "TSLA", 
    action: "BUY", 
    qty: 25, 
    price: 245.50, 
    cqScore: 87, 
    rationale: "Strong momentum, insider buying, positive sentiment",
    factors: { pegy: 0.85, insider: 0.92, sentiment: 0.88, technical: 0.82 },
    regime: "Risk On",
  },
  { 
    symbol: "AMD", 
    action: "BUY", 
    qty: 100, 
    price: 178.25, 
    cqScore: 82, 
    rationale: "Undervalued vs peers, AI chip demand, buyback support",
    factors: { pegy: 0.88, insider: 0.75, sentiment: 0.85, technical: 0.79 },
    regime: "Risk On",
  },
  { 
    symbol: "META", 
    action: "SELL", 
    qty: 30, 
    price: 485.00, 
    cqScore: 78, 
    rationale: "Profit taking at resistance, reduce concentration",
    factors: { pegy: 0.72, insider: 0.65, sentiment: 0.80, technical: 0.91 },
    regime: "Uncertain",
  },
];

const marketRegime = {
  regime: "Risk On",
  confidence: 0.82,
  vix: 14.2,
  breadth: 0.75,
  trend: "Bullish",
  timestamp: new Date().toISOString(),
};

// ============================================
// HELPER COMPONENTS
// ============================================

const StatCard = ({ title, value, change, changeType, icon: Icon, subtitle }: any) => (
  <motion.div
    initial={{ opacity: 0, y: 20 }}
    animate={{ opacity: 1, y: 0 }}
    className="glass-card rounded-2xl p-6 hover-glow"
  >
    <div className="flex items-start justify-between mb-4">
      <div className={`p-3 rounded-xl ${changeType === "positive" ? "bg-emerald-500/10" : changeType === "negative" ? "bg-rose-500/10" : "bg-blue-500/10"}`}>
        <Icon className={`w-6 h-6 ${changeType === "positive" ? "text-emerald-400" : changeType === "negative" ? "text-rose-400" : "text-blue-400"}`} />
      </div>
      {change && (
        <div className={`flex items-center gap-1 text-sm font-medium ${changeType === "positive" ? "text-emerald-400" : "text-rose-400"}`}>
          {changeType === "positive" ? <TrendingUp className="w-4 h-4" /> : <TrendingDown className="w-4 h-4" />}
          {change}
        </div>
      )}
    </div>
    <p className="text-gray-400 text-sm mb-1">{title}</p>
    <p className="text-2xl font-bold text-white">{value}</p>
    {subtitle && <p className="text-xs text-gray-500 mt-2">{subtitle}</p>}
  </motion.div>
);

const ProposalCard = ({ proposal, onConfirm, onReject }: any) => {
  const isBuy = proposal.action === "BUY";
  const totalValue = proposal.qty * proposal.price;
  
  return (
    <motion.div
      layout
      initial={{ opacity: 0, scale: 0.95 }}
      animate={{ opacity: 1, scale: 1 }}
      exit={{ opacity: 0, scale: 0.9 }}
      className="p-4 rounded-xl bg-gray-800/30 border border-gray-700/50 hover:border-gray-600/50 transition-all"
    >
      <div className="flex items-start justify-between mb-3">
        <div>
          <div className="flex items-center gap-2">
            <span className="text-lg font-bold text-white">{proposal.symbol}</span>
            <span className={`text-xs font-medium px-2 py-0.5 rounded-full ${isBuy ? "bg-emerald-500/20 text-emerald-400" : "bg-rose-500/20 text-rose-400"}`}>
              {proposal.action}
            </span>
          </div>
          <p className="text-sm text-gray-400">{proposal.qty} shares @ ${proposal.price.toFixed(2)}</p>
        </div>
        <div className="text-right">
          <div className="flex items-center gap-1">
            <Target className="w-4 h-4 text-blue-400" />
            <span className="text-lg font-bold text-blue-400">{proposal.cqScore}%</span>
          </div>
          <p className="text-xs text-gray-500">CQ Score</p>
        </div>
      </div>
      
      <p className="text-sm text-gray-300 mb-3">{proposal.rationale}</p>
      
      <div className="flex gap-2 mb-3">
        {Object.entries(proposal.factors).map(([key, value]) => (
          <div key={key} className="flex-1 text-center p-2 rounded-lg bg-gray-900/50">
            <p className="text-xs text-gray-500 uppercase">{key}</p>
            <p className={`text-sm font-medium ${(value as number) >= 0.8 ? "text-emerald-400" : (value as number) >= 0.6 ? "text-amber-400" : "text-gray-400"}`}>
              {Math.round((value as number) * 100)}%
            </p>
          </div>
        ))}
      </div>
      
      <div className="flex gap-2">
        <button
          onClick={onConfirm}
          className="flex-1 py-2 bg-emerald-600 hover:bg-emerald-500 text-white text-sm font-medium 
            rounded-lg transition-colors flex items-center justify-center gap-2"
        >
          <CheckCircle2 className="w-4 h-4" />
          Confirm
        </button>
        <button
          onClick={onReject}
          className="flex-1 py-2 bg-gray-700 hover:bg-gray-600 text-gray-300 text-sm font-medium 
            rounded-lg transition-colors flex items-center justify-center gap-2"
        >
          <XCircle className="w-4 h-4" />
          Reject
        </button>
      </div>
    </motion.div>
  );
};

// ============================================
// MAIN DASHBOARD PAGE
// ============================================

export default function DashboardPage() {
  const router = useRouter();
  const { user, isAuthenticated, isLoading } = useAuth();
  const [positions, setPositions] = useState(initialPositions);
  const [proposals, setProposals] = useState(tradeProposals);
  const [lastUpdate, setLastUpdate] = useState(new Date());
  const [showTour, setShowTour] = useState(false);
  const [selectedFeature, setSelectedFeature] = useState<typeof features[0] | null>(null);
  
  // Trading Mode State
  const [tradingMode, setTradingMode] = useState<TradingMode>("semi_auto");
  const [modeConfig, setModeConfig] = useState<TradingModeConfig>(DEFAULT_MODE_CONFIG);
  const [showModeWizard, setShowModeWizard] = useState(false);
  const [showModeSelector, setShowModeSelector] = useState(false);

  // Redirect to login if not authenticated
  useEffect(() => {
    if (!isLoading && !isAuthenticated) {
      router.push("/login");
    }
  }, [isLoading, isAuthenticated, router]);

  // Show loading while checking auth
  if (isLoading || !isAuthenticated) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-[#0a0f1c] via-[#111827] to-[#0a0f1c] flex items-center justify-center">
        <div className="w-8 h-8 border-2 border-blue-500/30 border-t-blue-500 rounded-full animate-spin" />
      </div>
    );
  }

  // Risk Levels for Chart
  const [riskLevels, setRiskLevels] = useState({
    entry: 155.50,
    stopLoss: 152.00,
    takeProfit: 162.00,
    position: "long" as "long" | "short",
  });

  // Simulate real-time price updates
  useEffect(() => {
    const interval = setInterval(() => {
      setPositions(prev => prev.map(pos => ({
        ...pos,
        currentPrice: pos.currentPrice * (1 + (Math.random() - 0.5) * 0.002)
      })));
      setLastUpdate(new Date());
    }, 5000);
    return () => clearInterval(interval);
  }, []);

  const handleConfirmProposal = (index: number) => {
    const proposal = proposals[index];
    // In a real app, this would call the backend API
    alert(`Confirmed: ${proposal.action} ${proposal.qty} ${proposal.symbol} @ $${proposal.price}`);
    setProposals(prev => prev.filter((_, i) => i !== index));
  };

  const handleRejectProposal = (index: number) => {
    setProposals(prev => prev.filter((_, i) => i !== index));
  };

  const totalValue = positions.reduce((sum, pos) => sum + (pos.qty * pos.currentPrice), 0);
  const totalCost = positions.reduce((sum, pos) => sum + (pos.qty * pos.avgPrice), 0);
  const totalPnL = totalValue - totalCost;
  const totalPnLPercent = (totalPnL / totalCost) * 100;

  // Mode-specific helper texts
  const getModeActionText = () => {
    switch (tradingMode) {
      case "manual":
        return "AI provides analysis. You execute all trades manually.";
      case "semi_auto":
        return `${proposals.length} AI proposals waiting for your confirmation.`;
      case "fully_auto":
        return "AI is actively trading within your risk limits.";
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0f1c] via-[#111827] to-[#0a0f1c]">
      <Sidebar />
      
      {/* First Time Welcome Modal */}
      <FirstTimeWelcome />
      
      {/* Trading Mode Wizard */}
      <TradingModeWizard
        isOpen={showModeWizard}
        onClose={() => setShowModeWizard(false)}
        onComplete={(config) => {
          setTradingMode(config.mode);
          setModeConfig(config);
          setShowModeWizard(false);
        }}
      />
      
      {/* Mode Selector Dropdown Modal */}
      <AnimatePresence>
        {showModeSelector && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm"
            onClick={() => setShowModeSelector(false)}
          >
            <motion.div
              initial={{ opacity: 0, scale: 0.95 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.95 }}
              onClick={(e) => e.stopPropagation()}
              className="w-full max-w-lg glass-card rounded-2xl p-6"
            >
              <div className="flex items-center justify-between mb-6">
                <h3 className="text-xl font-bold text-white">Trading Mode</h3>
                <button 
                  onClick={() => setShowModeSelector(false)}
                  className="p-2 hover:bg-gray-800 rounded-lg transition-colors"
                >
                  <XCircle className="w-5 h-5 text-gray-400" />
                </button>
              </div>
              <TradingModeSelector
                currentMode={tradingMode}
                onModeChange={(mode) => {
                  setTradingMode(mode);
                  setModeConfig({ ...modeConfig, mode });
                  setShowModeSelector(false);
                }}
              />
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
      
      {/* Feature Tour */}
      <FeatureTour isOpen={showTour} onClose={() => setShowTour(false)} />
      
      {/* Feature Detail Modal */}
      <AnimatePresence>
        {selectedFeature && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm"
            onClick={() => setSelectedFeature(null)}
          >
            <motion.div
              initial={{ opacity: 0, scale: 0.9 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.9 }}
              onClick={(e) => e.stopPropagation()}
              className="relative w-full max-w-2xl glass-card rounded-2xl p-8"
            >
              <button
                onClick={() => setSelectedFeature(null)}
                className="absolute top-4 right-4 p-2 text-gray-500 hover:text-white rounded-lg hover:bg-gray-800 transition-colors"
              >
                <XCircle className="w-6 h-6" />
              </button>
              
              <div className={`w-16 h-16 rounded-2xl flex items-center justify-center mb-6
                ${selectedFeature.color === "blue" ? "bg-blue-500/20 text-blue-400" : ""}
                ${selectedFeature.color === "emerald" ? "bg-emerald-500/20 text-emerald-400" : ""}
                ${selectedFeature.color === "amber" ? "bg-amber-500/20 text-amber-400" : ""}
                ${selectedFeature.color === "purple" ? "bg-purple-500/20 text-purple-400" : ""}
              `}>
                <selectedFeature.icon className="w-8 h-8" />
              </div>
              
              <h2 className="text-2xl font-bold text-white mb-2">{selectedFeature.title}</h2>
              <p className="text-gray-400 mb-6">{selectedFeature.fullDesc}</p>
              
              <h3 className="text-sm font-medium text-gray-500 uppercase tracking-wider mb-3">Capabilities</h3>
              <ul className="space-y-2 mb-6">
                {selectedFeature.capabilities.map((cap, i) => (
                  <li key={i} className="flex items-center gap-3 text-gray-300">
                    <CheckCircle2 className="w-5 h-5 text-emerald-400 flex-shrink-0" />
                    {cap}
                  </li>
                ))}
              </ul>
              
              <div className="flex gap-3">
                <button className="px-6 py-2.5 bg-blue-600 hover:bg-blue-500 text-white font-medium rounded-lg transition-colors">
                  Try It Now
                </button>
                <button 
                  onClick={() => setSelectedFeature(null)}
                  className="px-6 py-2.5 bg-gray-700 hover:bg-gray-600 text-gray-300 font-medium rounded-lg transition-colors"
                >
                  Close
                </button>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Main Content */}
      <main className="lg:ml-72 min-h-screen p-6 lg:p-8">
        <div className="max-w-7xl mx-auto space-y-6">
          
          {/* Header */}
          <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-4">
            <div>
              <motion.h1 
                initial={{ opacity: 0, x: -20 }}
                animate={{ opacity: 1, x: 0 }}
                className="text-3xl font-bold bg-gradient-to-r from-white via-blue-100 to-gray-300 bg-clip-text text-transparent"
              >
                Dashboard
              </motion.h1>
              <motion.p 
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                transition={{ delay: 0.1 }}
                className="text-gray-400 mt-1"
              >
                Real-time portfolio and AI trading insights
              </motion.p>
            </div>
            
            <div className="flex items-center gap-3">
              <TourTriggerButton />
              
              {/* Trading Mode Indicator */}
              <TradingModeIndicator 
                mode={tradingMode} 
                onClick={() => setShowModeSelector(true)}
                showLabel={true}
              />
              
              <motion.div 
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                transition={{ delay: 0.2 }}
                className="flex items-center gap-2 px-4 py-2 glass-card rounded-lg"
              >
                <div className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse" />
                <span className="text-sm text-gray-400">{lastUpdate.toLocaleTimeString()}</span>
              </motion.div>
            </div>
          </div>

          {/* Trading Mode Status Bar */}
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.05 }}
          >
            <ModeStatusBar
              mode={tradingMode}
              pendingProposals={proposals.length}
              onViewProposals={() => {}}
              onChangeMode={() => setShowModeSelector(true)}
            />
          </motion.div>

          {/* Market Regime Banner */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.1 }}
            className="glass-card rounded-2xl p-4 flex items-center justify-between"
          >
            <div className="flex items-center gap-4">
              <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-emerald-500/20 to-emerald-600/10 flex items-center justify-center">
                <Activity className="w-6 h-6 text-emerald-400" />
              </div>
              <div>
                <p className="text-sm text-gray-400">Market Regime</p>
                <div className="flex items-center gap-2">
                  <span className="text-xl font-bold text-emerald-400">{marketRegime.regime}</span>
                  <span className="text-sm text-gray-500">({Math.round(marketRegime.confidence * 100)}% confidence)</span>
                </div>
              </div>
            </div>
            
            <div className="flex items-center gap-6">
              <div className="text-center">
                <p className="text-xs text-gray-500 uppercase">VIX</p>
                <p className="text-lg font-semibold text-white">{marketRegime.vix.toFixed(1)}</p>
              </div>
              <div className="text-center">
                <p className="text-xs text-gray-500 uppercase">Breadth</p>
                <p className="text-lg font-semibold text-white">{Math.round(marketRegime.breadth * 100)}%</p>
              </div>
              <div className="text-center">
                <p className="text-xs text-gray-500 uppercase">Trend</p>
                <p className="text-lg font-semibold text-emerald-400">{marketRegime.trend}</p>
              </div>
            </div>
          </motion.div>

          {/* Stats Grid */}
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
            <StatCard
              title="Portfolio Value"
              value={`$${totalValue.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`}
              change={`+${totalPnLPercent.toFixed(2)}%`}
              changeType={totalPnL >= 0 ? "positive" : "negative"}
              icon={DollarSign}
              subtitle="+$12,342 YTD"
            />
            <StatCard
              title="Total P&L"
              value={`${totalPnL >= 0 ? "+" : ""}$${Math.abs(totalPnL).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`}
              change={`+${(totalPnL / totalCost * 100).toFixed(2)}%`}
              changeType={totalPnL >= 0 ? "positive" : "negative"}
              icon={totalPnL >= 0 ? TrendingUp : TrendingDown}
              subtitle="Unrealized gains"
            />
            <StatCard
              title="Win Rate"
              value="68.5%"
              change="+2.3%"
              changeType="positive"
              icon={Target}
              subtitle="37 winning / 17 losing"
            />
            <StatCard
              title="Sharpe Ratio"
              value="1.85"
              change="+0.12"
              changeType="positive"
              icon={BarChart3}
              subtitle="Risk-adjusted return"
            />
          </div>

          {/* Main Content Grid */}
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
            
            {/* Trading Chart */}
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.2 }}
              className="lg:col-span-2"
            >
              <TradingChart
                symbol="AAPL"
                mode={tradingMode}
                onModeChange={setTradingMode}
                riskLevels={riskLevels}
                className="h-[500px]"
              />
            </motion.div>

            {/* Sector Allocation */}
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.3 }}
              className="glass-card rounded-2xl p-6"
            >
              <h3 className="text-lg font-semibold text-white mb-6">Sector Allocation</h3>
              <div className="h-48">
                <ResponsiveContainer width="100%" height="100%">
                  <RePieChart>
                    <Pie
                      data={sectorData}
                      cx="50%"
                      cy="50%"
                      innerRadius={60}
                      outerRadius={80}
                      paddingAngle={5}
                      dataKey="value"
                    >
                      {sectorData.map((entry, index) => (
                        <Cell key={`cell-${index}`} fill={entry.color} />
                      ))}
                    </Pie>
                    <Tooltip
                      contentStyle={{ backgroundColor: "#1f2937", border: "1px solid #374151", borderRadius: "8px" }}
                      formatter={(value) => [`${Number(value)}%`, "Allocation"]}
                    />
                  </RePieChart>
                </ResponsiveContainer>
              </div>
              <div className="mt-4 space-y-2">
                {sectorData.map((sector) => (
                  <div key={sector.name} className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <div className="w-3 h-3 rounded-full" style={{ backgroundColor: sector.color }} />
                      <span className="text-sm text-gray-300">{sector.name}</span>
                    </div>
                    <span className="text-sm font-medium text-white">{sector.value}%</span>
                  </div>
                ))}
              </div>
            </motion.div>
          </div>

          {/* AI Trade Proposals */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.4 }}
            className="glass-card rounded-2xl p-6"
          >
            <div className="flex items-center justify-between mb-6">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-blue-500/20 to-cyan-500/10 flex items-center justify-center">
                  <Sparkles className="w-5 h-5 text-blue-400" />
                </div>
                <div>
                  <h3 className="text-lg font-semibold text-white">AI Trade Proposals</h3>
                  <p className="text-sm text-gray-500">
                    {tradingMode === "manual" && proposals.length > 0 
                      ? `${proposals.length} proposals - Copy to your broker to execute` 
                      : tradingMode === "fully_auto" && proposals.length > 0
                      ? `${proposals.length} proposals above auto-execution threshold`
                      : `${proposals.length} pending proposals`}
                  </p>
                </div>
              </div>
              <button className="flex items-center gap-2 px-4 py-2 text-blue-400 hover:text-blue-300 text-sm font-medium transition-colors">
                View All
                <ArrowRight className="w-4 h-4" />
              </button>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              <AnimatePresence>
                {proposals.map((proposal, index) => (
                  <ProposalCard
                    key={proposal.symbol}
                    proposal={proposal}
                    onConfirm={() => handleConfirmProposal(index)}
                    onReject={() => handleRejectProposal(index)}
                  />
                ))}
              </AnimatePresence>
              {proposals.length === 0 && (
                <motion.div
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  className="col-span-full py-12 text-center"
                >
                  <CheckCircle2 className="w-12 h-12 text-emerald-400 mx-auto mb-4" />
                  <p className="text-gray-400">All caught up! No pending proposals.</p>
                </motion.div>
              )}
            </div>
          </motion.div>

          {/* Active Positions */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.5 }}
            className="glass-card rounded-2xl overflow-hidden"
          >
            <div className="p-6 border-b border-gray-800">
              <div className="flex items-center justify-between">
                <div>
                  <h3 className="text-lg font-semibold text-white">Active Positions</h3>
                  <p className="text-sm text-gray-500">{positions.length} open positions</p>
                </div>
                <button 
                  onClick={() => window.location.href = "/positions"}
                  className="flex items-center gap-2 px-4 py-2 text-blue-400 hover:text-blue-300 
                    text-sm font-medium transition-colors"
                >
                  View All Positions
                  <ArrowRight className="w-4 h-4" />
                </button>
              </div>
            </div>
            
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead>
                  <tr className="text-left text-xs text-gray-500 uppercase tracking-wider">
                    <th className="px-6 py-4 font-medium">Symbol</th>
                    <th className="px-6 py-4 font-medium">Name</th>
                    <th className="px-6 py-4 font-medium">Qty</th>
                    <th className="px-6 py-4 font-medium">Avg Price</th>
                    <th className="px-6 py-4 font-medium">Current</th>
                    <th className="px-6 py-4 font-medium">P&L</th>
                    <th className="px-6 py-4 font-medium">Weight</th>
                  </tr>
                </thead>
                <tbody>
                  {positions.map((pos, index) => {
                    const pnl = (pos.currentPrice - pos.avgPrice) * pos.qty;
                    const pnlPercent = ((pos.currentPrice - pos.avgPrice) / pos.avgPrice) * 100;
                    const weight = ((pos.qty * pos.currentPrice) / totalValue) * 100;
                    
                    return (
                      <motion.tr
                        key={pos.symbol}
                        initial={{ opacity: 0, x: -20 }}
                        animate={{ opacity: 1, x: 0 }}
                        transition={{ delay: index * 0.05 }}
                        className="border-t border-gray-800 hover:bg-gray-800/30 transition-colors"
                      >
                        <td className="px-6 py-4">
                          <span className="font-semibold text-white">{pos.symbol}</span>
                        </td>
                        <td className="px-6 py-4 text-sm text-gray-400">{pos.name}</td>
                        <td className="px-6 py-4 text-white">{pos.qty}</td>
                        <td className="px-6 py-4 text-gray-400">${pos.avgPrice.toFixed(2)}</td>
                        <td className="px-6 py-4">
                          <span className="text-white font-medium">${pos.currentPrice.toFixed(2)}</span>
                        </td>
                        <td className="px-6 py-4">
                          <div className={`flex items-center gap-1 ${pnl >= 0 ? "text-emerald-400" : "text-rose-400"}`}>
                            {pnl >= 0 ? <TrendingUp className="w-4 h-4" /> : <TrendingDown className="w-4 h-4" />}
                            <span className="font-medium">{pnl >= 0 ? "+" : ""}${Math.abs(pnl).toFixed(0)}</span>
                            <span className="text-xs">({pnlPercent >= 0 ? "+" : ""}{pnlPercent.toFixed(2)}%)</span>
                          </div>
                        </td>
                        <td className="px-6 py-4">
                          <div className="flex items-center gap-2">
                            <div className="flex-1 h-2 bg-gray-700 rounded-full overflow-hidden w-20">
                              <div
                                className="h-full bg-blue-500 rounded-full"
                                style={{ width: `${Math.min(weight, 100)}%` }}
                              />
                            </div>
                            <span className="text-sm text-gray-400 w-12">{weight.toFixed(1)}%</span>
                          </div>
                        </td>
                      </motion.tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          </motion.div>

          {/* Feature Grid - What You Can Do */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.6 }}
          >
            <div className="flex items-center gap-3 mb-6">
              <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-emerald-500/20 to-emerald-600/10 flex items-center justify-center">
                <Zap className="w-5 h-5 text-emerald-400" />
              </div>
              <div>
                <h3 className="text-lg font-semibold text-white">What You Can Do</h3>
                <p className="text-sm text-gray-500">Explore Investor OS features</p>
              </div>
            </div>
            
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
              {features.slice(0, 4).map((feature) => (
                <FeatureCard 
                  key={feature.id} 
                  feature={feature} 
                  onClick={() => setSelectedFeature(feature)}
                />
              ))}
            </div>
          </motion.div>

          {/* Bottom Row: Quick Actions & System Status */}
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <QuickActionsPanel />
            <SystemStatusOverview />
          </div>

          {/* Trading Flow Diagram */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.65 }}
          >
            <TradingFlowDiagram mode={tradingMode} />
          </motion.div>

          {/* Help Section */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.7 }}
            className="glass-card rounded-2xl p-6"
          >
            <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
              <div className="flex items-center gap-4">
                <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-purple-500/20 to-pink-500/10 flex items-center justify-center">
                  <HelpCircle className="w-6 h-6 text-purple-400" />
                </div>
                <div>
                  <h3 className="text-lg font-semibold text-white">Need Help?</h3>
                  <p className="text-sm text-gray-500">New to Investor OS? Take a tour or read the docs</p>
                </div>
              </div>
              <div className="flex gap-3">
                <button
                  onClick={() => setShowTour(true)}
                  className="flex items-center gap-2 px-4 py-2.5 bg-gray-800 hover:bg-gray-700 text-white 
                    font-medium rounded-lg transition-colors"
                >
                  <Play className="w-4 h-4" />
                  Take Tour
                </button>
                <button className="flex items-center gap-2 px-4 py-2.5 bg-blue-600 hover:bg-blue-500 text-white 
                  font-medium rounded-lg transition-colors"
                >
                  <FileText className="w-4 h-4" />
                  Documentation
                </button>
              </div>
            </div>
          </motion.div>

        </div>
      </main>
    </div>
  );
}
