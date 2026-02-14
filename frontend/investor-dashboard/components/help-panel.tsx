"use client";

import React, { useState, useEffect, createContext, useContext, ReactNode } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  HelpCircle,
  X,
  BookOpen,
  Lightbulb,
  MessageCircle,
  ChevronRight,
  Search,
  Sparkles,
  Info,
  AlertTriangle,
  CheckCircle2,
  Keyboard,
  Zap,
  BarChart3,
  Shield,
  TrendingUp,
  Brain,
  Settings,
  FileText,
  Activity,
  DollarSign,
  Target,
  Clock,
  Bell,
  LayoutDashboard,
  PieChart,
  Wallet,
} from "lucide-react";

// ============================================================================
// TYPES & CONTEXT
// ============================================================================

export type HelpTopic = {
  id: string;
  title: string;
  category: string;
  icon: React.ElementType;
  shortDesc: string;
  fullContent: React.ReactNode;
  relatedTopics?: string[];
  shortcuts?: { key: string; description: string }[];
};

type HelpContextType = {
  isOpen: boolean;
  setIsOpen: (open: boolean) => void;
  currentTopic: HelpTopic | null;
  setCurrentTopic: (topic: HelpTopic | null) => void;
  activeContext: string;
  setActiveContext: (context: string) => void;
  showTooltip: (content: string, targetRect: DOMRect) => void;
  hideTooltip: () => void;
};

const HelpContext = createContext<HelpContextType | undefined>(undefined);

export function useHelp() {
  const context = useContext(HelpContext);
  if (!context) {
    throw new Error("useHelp must be used within HelpProvider");
  }
  return context;
}

// ============================================================================
// HELP PROVIDER
// ============================================================================

export function HelpProvider({ children }: { children: ReactNode }) {
  const [isOpen, setIsOpen] = useState(false);
  const [currentTopic, setCurrentTopic] = useState<HelpTopic | null>(null);
  const [activeContext, setActiveContext] = useState("dashboard");
  const [tooltipContent, setTooltipContent] = useState<string | null>(null);
  const [tooltipPosition, setTooltipPosition] = useState({ x: 0, y: 0 });

  const showTooltip = (content: string, targetRect: DOMRect) => {
    setTooltipContent(content);
    setTooltipPosition({
      x: targetRect.left + targetRect.width / 2,
      y: targetRect.top - 10,
    });
  };

  const hideTooltip = () => {
    setTooltipContent(null);
  };

  return (
    <HelpContext.Provider
      value={{
        isOpen,
        setIsOpen,
        currentTopic,
        setCurrentTopic,
        activeContext,
        setActiveContext,
        showTooltip,
        hideTooltip,
      }}
    >
      {children}
      <HelpTooltip content={tooltipContent} position={tooltipPosition} />
    </HelpContext.Provider>
  );
}

// ============================================================================
// HOVER TOOLTIP COMPONENT
// ============================================================================

function HelpTooltip({
  content,
  position,
}: {
  content: string | null;
  position: { x: number; y: number };
}) {
  if (!content) return null;

  return (
    <motion.div
      initial={{ opacity: 0, y: 5, scale: 0.95 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, y: 5, scale: 0.95 }}
      className="fixed z-[100] max-w-xs pointer-events-none"
      style={{
        left: position.x,
        top: position.y,
        transform: "translateX(-50%) translateY(-100%)",
      }}
    >
      <div className="glass-card px-4 py-3 rounded-xl border border-blue-500/30 shadow-2xl shadow-blue-500/20">
        <div className="flex items-start gap-2">
          <Sparkles className="w-4 h-4 text-blue-400 flex-shrink-0 mt-0.5" />
          <p className="text-sm text-gray-200 leading-relaxed">{content}</p>
        </div>
      </div>
      <div className="absolute bottom-0 left-1/2 transform -translate-x-1/2 translate-y-full">
        <div className="w-0 h-0 border-l-[6px] border-l-transparent border-r-[6px] border-r-transparent border-t-[8px] border-t-gray-800/90" />
      </div>
    </motion.div>
  );
}

// ============================================================================
// HOOK FOR CONTEXT-AWARE HELP
// ============================================================================

export function useContextualHelp(topicId: string) {
  const { setCurrentTopic, setIsOpen, setActiveContext } = useHelp();
  const [isHovered, setIsHovered] = useState(false);

  const topic = helpTopics.find((t) => t.id === topicId);

  const handleMouseEnter = (e: React.MouseEvent) => {
    setIsHovered(true);
    if (topic) {
      setActiveContext(topicId);
    }
  };

  const handleMouseLeave = () => {
    setIsHovered(false);
  };

  const handleClick = () => {
    if (topic) {
      setCurrentTopic(topic);
      setIsOpen(true);
    }
  };

  return {
    isHovered,
    handleMouseEnter,
    handleMouseLeave,
    handleClick,
    topic,
  };
}

// ============================================================================
// CONTEXTUAL HELP TRIGGER (HOVER ELEMENT)
// ============================================================================

export function ContextualHelpTrigger({
  topicId,
  children,
  className = "",
}: {
  topicId: string;
  children: ReactNode;
  className?: string;
}) {
  const { handleMouseEnter, handleMouseLeave, handleClick } =
    useContextualHelp(topicId);

  return (
    <div
      className={`relative ${className}`}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      onClick={handleClick}
    >
      {children}
      <div className="absolute -top-1 -right-1 w-2 h-2 rounded-full bg-blue-500 animate-pulse opacity-0 group-hover:opacity-100 transition-opacity" />
    </div>
  );
}

// ============================================================================
// HELP TOPICS DATA
// ============================================================================

export const helpTopics: HelpTopic[] = [
  {
    id: "dashboard",
    title: "Dashboard Overview",
    category: "Getting Started",
    icon: LayoutDashboard,
    shortDesc: "Your command center for portfolio management",
    fullContent: (
      <div className="space-y-4">
        <p className="text-gray-300 leading-relaxed">
          The Investor OS Dashboard is your central hub for monitoring portfolio performance, 
          reviewing AI trading proposals, and accessing all system features.
        </p>
        
        <div className="grid grid-cols-2 gap-3 mt-4">
          <div className="p-3 rounded-lg bg-gray-800/50 border border-gray-700">
            <div className="flex items-center gap-2 mb-2">
              <DollarSign className="w-4 h-4 text-emerald-400" />
              <span className="text-sm font-medium text-white">Portfolio Value</span>
            </div>
            <p className="text-xs text-gray-400">Real-time value of all holdings</p>
          </div>
          <div className="p-3 rounded-lg bg-gray-800/50 border border-gray-700">
            <div className="flex items-center gap-2 mb-2">
              <TrendingUp className="w-4 h-4 text-blue-400" />
              <span className="text-sm font-medium text-white">P&L Tracking</span>
            </div>
            <p className="text-xs text-gray-400">Unrealized gains and losses</p>
          </div>
          <div className="p-3 rounded-lg bg-gray-800/50 border border-gray-700">
            <div className="flex items-center gap-2 mb-2">
              <Target className="w-4 h-4 text-purple-400" />
              <span className="text-sm font-medium text-white">Win Rate</span>
            </div>
            <p className="text-xs text-gray-400">Success ratio of trades</p>
          </div>
          <div className="p-3 rounded-lg bg-gray-800/50 border border-gray-700">
            <div className="flex items-center gap-2 mb-2">
              <BarChart3 className="w-4 h-4 text-amber-400" />
              <span className="text-sm font-medium text-white">Sharpe Ratio</span>
            </div>
            <p className="text-xs text-gray-400">Risk-adjusted returns</p>
          </div>
        </div>
        
        <div className="mt-4 p-4 rounded-lg bg-blue-500/10 border border-blue-500/20">
          <div className="flex items-start gap-3">
            <Lightbulb className="w-5 h-5 text-blue-400 flex-shrink-0 mt-0.5" />
            <div>
              <h4 className="text-sm font-medium text-blue-400 mb-1">Pro Tip</h4>
              <p className="text-sm text-gray-300">
                Use the <kbd className="px-2 py-0.5 bg-gray-700 rounded text-xs">Cmd+K</kbd> shortcut 
                to quickly access any feature from anywhere in the app.
              </p>
            </div>
          </div>
        </div>
      </div>
    ),
    relatedTopics: ["trading-modes", "positions", "proposals"],
    shortcuts: [
      { key: "Cmd+K", description: "Open Command Palette" },
      { key: "?", description: "Toggle Help Panel" },
      { key: "P", description: "Go to Portfolio" },
    ],
  },
  {
    id: "trading-modes",
    title: "Trading Modes",
    category: "AI Trading",
    icon: Brain,
    shortDesc: "Choose how AI assists your trading",
    fullContent: (
      <div className="space-y-4">
        <p className="text-gray-300 leading-relaxed">
          Investor OS offers three trading modes to match your comfort level with AI automation.
        </p>
        
        <div className="space-y-3 mt-4">
          <div className="p-4 rounded-xl bg-gray-800/50 border border-emerald-500/30">
            <div className="flex items-center gap-3 mb-2">
              <div className="w-8 h-8 rounded-lg bg-emerald-500/20 flex items-center justify-center">
                <span className="text-emerald-400 font-bold">M</span>
              </div>
              <div>
                <h4 className="font-medium text-white">Manual Mode</h4>
                <p className="text-xs text-emerald-400">AI provides analysis only</p>
              </div>
            </div>
            <p className="text-sm text-gray-400">
              AI generates insights and proposals but you execute all trades manually. 
              Best for learning and maintaining full control.
            </p>
          </div>
          
          <div className="p-4 rounded-xl bg-gray-800/50 border border-amber-500/30">
            <div className="flex items-center gap-3 mb-2">
              <div className="w-8 h-8 rounded-lg bg-amber-500/20 flex items-center justify-center">
                <span className="text-amber-400 font-bold">S</span>
              </div>
              <div>
                <h4 className="font-medium text-white">Semi-Auto Mode</h4>
                <p className="text-xs text-amber-400">AI proposes, you approve</p>
              </div>
            </div>
            <p className="text-sm text-gray-400">
              AI identifies opportunities and sends proposals. You review and approve 
              each trade before execution. Recommended for most users.
            </p>
          </div>
          
          <div className="p-4 rounded-xl bg-gray-800/50 border border-purple-500/30">
            <div className="flex items-center gap-3 mb-2">
              <div className="w-8 h-8 rounded-lg bg-purple-500/20 flex items-center justify-center">
                <span className="text-purple-400 font-bold">A</span>
              </div>
              <div>
                <h4 className="font-medium text-white">Fully Auto Mode</h4>
                <p className="text-xs text-purple-400">AI trades within limits</p>
              </div>
            </div>
            <p className="text-sm text-gray-400">
              AI executes trades automatically within your configured risk limits. 
              Set strict boundaries and let AI optimize continuously.
            </p>
          </div>
        </div>
        
        <div className="mt-4 p-4 rounded-lg bg-amber-500/10 border border-amber-500/20">
          <div className="flex items-start gap-3">
            <AlertTriangle className="w-5 h-5 text-amber-400 flex-shrink-0 mt-0.5" />
            <div>
              <h4 className="text-sm font-medium text-amber-400 mb-1">Important</h4>
              <p className="text-sm text-gray-300">
                Always configure your risk limits before enabling any automated trading mode.
              </p>
            </div>
          </div>
        </div>
      </div>
    ),
    relatedTopics: ["risk-management", "proposals"],
  },
  {
    id: "proposals",
    title: "AI Trade Proposals",
    category: "AI Trading",
    icon: Sparkles,
    shortDesc: "Review and act on AI-generated trade ideas",
    fullContent: (
      <div className="space-y-4">
        <p className="text-gray-300 leading-relaxed">
          AI Trade Proposals are intelligent recommendations generated by our advanced 
          algorithms analyzing market conditions, your portfolio, and risk parameters.
        </p>
        
        <h4 className="text-sm font-medium text-white mt-4">Understanding Confidence Scores</h4>
        <div className="space-y-2 mt-2">
          <div className="flex items-center justify-between p-2 rounded bg-emerald-500/10 border border-emerald-500/20">
            <span className="text-sm text-emerald-400">High (85%+)</span>
            <span className="text-xs text-gray-400">Strong signal, favorable conditions</span>
          </div>
          <div className="flex items-center justify-between p-2 rounded bg-amber-500/10 border border-amber-500/20">
            <span className="text-sm text-amber-400">Medium (70-84%)</span>
            <span className="text-xs text-gray-400">Good opportunity, moderate risk</span>
          </div>
          <div className="flex items-center justify-between p-2 rounded bg-blue-500/10 border border-blue-500/20">
            <span className="text-sm text-blue-400">Low (50-69%)</span>
            <span className="text-xs text-gray-400">Speculative, higher uncertainty</span>
          </div>
        </div>
        
        <h4 className="text-sm font-medium text-white mt-4">Available Actions</h4>
        <div className="grid grid-cols-2 gap-2 mt-2">
          <div className="p-2 rounded bg-gray-800 border border-gray-700 text-center">
            <CheckCircle2 className="w-4 h-4 text-emerald-400 mx-auto mb-1" />
            <span className="text-xs text-gray-300">Confirm</span>
          </div>
          <div className="p-2 rounded bg-gray-800 border border-gray-700 text-center">
            <X className="w-4 h-4 text-rose-400 mx-auto mb-1" />
            <span className="text-xs text-gray-300">Reject</span>
          </div>
          <div className="p-2 rounded bg-gray-800 border border-gray-700 text-center">
            <Clock className="w-4 h-4 text-amber-400 mx-auto mb-1" />
            <span className="text-xs text-gray-300">Snooze</span>
          </div>
          <div className="p-2 rounded bg-gray-800 border border-gray-700 text-center">
            <Info className="w-4 h-4 text-blue-400 mx-auto mb-1" />
            <span className="text-xs text-gray-300">Details</span>
          </div>
        </div>
      </div>
    ),
    relatedTopics: ["trading-modes", "positions"],
  },
  {
    id: "positions",
    title: "Portfolio Positions",
    category: "Portfolio",
    icon: PieChart,
    shortDesc: "Track your holdings and performance",
    fullContent: (
      <div className="space-y-4">
        <p className="text-gray-300 leading-relaxed">
          The Positions table displays all your current holdings with real-time 
          pricing, P&L calculations, and portfolio weight analysis.
        </p>
        
        <h4 className="text-sm font-medium text-white mt-4">Key Metrics</h4>
        <ul className="space-y-2 text-sm text-gray-400">
          <li className="flex items-start gap-2">
            <span className="text-blue-400">•</span>
            <span><strong className="text-gray-300">Qty:</strong> Number of shares/contracts held</span>
          </li>
          <li className="flex items-start gap-2">
            <span className="text-blue-400">•</span>
            <span><strong className="text-gray-300">Avg Price:</strong> Average cost basis per share</span>
          </li>
          <li className="flex items-start gap-2">
            <span className="text-blue-400">•</span>
            <span><strong className="text-gray-300">P&L:</strong> Unrealized profit/loss</span>
          </li>
          <li className="flex items-start gap-2">
            <span className="text-blue-400">•</span>
            <span><strong className="text-gray-300">Weight:</strong> Position size as % of portfolio</span>
          </li>
        </ul>
        
        <div className="mt-4 p-3 rounded-lg bg-gray-800/50 border border-gray-700">
          <div className="flex items-center gap-2 mb-2">
            <Zap className="w-4 h-4 text-yellow-400" />
            <span className="text-sm font-medium text-white">Rebalancing Suggestions</span>
          </div>
          <p className="text-xs text-gray-400">
            When a position exceeds your target allocation by more than 5%, 
            AI will suggest rebalancing trades to maintain optimal diversification.
          </p>
        </div>
      </div>
    ),
    relatedTopics: ["dashboard", "risk-management"],
  },
  {
    id: "risk-management",
    title: "Risk Management",
    category: "Management",
    icon: Shield,
    shortDesc: "Configure VaR, limits, and safeguards",
    fullContent: (
      <div className="space-y-4">
        <p className="text-gray-300 leading-relaxed">
          Risk Management settings protect your portfolio from excessive losses 
          and ensure AI trading stays within your comfort zone.
        </p>
        
        <h4 className="text-sm font-medium text-white mt-4">Key Limits</h4>
        <div className="space-y-2">
          <div className="p-3 rounded-lg bg-gray-800/50 border-l-4 border-rose-500">
            <h5 className="text-sm font-medium text-white">Max Position Size</h5>
            <p className="text-xs text-gray-400 mt-1">
              Maximum % of portfolio in any single position (default: 20%)
            </p>
          </div>
          <div className="p-3 rounded-lg bg-gray-800/50 border-l-4 border-amber-500">
            <h5 className="text-sm font-medium text-white">Daily Loss Limit</h5>
            <p className="text-xs text-gray-400 mt-1">
              Trading stops when daily loss reaches this threshold
            </p>
          </div>
          <div className="p-3 rounded-lg bg-gray-800/50 border-l-4 border-blue-500">
            <h5 className="text-sm font-medium text-white">VaR Limit</h5>
            <p className="text-xs text-gray-400 mt-1">
              Value at Risk - maximum expected loss at 95% confidence
            </p>
          </div>
        </div>
      </div>
    ),
    relatedTopics: ["trading-modes", "positions"],
  },
  {
    id: "market-regime",
    title: "Market Regime Detection",
    category: "AI Trading",
    icon: Activity,
    shortDesc: "Understand current market conditions",
    fullContent: (
      <div className="space-y-4">
        <p className="text-gray-300 leading-relaxed">
          Market Regime Detection uses machine learning to classify current 
          market conditions and adjust trading strategies accordingly.
        </p>
        
        <h4 className="text-sm font-medium text-white mt-4">Regime Types</h4>
        <div className="grid grid-cols-2 gap-2">
          <div className="p-2 rounded bg-emerald-500/10 border border-emerald-500/20 text-center">
            <TrendingUp className="w-4 h-4 text-emerald-400 mx-auto mb-1" />
            <span className="text-xs text-emerald-400">Bull Market</span>
          </div>
          <div className="p-2 rounded bg-rose-500/10 border border-rose-500/20 text-center">
            <TrendingUp className="w-4 h-4 text-rose-400 mx-auto mb-1 rotate-180" />
            <span className="text-xs text-rose-400">Bear Market</span>
          </div>
          <div className="p-2 rounded bg-amber-500/10 border border-amber-500/20 text-center">
            <Activity className="w-4 h-4 text-amber-400 mx-auto mb-1" />
            <span className="text-xs text-amber-400">Volatile</span>
          </div>
          <div className="p-2 rounded bg-blue-500/10 border border-blue-500/20 text-center">
            <Minus className="w-4 h-4 text-blue-400 mx-auto mb-1" />
            <span className="text-xs text-blue-400">Sideways</span>
          </div>
        </div>
      </div>
    ),
    relatedTopics: ["dashboard", "trading-modes"],
  },
  {
    id: "shortcuts",
    title: "Keyboard Shortcuts",
    category: "System",
    icon: Keyboard,
    shortDesc: "Speed up your workflow",
    fullContent: (
      <div className="space-y-4">
        <p className="text-gray-300 leading-relaxed">
          Master these keyboard shortcuts to navigate Investor OS like a pro.
        </p>
        
        <h4 className="text-sm font-medium text-white mt-4">Navigation</h4>
        <div className="space-y-1">
          <ShortcutRow keys={["Cmd", "K"]} description="Open Command Palette" />
          <ShortcutRow keys={["?"]} description="Toggle Help Panel" />
          <ShortcutRow keys={["Esc"]} description="Close modals/panels" />
          <ShortcutRow keys={["G"]} description="Go to... (quick nav)" />
        </div>
        
        <h4 className="text-sm font-medium text-white mt-4">Actions</h4>
        <div className="space-y-1">
          <ShortcutRow keys={["C"]} description="Confirm proposal" />
          <ShortcutRow keys={["R"]} description="Reject proposal" />
          <ShortcutRow keys={["S"]} description="Sync data" />
          <ShortcutRow keys={["N"]} description="New trade" />
        </div>
        
        <h4 className="text-sm font-medium text-white mt-4">Views</h4>
        <div className="space-y-1">
          <ShortcutRow keys={["P"]} description="Portfolio" />
          <ShortcutRow keys={["D"]} description="Dashboard" />
          <ShortcutRow keys={["T"]} description="Trades" />
          <ShortcutRow keys={["J"]} description="Journal" />
        </div>
      </div>
    ),
  },
  {
    id: "notifications",
    title: "Notifications",
    category: "System",
    icon: Bell,
    shortDesc: "Stay informed about important events",
    fullContent: (
      <div className="space-y-4">
        <p className="text-gray-300 leading-relaxed">
          Notifications keep you updated on trade executions, AI proposals, 
          risk alerts, and system events.
        </p>
        
        <h4 className="text-sm font-medium text-white mt-4">Notification Types</h4>
        <div className="space-y-2">
          <div className="flex items-center gap-3 p-2 rounded bg-emerald-500/10">
            <CheckCircle2 className="w-4 h-4 text-emerald-400" />
            <div>
              <span className="text-sm text-emerald-400">Trade Executed</span>
              <p className="text-xs text-gray-500">Order fill confirmations</p>
            </div>
          </div>
          <div className="flex items-center gap-3 p-2 rounded bg-blue-500/10">
            <Sparkles className="w-4 h-4 text-blue-400" />
            <div>
              <span className="text-sm text-blue-400">New AI Proposal</span>
              <p className="text-xs text-gray-500">Trading opportunities found</p>
            </div>
          </div>
          <div className="flex items-center gap-3 p-2 rounded bg-amber-500/10">
            <AlertTriangle className="w-4 h-4 text-amber-400" />
            <div>
              <span className="text-sm text-amber-400">Risk Alert</span>
              <p className="text-xs text-gray-500">Limits approached</p>
            </div>
          </div>
        </div>
      </div>
    ),
  },
];

function ShortcutRow({ keys, description }: { keys: string[]; description: string }) {
  return (
    <div className="flex items-center justify-between py-1.5">
      <span className="text-sm text-gray-400">{description}</span>
      <div className="flex items-center gap-1">
        {keys.map((key, i) => (
          <React.Fragment key={i}>
            <kbd className="px-2 py-0.5 bg-gray-700 rounded text-xs text-gray-200 font-mono">
              {key}
            </kbd>
            {i < keys.length - 1 && <span className="text-gray-600">+</span>}
          </React.Fragment>
        ))}
      </div>
    </div>
  );
}

import { Minus } from "lucide-react";

// ============================================================================
// MAIN HELP PANEL COMPONENT
// ============================================================================

export function HelpPanel() {
  const { isOpen, setIsOpen, currentTopic, setCurrentTopic, activeContext } = useHelp();
  const [searchQuery, setSearchQuery] = useState("");
  const [activeCategory, setActiveCategory] = useState("All");

  // Auto-select topic based on context
  useEffect(() => {
    if (!currentTopic && activeContext) {
      const topic = helpTopics.find((t) => t.id === activeContext);
      if (topic) {
        setCurrentTopic(topic);
      }
    }
  }, [activeContext, currentTopic, setCurrentTopic]);

  const categories = ["All", ...Array.from(new Set(helpTopics.map((t) => t.category)))];

  const filteredTopics = helpTopics.filter((topic) => {
    const matchesSearch =
      topic.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
      topic.shortDesc.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesCategory = activeCategory === "All" || topic.category === activeCategory;
    return matchesSearch && matchesCategory;
  });

  return (
    <>
      {/* Toggle Button */}
      <motion.button
        initial={{ opacity: 0, x: 20 }}
        animate={{ opacity: 1, x: 0 }}
        onClick={() => setIsOpen(!isOpen)}
        className={`fixed right-4 bottom-4 z-40 p-3 rounded-xl shadow-lg transition-colors ${
          isOpen
            ? "bg-rose-500 hover:bg-rose-600 text-white"
            : "bg-blue-600 hover:bg-blue-500 text-white"
        }`}
      >
        {isOpen ? <X className="w-5 h-5" /> : <HelpCircle className="w-5 h-5" />}
      </motion.button>

      {/* Help Panel */}
      <AnimatePresence>
        {isOpen && (
          <motion.div
            initial={{ opacity: 0, x: 300 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: 300 }}
            transition={{ type: "spring", damping: 25, stiffness: 200 }}
            className="fixed right-0 top-0 h-screen w-96 bg-gradient-to-b from-[#0a0f1c] via-[#111827] to-[#0a0f1c] 
              border-l border-gray-800/50 shadow-2xl z-30 flex flex-col"
          >
            {/* Header */}
            <div className="p-4 border-b border-gray-800/50">
              <div className="flex items-center justify-between mb-4">
                <div className="flex items-center gap-2">
                  <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-blue-600 to-purple-600 flex items-center justify-center">
                    <HelpCircle className="w-4 h-4 text-white" />
                  </div>
                  <div>
                    <h2 className="font-semibold text-white">Help Center</h2>
                    <p className="text-xs text-gray-500">Context-aware assistance</p>
                  </div>
                </div>
                <button
                  onClick={() => setIsOpen(false)}
                  className="p-2 hover:bg-gray-800 rounded-lg transition-colors"
                >
                  <X className="w-4 h-4 text-gray-400" />
                </button>
              </div>

              {/* Search */}
              <div className="relative">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-500" />
                <input
                  type="text"
                  placeholder="Search help topics..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="w-full pl-9 pr-4 py-2 bg-gray-800/50 border border-gray-700 rounded-lg
                    text-sm text-white placeholder-gray-500 focus:outline-none focus:border-blue-500/50"
                />
              </div>
            </div>

            {/* Content */}
            <div className="flex-1 overflow-hidden flex">
              {/* Topics List */}
              <div className="w-full flex flex-col">
                {/* Categories */}
                <div className="px-4 py-3 border-b border-gray-800/50">
                  <div className="flex gap-2 overflow-x-auto scrollbar-hide">
                    {categories.map((cat) => (
                      <button
                        key={cat}
                        onClick={() => setActiveCategory(cat)}
                        className={`px-3 py-1 rounded-full text-xs font-medium whitespace-nowrap transition-colors ${
                          activeCategory === cat
                            ? "bg-blue-600 text-white"
                            : "bg-gray-800 text-gray-400 hover:bg-gray-700"
                        }`}
                      >
                        {cat}
                      </button>
                    ))}
                  </div>
                </div>

                {/* Topics */}
                <div className="flex-1 overflow-y-auto p-4 space-y-2">
                  {filteredTopics.map((topic) => (
                    <TopicCard
                      key={topic.id}
                      topic={topic}
                      isActive={currentTopic?.id === topic.id}
                      onClick={() => setCurrentTopic(topic)}
                    />
                  ))}
                </div>
              </div>
            </div>

            {/* Current Topic Detail (Slide over) */}
            <AnimatePresence>
              {currentTopic && (
                <motion.div
                  initial={{ opacity: 0, y: 20 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: 20 }}
                  className="absolute inset-x-0 bottom-0 max-h-[70%] bg-gradient-to-b from-gray-900 to-[#0a0f1c] 
                    border-t border-gray-800/50 rounded-t-2xl shadow-2xl overflow-hidden flex flex-col"
                >
                  {/* Detail Header */}
                  <div className="p-4 border-b border-gray-800/50 flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-blue-500/20 to-purple-500/10 
                        flex items-center justify-center">
                        <currentTopic.icon className="w-5 h-5 text-blue-400" />
                      </div>
                      <div>
                        <h3 className="font-semibold text-white">{currentTopic.title}</h3>
                        <p className="text-xs text-gray-500">{currentTopic.category}</p>
                      </div>
                    </div>
                    <button
                      onClick={() => setCurrentTopic(null)}
                      className="p-2 hover:bg-gray-800 rounded-lg transition-colors"
                    >
                      <X className="w-4 h-4 text-gray-400" />
                    </button>
                  </div>

                  {/* Detail Content */}
                  <div className="flex-1 overflow-y-auto p-4">
                    {currentTopic.fullContent}

                    {/* Shortcuts */}
                    {currentTopic.shortcuts && currentTopic.shortcuts.length > 0 && (
                      <div className="mt-6 pt-4 border-t border-gray-800/50">
                        <h4 className="text-sm font-medium text-white mb-3 flex items-center gap-2">
                          <Keyboard className="w-4 h-4 text-gray-400" />
                          Related Shortcuts
                        </h4>
                        <div className="space-y-2">
                          {currentTopic.shortcuts.map((shortcut, i) => (
                            <div
                              key={i}
                              className="flex items-center justify-between py-1.5 px-3 rounded-lg bg-gray-800/30"
                            >
                              <span className="text-sm text-gray-400">{shortcut.description}</span>
                              <kbd className="px-2 py-0.5 bg-gray-700 rounded text-xs text-gray-200 font-mono">
                                {shortcut.key}
                              </kbd>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}

                    {/* Related Topics */}
                    {currentTopic.relatedTopics && currentTopic.relatedTopics.length > 0 && (
                      <div className="mt-6 pt-4 border-t border-gray-800/50">
                        <h4 className="text-sm font-medium text-white mb-3 flex items-center gap-2">
                          <BookOpen className="w-4 h-4 text-gray-400" />
                          Related Topics
                        </h4>
                        <div className="flex flex-wrap gap-2">
                          {currentTopic.relatedTopics.map((topicId) => {
                            const related = helpTopics.find((t) => t.id === topicId);
                            if (!related) return null;
                            return (
                              <button
                                key={topicId}
                                onClick={() => setCurrentTopic(related)}
                                className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-gray-800/50 
                                  hover:bg-gray-700 text-sm text-gray-300 transition-colors"
                              >
                                <related.icon className="w-3.5 h-3.5" />
                                {related.title}
                              </button>
                            );
                          })}
                        </div>
                      </div>
                    )}
                  </div>
                </motion.div>
              )}
            </AnimatePresence>

            {/* Footer */}
            <div className="p-3 border-t border-gray-800/50 bg-gray-900/30">
              <div className="flex items-center justify-between text-xs text-gray-500">
                <span>Press <kbd className="px-1.5 py-0.5 bg-gray-800 rounded">?</kbd> to toggle</span>
                <span>{helpTopics.length} topics</span>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </>
  );
}

// ============================================================================
// TOPIC CARD COMPONENT
// ============================================================================

function TopicCard({
  topic,
  isActive,
  onClick,
}: {
  topic: HelpTopic;
  isActive: boolean;
  onClick: () => void;
}) {
  return (
    <motion.button
      whileHover={{ scale: 1.02 }}
      whileTap={{ scale: 0.98 }}
      onClick={onClick}
      className={`w-full p-3 rounded-xl text-left transition-all ${
        isActive
          ? "bg-gradient-to-r from-blue-600/20 to-purple-600/10 border border-blue-500/30"
          : "bg-gray-800/30 hover:bg-gray-800/50 border border-transparent"
      }`}
    >
      <div className="flex items-start gap-3">
        <div
          className={`w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0 ${
            isActive ? "bg-blue-500/20" : "bg-gray-700/50"
          }`}
        >
          <topic.icon className={`w-4 h-4 ${isActive ? "text-blue-400" : "text-gray-400"}`} />
        </div>
        <div className="flex-1 min-w-0">
          <h4 className={`text-sm font-medium ${isActive ? "text-blue-400" : "text-gray-200"}`}>
            {topic.title}
          </h4>
          <p className="text-xs text-gray-500 mt-0.5 line-clamp-2">{topic.shortDesc}</p>
        </div>
        <ChevronRight className={`w-4 h-4 flex-shrink-0 ${isActive ? "text-blue-400" : "text-gray-600"}`} />
      </div>
    </motion.button>
  );
}

// ============================================================================
// HELPER COMPONENTS
// ============================================================================

export function HelpBadge({ topicId }: { topicId: string }) {
  const { setCurrentTopic, setIsOpen } = useHelp();
  const topic = helpTopics.find((t) => t.id === topicId);

  if (!topic) return null;

  return (
    <button
      onClick={() => {
        setCurrentTopic(topic);
        setIsOpen(true);
      }}
      className="inline-flex items-center gap-1.5 px-2 py-1 rounded-md bg-blue-500/10 
        hover:bg-blue-500/20 border border-blue-500/20 text-xs text-blue-400 transition-colors"
    >
      <HelpCircle className="w-3 h-3" />
      Help
    </button>
  );
}

export function InlineHelp({ children, topicId }: { children: ReactNode; topicId: string }) {
  const { setCurrentTopic, setIsOpen, showTooltip, hideTooltip } = useHelp();
  const topic = helpTopics.find((t) => t.id === topicId);

  if (!topic) return <>{children}</>;

  return (
    <span
      className="inline-flex items-center gap-1 cursor-help group"
      onMouseEnter={(e) => showTooltip(topic.shortDesc, e.currentTarget.getBoundingClientRect())}
      onMouseLeave={hideTooltip}
      onClick={() => {
        setCurrentTopic(topic);
        setIsOpen(true);
      }}
    >
      {children}
      <HelpCircle className="w-3.5 h-3.5 text-blue-400 opacity-0 group-hover:opacity-100 transition-opacity" />
    </span>
  );
}
