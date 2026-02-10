"use client";

import { motion } from "framer-motion";
import {
  Brain,
  TrendingUp,
  Shield,
  Zap,
  BarChart3,
  Lock,
  RefreshCw,
  Bell,
  LineChart,
  PieChart,
  Activity,
  Target,
  Cpu,
  Globe,
  Clock,
  Wallet,
  FileText,
  Settings,
  Search,
  AlertTriangle,
  CheckCircle2,
  XCircle,
  ChevronRight,
} from "lucide-react";
import { useState } from "react";

// ============================================
// FEATURE DEFINITIONS
// ============================================

export const features = [
  {
    id: "ai-trading",
    icon: Brain,
    title: "AI-Powered Trading",
    shortDesc: "Smart trade proposals",
    fullDesc: "Our AI analyzes market data, insider activity, sentiment, and technical indicators to generate high-confidence trade proposals with CQ (Conviction Quotient) scores.",
    capabilities: [
      "CQ Score calculation (0-100%)",
      "Multi-factor analysis (PEGY, Insider, Sentiment)",
      "Automatic trade proposal generation",
      "Confidence-based filtering",
      "Rationale explanations",
    ],
    status: "active",
    color: "blue",
  },
  {
    id: "portfolio-mgmt",
    icon: PieChart,
    title: "Portfolio Management",
    shortDesc: "Track & optimize positions",
    fullDesc: "Real-time portfolio tracking with P&L calculations, sector allocation analysis, and risk monitoring.",
    capabilities: [
      "Real-time P&L tracking",
      "Sector allocation visualization",
      "Position weight monitoring",
      "Win rate analytics",
      "Sharpe ratio calculation",
    ],
    status: "active",
    color: "emerald",
  },
  {
    id: "risk-mgmt",
    icon: Shield,
    title: "Risk Management",
    shortDesc: "Protect your capital",
    fullDesc: "Advanced risk controls including VaR calculation, kill switch, concentration alerts, and circuit breakers.",
    capabilities: [
      "Value at Risk (VaR) 95%",
      "Kill switch (emergency stop)",
      "Concentration warnings",
      "Circuit breakers for APIs",
      "Daily loss limits",
    ],
    status: "active",
    color: "amber",
  },
  {
    id: "backtesting",
    icon: LineChart,
    title: "Backtesting Engine",
    shortDesc: "Test strategies historically",
    fullDesc: "Walk-forward analysis with transaction costs, slippage modeling, and performance attribution.",
    capabilities: [
      "Walk-forward analysis",
      "Transaction cost modeling",
      "Slippage simulation",
      "Sharpe/Sortino ratios",
      "Max drawdown analysis",
    ],
    status: "active",
    color: "purple",
  },
  {
    id: "market-regime",
    icon: Activity,
    title: "Market Regime Detection",
    shortDesc: "Know when to trade",
    fullDesc: "AI detects market regimes (Risk On/Off, Uncertain) using VIX, breadth, and trend analysis.",
    capabilities: [
      "Risk On/Off detection",
      "VIX level monitoring",
      "Market breadth analysis",
      "Regime fit scoring",
      "Automatic position sizing",
    ],
    status: "active",
    color: "cyan",
  },
  {
    id: "ml-analytics",
    icon: Cpu,
    title: "ML Analytics",
    shortDesc: "Predictive modeling",
    fullDesc: "XGBoost-based CQ prediction with anomaly detection and feature importance analysis.",
    capabilities: [
      "XGBoost CQ prediction",
      "Anomaly detection",
      "Feature importance",
      "Regime change alerts",
      "Model performance tracking",
    ],
    status: "active",
    color: "rose",
  },
  {
    id: "order-execution",
    icon: Zap,
    title: "Smart Order Execution",
    shortDesc: "Optimal trade routing",
    fullDesc: "Intelligent order routing with broker integration (Alpaca, Interactive Brokers, Binance).",
    capabilities: [
      "Multi-broker support",
      "TWAP/VWAP algorithms",
      "Paper trading mode",
      "Order status tracking",
      "Execution analysis",
    ],
    status: "active",
    color: "orange",
  },
  {
    id: "journal",
    icon: FileText,
    title: "Trading Journal",
    shortDesc: "Log & learn",
    fullDesc: "RAG-powered trading journal with AI-assisted reflection and decision tracking.",
    capabilities: [
      "Trade logging",
      "AI-powered insights",
      "Decision rationale tracking",
      "Performance review",
      "Pattern recognition",
    ],
    status: "coming-soon",
    color: "gray",
  },
];

// ============================================
// QUICK ACTIONS
// ============================================

export const quickActions = [
  {
    id: "new-trade",
    icon: TrendingUp,
    label: "New Trade",
    description: "Execute a manual trade",
    shortcut: "⌘T",
    color: "blue",
  },
  {
    id: "view-proposals",
    icon: Target,
    label: "Review Proposals",
    description: "3 AI proposals waiting",
    shortcut: "⌘P",
    badge: 3,
    color: "emerald",
  },
  {
    id: "run-backtest",
    icon: RefreshCw,
    label: "Run Backtest",
    description: "Test strategy historically",
    shortcut: "⌘B",
    color: "purple",
  },
  {
    id: "risk-check",
    icon: AlertTriangle,
    label: "Risk Check",
    description: "Current: Medium risk",
    shortcut: "⌘R",
    color: "amber",
  },
  {
    id: "export-report",
    icon: FileText,
    label: "Export Report",
    description: "Generate PDF summary",
    shortcut: "⌘E",
    color: "gray",
  },
  {
    id: "settings",
    icon: Settings,
    label: "Settings",
    description: "Configure system",
    shortcut: "⌘,",
    color: "gray",
  },
];

// ============================================
// FEATURE CARD COMPONENT
// ============================================

export function FeatureCard({ feature, onClick }: { feature: typeof features[0]; onClick?: () => void }) {
  const Icon = feature.icon;
  const colorClasses = {
    blue: "from-blue-500/20 to-blue-600/10 text-blue-400 border-blue-500/30",
    emerald: "from-emerald-500/20 to-emerald-600/10 text-emerald-400 border-emerald-500/30",
    amber: "from-amber-500/20 to-amber-600/10 text-amber-400 border-amber-500/30",
    purple: "from-purple-500/20 to-purple-600/10 text-purple-400 border-purple-500/30",
    cyan: "from-cyan-500/20 to-cyan-600/10 text-cyan-400 border-cyan-500/30",
    rose: "from-rose-500/20 to-rose-600/10 text-rose-400 border-rose-500/30",
    orange: "from-orange-500/20 to-orange-600/10 text-orange-400 border-orange-500/30",
    gray: "from-gray-500/20 to-gray-600/10 text-gray-400 border-gray-500/30",
  };

  return (
    <motion.div
      whileHover={{ scale: 1.02, y: -4 }}
      whileTap={{ scale: 0.98 }}
      onClick={onClick}
      className={`relative p-6 rounded-2xl bg-gradient-to-br ${colorClasses[feature.color as keyof typeof colorClasses]} 
        border backdrop-blur-sm cursor-pointer group transition-all duration-300
        ${feature.status === "coming-soon" ? "opacity-60" : ""}`}
    >
      {feature.status === "coming-soon" && (
        <span className="absolute top-4 right-4 text-xs font-medium text-gray-500 bg-gray-800/50 px-2 py-1 rounded-full">
          Coming Soon
        </span>
      )}
      
      <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-gray-800 to-gray-900 flex items-center justify-center mb-4 
        group-hover:scale-110 transition-transform duration-300 shadow-lg">
        <Icon className="w-6 h-6" />
      </div>
      
      <h3 className="text-lg font-semibold text-white mb-2">{feature.title}</h3>
      <p className="text-sm text-gray-400 mb-4">{feature.shortDesc}</p>
      
      <div className="flex items-center text-xs font-medium opacity-0 group-hover:opacity-100 transition-opacity">
        <span>Learn more</span>
        <ChevronRight className="w-4 h-4 ml-1" />
      </div>
    </motion.div>
  );
}

// ============================================
// QUICK ACTIONS PANEL
// ============================================

export function QuickActionsPanel() {
  const [hoveredAction, setHoveredAction] = useState<string | null>(null);

  return (
    <div className="glass-card rounded-2xl p-6">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h3 className="text-lg font-semibold text-white">Quick Actions</h3>
          <p className="text-sm text-gray-500">Fast access to common tasks</p>
        </div>
        <div className="text-xs text-gray-500 bg-gray-800/50 px-3 py-1.5 rounded-full">
          Press ⌘ for shortcuts
        </div>
      </div>
      
      <div className="grid grid-cols-2 md:grid-cols-3 gap-3">
        {quickActions.map((action) => {
          const Icon = action.icon;
          const isHovered = hoveredAction === action.id;
          
          return (
            <motion.button
              key={action.id}
              onMouseEnter={() => setHoveredAction(action.id)}
              onMouseLeave={() => setHoveredAction(null)}
              whileHover={{ scale: 1.02 }}
              whileTap={{ scale: 0.98 }}
              className="relative flex items-center gap-3 p-4 rounded-xl bg-gray-800/30 border border-gray-700/50 
                hover:bg-gray-800/50 hover:border-gray-600 transition-all text-left group"
            >
              <div className={`w-10 h-10 rounded-lg flex items-center justify-center transition-colors
                ${action.color === "blue" ? "bg-blue-500/10 text-blue-400" : ""}
                ${action.color === "emerald" ? "bg-emerald-500/10 text-emerald-400" : ""}
                ${action.color === "amber" ? "bg-amber-500/10 text-amber-400" : ""}
                ${action.color === "purple" ? "bg-purple-500/10 text-purple-400" : ""}
                ${action.color === "gray" ? "bg-gray-500/10 text-gray-400" : ""}
              `}>
                <Icon className="w-5 h-5" />
                {action.badge && (
                  <span className="absolute -top-1 -right-1 w-5 h-5 bg-red-500 text-white text-xs 
                    font-bold rounded-full flex items-center justify-center">
                    {action.badge}
                  </span>
                )}
              </div>
              
              <div className="flex-1 min-w-0">
                <p className="font-medium text-white text-sm">{action.label}</p>
                <p className="text-xs text-gray-500 truncate">{action.description}</p>
              </div>
              
              <span className="text-xs text-gray-600 font-mono hidden lg:block">{action.shortcut}</span>
            </motion.button>
          );
        })}
      </div>
    </div>
  );
}

// ============================================
// WHAT YOU CAN DO SECTION
// ============================================

export function WhatYouCanDo() {
  const capabilities = [
    {
      title: "Review AI Trade Proposals",
      desc: "Get AI-generated trade ideas with CQ scores and confidence levels",
      icon: Target,
      action: "3 pending proposals",
      color: "emerald",
    },
    {
      title: "Execute Manual Trades",
      desc: "Place buy/sell orders across multiple brokers",
      icon: TrendingUp,
      action: "New Trade",
      color: "blue",
    },
    {
      title: "Monitor Portfolio",
      desc: "Track P&L, positions, and risk metrics in real-time",
      icon: PieChart,
      action: "View Positions",
      color: "purple",
    },
    {
      title: "Test Strategies",
      desc: "Run backtests with historical data and transaction costs",
      icon: RefreshCw,
      action: "Run Backtest",
      color: "amber",
    },
    {
      title: "Check Risk Metrics",
      desc: "Monitor VaR, concentration, and circuit breaker status",
      icon: Shield,
      action: "Risk Dashboard",
      color: "rose",
    },
    {
      title: "Analyze Market Regime",
      desc: "See current market conditions (Risk On/Off/Uncertain)",
      icon: Activity,
      action: "View Regime",
      color: "cyan",
    },
  ];

  return (
    <div className="glass-card rounded-2xl p-6">
      <div className="flex items-center gap-3 mb-6">
        <div className="w-10 h-10 rounded-lg bg-blue-500/10 flex items-center justify-center">
          <Zap className="w-5 h-5 text-blue-400" />
        </div>
        <div>
          <h3 className="text-lg font-semibold text-white">What You Can Do</h3>
          <p className="text-sm text-gray-500">Quick guide to Investor OS capabilities</p>
        </div>
      </div>
      
      <div className="space-y-3">
        {capabilities.map((cap, index) => {
          const Icon = cap.icon;
          return (
            <motion.div
              key={cap.title}
              initial={{ opacity: 0, x: -20 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ delay: index * 0.1 }}
              className="flex items-start gap-4 p-4 rounded-xl bg-gray-800/20 hover:bg-gray-800/40 
                transition-colors group cursor-pointer"
            >
              <div className={`w-10 h-10 rounded-lg flex items-center justify-center flex-shrink-0
                ${cap.color === "blue" ? "bg-blue-500/10 text-blue-400" : ""}
                ${cap.color === "emerald" ? "bg-emerald-500/10 text-emerald-400" : ""}
                ${cap.color === "purple" ? "bg-purple-500/10 text-purple-400" : ""}
                ${cap.color === "amber" ? "bg-amber-500/10 text-amber-400" : ""}
                ${cap.color === "rose" ? "bg-rose-500/10 text-rose-400" : ""}
                ${cap.color === "cyan" ? "bg-cyan-500/10 text-cyan-400" : ""}
              `}>
                <Icon className="w-5 h-5" />
              </div>
              
              <div className="flex-1">
                <div className="flex items-center justify-between mb-1">
                  <h4 className="font-medium text-white">{cap.title}</h4>
                  <span className="text-xs text-blue-400 font-medium">{cap.action}</span>
                </div>
                <p className="text-sm text-gray-400">{cap.desc}</p>
              </div>
              
              <ChevronRight className="w-5 h-5 text-gray-600 group-hover:text-gray-400 transition-colors" />
            </motion.div>
          );
        })}
      </div>
    </div>
  );
}

// ============================================
// SYSTEM STATUS OVERVIEW
// ============================================

export function SystemStatusOverview() {
  const services = [
    { name: "AI Engine", status: "running", latency: "45ms" },
    { name: "Market Data", status: "running", latency: "12ms" },
    { name: "Broker API", status: "running", latency: "89ms" },
    { name: "Risk Engine", status: "running", latency: "23ms" },
    { name: "Backtest Engine", status: "running", latency: "156ms" },
  ];

  return (
    <div className="glass-card rounded-2xl p-6">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h3 className="text-lg font-semibold text-white">System Status</h3>
          <p className="text-sm text-gray-500">All systems operational</p>
        </div>
        <div className="flex items-center gap-2 px-3 py-1.5 bg-emerald-500/10 rounded-full border border-emerald-500/20">
          <div className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse" />
          <span className="text-xs font-medium text-emerald-400">Operational</span>
        </div>
      </div>
      
      <div className="space-y-3">
        {services.map((service) => (
          <div key={service.name} className="flex items-center justify-between py-2">
            <div className="flex items-center gap-3">
              <div className="w-2 h-2 rounded-full bg-emerald-500" />
              <span className="text-sm text-gray-300">{service.name}</span>
            </div>
            <span className="text-xs text-gray-500 font-mono">{service.latency}</span>
          </div>
        ))}
      </div>
      
      <div className="mt-6 pt-4 border-t border-gray-800">
        <div className="grid grid-cols-3 gap-4 text-center">
          <div>
            <p className="text-2xl font-bold text-white">99.9%</p>
            <p className="text-xs text-gray-500">Uptime</p>
          </div>
          <div>
            <p className="text-2xl font-bold text-white">&lt;50ms</p>
            <p className="text-xs text-gray-500">Avg Latency</p>
          </div>
          <div>
            <p className="text-2xl font-bold text-white">102</p>
            <p className="text-xs text-gray-500">Tests Passing</p>
          </div>
        </div>
      </div>
    </div>
  );
}
