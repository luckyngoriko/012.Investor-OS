"use client";

import { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  User,
  UserCog,
  Bot,
  AlertTriangle,
  CheckCircle2,
  ChevronDown,
  ChevronUp,
  Settings,
  Bell,
  Shield,
  Info,
  X,
  Sparkles,
  BarChart3,
  Clock,
} from "lucide-react";

// ============================================
// TRADING MODE TYPES
// ============================================

export type TradingMode = "disabled" | "manual" | "semi_auto" | "fully_auto";

export interface TradingModeConfig {
  mode: TradingMode;
  auto_execute_cq_threshold: number;
  max_auto_trade_value: number;
  notifications: {
    on_proposal: boolean;
    on_execution: boolean;
    on_rejection: boolean;
    on_risk_alert: boolean;
    email_enabled: boolean;
    push_enabled: boolean;
  };
}

export const DEFAULT_MODE_CONFIG: TradingModeConfig = {
  mode: "semi_auto",
  auto_execute_cq_threshold: 80,
  max_auto_trade_value: 10000,
  notifications: {
    on_proposal: true,
    on_execution: true,
    on_rejection: false,
    on_risk_alert: true,
    email_enabled: true,
    push_enabled: true,
  },
};

// ============================================
// MODE DEFINITIONS
// ============================================

export const TRADING_MODES: Record<TradingMode, {
  id: TradingMode;
  name: string;
  shortName: string;
  description: string;
  detailedDescription: string;
  icon: React.ElementType;
  color: string;
  bgColor: string;
  borderColor: string;
  autonomyLevel: number;
  recommendedFor: string;
  features: string[];
}> = {
  disabled: {
    id: "disabled",
    name: "Trading Disabled",
    shortName: "OFF",
    description: "All trading is disabled. View-only mode.",
    detailedDescription: "Trading is completely disabled. You can view portfolio and analytics but no trades will be proposed or executed. Use this for maintenance or when you want to pause all activity.",
    icon: AlertTriangle,
    color: "text-red-400",
    bgColor: "bg-red-500/10",
    borderColor: "border-red-500/30",
    autonomyLevel: 0,
    recommendedFor: "Maintenance, vacation, risk mitigation",
    features: [
      "View portfolio and positions",
      "AI analysis continues in background",
      "No trades proposed or executed",
      "Manual reactivation required",
      "All alerts and monitoring active",
    ],
  },
  manual: {
    id: "manual",
    name: "Manual Mode",
    shortName: "Manual",
    description: "You make all trading decisions. AI provides analysis only.",
    detailedDescription: "In Manual mode, AI analyzes market data and generates proposals with CQ scores, but you must manually execute all trades through your broker. Best for learning and maximum control.",
    icon: User,
    color: "text-blue-400",
    bgColor: "bg-blue-500/10",
    borderColor: "border-blue-500/30",
    autonomyLevel: 1,
    recommendedFor: "Beginners, learning, maximum control",
    features: [
      "AI generates trade proposals with CQ scores",
      "You manually review all proposals",
      "You execute trades through your broker",
      "AI tracks performance and provides insights",
      "Full control over every decision",
    ],
  },
  semi_auto: {
    id: "semi_auto",
    name: "Semi-Automatic",
    shortName: "Semi-Auto",
    description: "AI proposes trades, you confirm. AI executes confirmed trades.",
    detailedDescription: "In Semi-Auto mode, AI generates proposals and waits for your confirmation. Once you confirm, AI automatically executes the trade. Perfect balance of AI power and human oversight.",
    icon: UserCog,
    color: "text-amber-400",
    bgColor: "bg-amber-500/10",
    borderColor: "border-amber-500/30",
    autonomyLevel: 2,
    recommendedFor: "Most users, balanced approach",
    features: [
      "AI generates trade proposals with CQ scores",
      "You confirm or reject each proposal",
      "AI auto-executes confirmed trades",
      "Configurable confirmation timeouts",
      "Notifications for new proposals",
    ],
  },
  fully_auto: {
    id: "fully_auto",
    name: "Fully Automatic",
    shortName: "Auto",
    description: "AI trades automatically within your risk limits.",
    detailedDescription: "In Fully Auto mode, AI continuously analyzes markets and executes trades automatically when conditions are met. All trades respect your risk limits. Kill switch available for emergencies.",
    icon: Bot,
    color: "text-emerald-400",
    bgColor: "bg-emerald-500/10",
    borderColor: "border-emerald-500/30",
    autonomyLevel: 3,
    recommendedFor: "Experienced users, hands-off trading",
    features: [
      "AI continuously monitors markets",
      "Auto-executes when CQ >= threshold",
      "Respects all risk limits",
      "Real-time notifications of trades",
      "Emergency kill switch available",
    ],
  },
};

// ============================================
// TRADING MODE INDICATOR (Header Badge)
// ============================================

interface TradingModeIndicatorProps {
  mode: TradingMode;
  onClick?: () => void;
  showLabel?: boolean;
}

export function TradingModeIndicator({ mode, onClick, showLabel = true }: TradingModeIndicatorProps) {
  const modeConfig = TRADING_MODES[mode];
  const Icon = modeConfig.icon;

  return (
    <motion.button
      onClick={onClick}
      whileHover={{ scale: 1.02 }}
      whileTap={{ scale: 0.98 }}
      className={`flex items-center gap-2 px-3 py-1.5 rounded-lg ${modeConfig.bgColor} 
        ${modeConfig.borderColor} border backdrop-blur-sm transition-all
        ${onClick ? "cursor-pointer hover:brightness-110" : "cursor-default"}`}
    >
      <Icon className={`w-4 h-4 ${modeConfig.color}`} />
      {showLabel && (
        <>
          <span className={`text-sm font-medium ${modeConfig.color}`}>
            {modeConfig.shortName}
          </span>
          {onClick && <ChevronDown className={`w-3 h-3 ${modeConfig.color}`} />}
        </>
      )}
    </motion.button>
  );
}

// ============================================
// TRADING MODE SELECTOR (Dropdown)
// ============================================

interface TradingModeSelectorProps {
  currentMode: TradingMode;
  onModeChange: (mode: TradingMode) => void;
  disabled?: boolean;
}

export function TradingModeSelector({ currentMode, onModeChange, disabled }: TradingModeSelectorProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [showConfirmation, setShowConfirmation] = useState<TradingMode | null>(null);

  const handleModeSelect = (mode: TradingMode) => {
    if (mode === currentMode) {
      setIsOpen(false);
      return;
    }
    
    // Show confirmation for Fully Auto (high risk)
    if (mode === "fully_auto") {
      setShowConfirmation(mode);
      setIsOpen(false);
      return;
    }
    
    onModeChange(mode);
    setIsOpen(false);
  };

  const confirmModeChange = () => {
    if (showConfirmation) {
      onModeChange(showConfirmation);
      setShowConfirmation(null);
    }
  };

  const currentConfig = TRADING_MODES[currentMode];
  const CurrentIcon = currentConfig.icon;

  return (
    <>
      <div className="relative">
        <motion.button
          onClick={() => !disabled && setIsOpen(!isOpen)}
          disabled={disabled}
          whileHover={!disabled ? { scale: 1.01 } : {}}
          whileTap={!disabled ? { scale: 0.99 } : {}}
          className={`w-full flex items-center justify-between p-4 rounded-xl 
            ${currentConfig.bgColor} ${currentConfig.borderColor} border
            transition-all ${disabled ? "opacity-50 cursor-not-allowed" : "cursor-pointer hover:brightness-110"}`}
        >
          <div className="flex items-center gap-3">
            <div className={`w-10 h-10 rounded-lg flex items-center justify-center ${currentConfig.bgColor}`}>
              <CurrentIcon className={`w-5 h-5 ${currentConfig.color}`} />
            </div>
            <div className="text-left">
              <p className={`font-semibold ${currentConfig.color}`}>{currentConfig.name}</p>
              <p className="text-xs text-gray-400">{currentConfig.description}</p>
            </div>
          </div>
          <ChevronDown className={`w-5 h-5 ${currentConfig.color} transition-transform ${isOpen ? "rotate-180" : ""}`} />
        </motion.button>

        <AnimatePresence>
          {isOpen && (
            <motion.div
              initial={{ opacity: 0, y: -10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              className="absolute top-full left-0 right-0 mt-2 glass-card rounded-xl overflow-hidden z-50"
            >
              {(Object.keys(TRADING_MODES) as TradingMode[]).map((mode) => {
                const config = TRADING_MODES[mode];
                const Icon = config.icon;
                const isActive = mode === currentMode;

                return (
                  <button
                    key={mode}
                    onClick={() => handleModeSelect(mode)}
                    className={`w-full flex items-center gap-3 p-4 text-left transition-colors
                      ${isActive ? `${config.bgColor} ${config.color}` : "hover:bg-gray-800/50 text-gray-300"}
                      ${mode !== "manual" ? "border-t border-gray-800" : ""}`}
                  >
                    <Icon className={`w-5 h-5 ${isActive ? config.color : "text-gray-400"}`} />
                    <div className="flex-1">
                      <p className={`font-medium ${isActive ? "text-white" : ""}`}>{config.name}</p>
                      <p className="text-xs text-gray-500">{config.recommendedFor}</p>
                    </div>
                    {isActive && <CheckCircle2 className={`w-5 h-5 ${config.color}`} />}
                  </button>
                );
              })}
            </motion.div>
          )}
        </AnimatePresence>
      </div>

      {/* Confirmation Modal for Fully Auto */}
      <AnimatePresence>
        {showConfirmation && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm"
            onClick={() => setShowConfirmation(null)}
          >
            <motion.div
              initial={{ opacity: 0, scale: 0.9 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.9 }}
              onClick={(e) => e.stopPropagation()}
              className="w-full max-w-md glass-card rounded-2xl p-6"
            >
              <div className="flex items-center gap-3 mb-4">
                <div className="w-12 h-12 rounded-xl bg-amber-500/20 flex items-center justify-center">
                  <AlertTriangle className="w-6 h-6 text-amber-400" />
                </div>
                <div>
                  <h3 className="text-lg font-bold text-white">Enable Fully Auto?</h3>
                  <p className="text-sm text-gray-400">High autonomy trading mode</p>
                </div>
              </div>

              <div className="p-4 rounded-xl bg-amber-500/10 border border-amber-500/20 mb-6">
                <p className="text-sm text-amber-200 mb-2">
                  In Fully Auto mode, AI will:
                </p>
                <ul className="text-sm text-amber-200/80 space-y-1 list-disc list-inside">
                  <li>Execute trades automatically without confirmation</li>
                  <li>Use your configured CQ threshold ({DEFAULT_MODE_CONFIG.auto_execute_cq_threshold}%)</li>
                  <li>Respect max trade value (${DEFAULT_MODE_CONFIG.max_auto_trade_value.toLocaleString()})</li>
                </ul>
              </div>

              <div className="flex gap-3">
                <button
                  onClick={() => setShowConfirmation(null)}
                  className="flex-1 py-2.5 bg-gray-700 hover:bg-gray-600 text-white rounded-lg transition-colors"
                >
                  Cancel
                </button>
                <button
                  onClick={confirmModeChange}
                  className="flex-1 py-2.5 bg-emerald-600 hover:bg-emerald-500 text-white rounded-lg transition-colors"
                >
                  Enable Auto
                </button>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </>
  );
}

// ============================================
// TRADING MODE CARD (For Settings/Info)
// ============================================

interface TradingModeCardProps {
  mode: TradingMode;
  isActive?: boolean;
  onSelect?: (mode: TradingMode) => void;
}

export function TradingModeCard({ mode, isActive, onSelect }: TradingModeCardProps) {
  const config = TRADING_MODES[mode];
  const Icon = config.icon;

  return (
    <motion.div
      onClick={() => onSelect?.(mode)}
      whileHover={onSelect ? { scale: 1.01 } : {}}
      className={`relative p-6 rounded-2xl border transition-all
        ${isActive 
          ? `${config.bgColor} ${config.borderColor} ring-1 ring-opacity-50` 
          : "bg-gray-800/30 border-gray-700/50 hover:border-gray-600"
        }
        ${onSelect ? "cursor-pointer" : ""}`}
    >
      {isActive && (
        <div className={`absolute top-4 right-4 w-6 h-6 rounded-full ${config.bgColor} flex items-center justify-center`}>
          <CheckCircle2 className={`w-4 h-4 ${config.color}`} />
        </div>
      )}

      <div className={`w-14 h-14 rounded-xl ${config.bgColor} flex items-center justify-center mb-4`}>
        <Icon className={`w-7 h-7 ${config.color}`} />
      </div>

      <h3 className={`text-lg font-bold mb-2 ${isActive ? "text-white" : "text-gray-200"}`}>
        {config.name}
      </h3>
      
      <p className="text-sm text-gray-400 mb-4">{config.description}</p>

      <div className="space-y-2">
        {config.features.slice(0, 3).map((feature, idx) => (
          <div key={idx} className="flex items-start gap-2 text-sm">
            <Sparkles className={`w-4 h-4 mt-0.5 flex-shrink-0 ${config.color}`} />
            <span className="text-gray-300">{feature}</span>
          </div>
        ))}
      </div>

      <div className={`mt-4 pt-4 border-t ${isActive ? config.borderColor : "border-gray-700"}`}>
        <p className="text-xs text-gray-500">
          <span className="font-medium">Best for:</span> {config.recommendedFor}
        </p>
      </div>
    </motion.div>
  );
}

// ============================================
// MODE SETTINGS PANEL
// ============================================

interface ModeSettingsPanelProps {
  config: TradingModeConfig;
  onConfigChange: (config: TradingModeConfig) => void;
}

export function ModeSettingsPanel({ config, onConfigChange }: ModeSettingsPanelProps) {
  const modeConfig = TRADING_MODES[config.mode];

  const updateConfig = (updates: Partial<TradingModeConfig>) => {
    onConfigChange({ ...config, ...updates });
  };

  const updateNotifications = (updates: Partial<TradingModeConfig["notifications"]>) => {
    onConfigChange({
      ...config,
      notifications: { ...config.notifications, ...updates },
    });
  };

  return (
    <div className="space-y-6">
      {/* Mode Info */}
      <div className={`p-4 rounded-xl ${modeConfig.bgColor} ${modeConfig.borderColor} border`}>
        <div className="flex items-center gap-3 mb-2">
          <modeConfig.icon className={`w-5 h-5 ${modeConfig.color}`} />
          <span className={`font-semibold ${modeConfig.color}`}>{modeConfig.name}</span>
        </div>
        <p className="text-sm text-gray-300">{modeConfig.detailedDescription}</p>
      </div>

      {/* Auto-Execute Settings (for Semi and Full Auto) */}
      {config.mode !== "manual" && (
        <div className="space-y-4">
          <h4 className="text-sm font-semibold text-gray-400 uppercase tracking-wider flex items-center gap-2">
            <Settings className="w-4 h-4" />
            Auto-Execution Settings
          </h4>

          <div className="glass-card rounded-xl p-4 space-y-4">
            {/* CQ Threshold */}
            <div>
              <div className="flex items-center justify-between mb-2">
                <label className="text-sm text-gray-300">CQ Threshold for Auto-Execution</label>
                <span className={`text-sm font-bold ${modeConfig.color}`}>{config.auto_execute_cq_threshold}%</span>
              </div>
              <input
                type="range"
                min="50"
                max="100"
                value={config.auto_execute_cq_threshold}
                onChange={(e) => updateConfig({ auto_execute_cq_threshold: parseInt(e.target.value) })}
                className="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-blue-500"
              />
              <p className="text-xs text-gray-500 mt-1">
                Trades with CQ below this threshold will require manual confirmation
              </p>
            </div>

            {/* Max Trade Value */}
            <div>
              <div className="flex items-center justify-between mb-2">
                <label className="text-sm text-gray-300">Max Auto Trade Value</label>
                <span className={`text-sm font-bold ${modeConfig.color}`}>
                  ${config.max_auto_trade_value.toLocaleString()}
                </span>
              </div>
              <input
                type="range"
                min="1000"
                max="100000"
                step="1000"
                value={config.max_auto_trade_value}
                onChange={(e) => updateConfig({ max_auto_trade_value: parseInt(e.target.value) })}
                className="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-blue-500"
              />
              <p className="text-xs text-gray-500 mt-1">
                Trades above this value will require manual confirmation
              </p>
            </div>
          </div>
        </div>
      )}

      {/* Notifications */}
      <div className="space-y-4">
        <h4 className="text-sm font-semibold text-gray-400 uppercase tracking-wider flex items-center gap-2">
          <Bell className="w-4 h-4" />
          Notifications
        </h4>

        <div className="glass-card rounded-xl p-4 space-y-3">
          {[
            { key: "on_proposal", label: "New trade proposals", description: "Get notified when AI generates a new proposal" },
            { key: "on_execution", label: "Trade execution", description: "Get notified when a trade is executed" },
            { key: "on_rejection", label: "Proposal rejections", description: "Get notified when AI rejects a proposal" },
            { key: "on_risk_alert", label: "Risk alerts", description: "Get notified about risk limit breaches" },
          ].map(({ key, label, description }) => (
            <label key={key} className="flex items-start gap-3 cursor-pointer group">
              <input
                type="checkbox"
                checked={config.notifications[key as keyof typeof config.notifications] as boolean}
                onChange={(e) => updateNotifications({ [key]: e.target.checked })}
                className="mt-1 w-4 h-4 rounded border-gray-600 bg-gray-700 text-blue-600 focus:ring-blue-500"
              />
              <div className="flex-1">
                <p className="text-sm text-gray-300 group-hover:text-white transition-colors">{label}</p>
                <p className="text-xs text-gray-500">{description}</p>
              </div>
            </label>
          ))}
        </div>
      </div>

      {/* Mode Comparison */}
      <div className="pt-4 border-t border-gray-800">
        <button
          onClick={() => {}}
          className="text-sm text-blue-400 hover:text-blue-300 flex items-center gap-2 transition-colors"
        >
          <Info className="w-4 h-4" />
          Compare all modes
        </button>
      </div>
    </div>
  );
}

// ============================================
// TRADING MODE WIZARD (For First-Time Setup)
// ============================================

interface TradingModeWizardProps {
  isOpen: boolean;
  onClose: () => void;
  onComplete: (config: TradingModeConfig) => void;
}

export function TradingModeWizard({ isOpen, onClose, onComplete }: TradingModeWizardProps) {
  const [selectedMode, setSelectedMode] = useState<TradingMode>("semi_auto");
  const [step, setStep] = useState<"select" | "configure" | "confirm">("select");
  const [config, setConfig] = useState<TradingModeConfig>(DEFAULT_MODE_CONFIG);

  if (!isOpen) return null;

  return (
    <AnimatePresence>
      <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-md">
        <motion.div
          initial={{ opacity: 0, scale: 0.95 }}
          animate={{ opacity: 1, scale: 1 }}
          exit={{ opacity: 0, scale: 0.95 }}
          className="w-full max-w-4xl glass-card rounded-3xl overflow-hidden"
        >
          {/* Header */}
          <div className="p-6 border-b border-gray-800 flex items-center justify-between">
            <div>
              <h2 className="text-2xl font-bold text-white">Choose Trading Mode</h2>
              <p className="text-gray-400">Select how you want Investor OS to trade</p>
            </div>
            <button onClick={onClose} className="p-2 hover:bg-gray-800 rounded-lg transition-colors">
              <X className="w-5 h-5 text-gray-400" />
            </button>
          </div>

          {/* Content */}
          <div className="p-6">
            {step === "select" && (
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                {(Object.keys(TRADING_MODES) as TradingMode[]).map((mode) => (
                  <TradingModeCard
                    key={mode}
                    mode={mode}
                    isActive={selectedMode === mode}
                    onSelect={setSelectedMode}
                  />
                ))}
              </div>
            )}

            {step === "configure" && (
              <ModeSettingsPanel
                config={{ ...config, mode: selectedMode }}
                onConfigChange={(newConfig) => {
                  setConfig(newConfig);
                  setSelectedMode(newConfig.mode);
                }}
              />
            )}
          </div>

          {/* Footer */}
          <div className="p-6 border-t border-gray-800 flex items-center justify-between">
            <button
              onClick={() => step === "configure" ? setStep("select") : onClose()}
              className="px-6 py-2.5 text-gray-400 hover:text-white transition-colors"
            >
              {step === "select" ? "Cancel" : "Back"}
            </button>
            
            <div className="flex gap-3">
              {step === "select" ? (
                <button
                  onClick={() => setStep("configure")}
                  className="px-6 py-2.5 bg-blue-600 hover:bg-blue-500 text-white rounded-lg transition-colors"
                >
                  Continue
                </button>
              ) : (
                <button
                  onClick={() => onComplete({ ...config, mode: selectedMode })}
                  className="px-6 py-2.5 bg-emerald-600 hover:bg-emerald-500 text-white rounded-lg transition-colors"
                >
                  Start Trading
                </button>
              )}
            </div>
          </div>
        </motion.div>
      </div>
    </AnimatePresence>
  );
}

// ============================================
// MODE STATUS BAR (For Dashboard)
// ============================================

interface ModeStatusBarProps {
  mode: TradingMode;
  pendingProposals?: number;
  onViewProposals?: () => void;
  onChangeMode?: () => void;
}

export function ModeStatusBar({ mode, pendingProposals = 0, onViewProposals, onChangeMode }: ModeStatusBarProps) {
  const config = TRADING_MODES[mode];
  const Icon = config.icon;

  return (
    <div className={`p-4 rounded-xl ${config.bgColor} ${config.borderColor} border`}>
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <div className={`w-12 h-12 rounded-xl ${config.bgColor} flex items-center justify-center`}>
            <Icon className={`w-6 h-6 ${config.color}`} />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <span className={`font-bold ${config.color}`}>{config.name}</span>
              <span className="text-xs text-gray-500">Active</span>
            </div>
            <p className="text-sm text-gray-400">{config.description}</p>
          </div>
        </div>

        <div className="flex items-center gap-3">
          {mode !== "manual" && pendingProposals > 0 && (
            <button
              onClick={onViewProposals}
              className="flex items-center gap-2 px-4 py-2 bg-gray-800 hover:bg-gray-700 
                text-white text-sm font-medium rounded-lg transition-colors"
            >
              <Bell className="w-4 h-4" />
              {pendingProposals} Pending
            </button>
          )}
          
          <button
            onClick={onChangeMode}
            className="flex items-center gap-2 px-4 py-2 bg-gray-800 hover:bg-gray-700 
              text-gray-300 hover:text-white text-sm font-medium rounded-lg transition-colors"
          >
            <Settings className="w-4 h-4" />
            Change Mode
          </button>
        </div>
      </div>
    </div>
  );
}

// ============================================
// EMERGENCY KILL SWITCH
// ============================================

interface KillSwitchProps {
  onActivate: () => void;
  disabled?: boolean;
}

export function KillSwitch({ onActivate, disabled }: KillSwitchProps) {
  const [isConfirming, setIsConfirming] = useState(false);
  const [countdown, setCountdown] = useState(3);

  useEffect(() => {
    if (isConfirming && countdown > 0) {
      const timer = setTimeout(() => setCountdown(c => c - 1), 1000);
      return () => clearTimeout(timer);
    }
  }, [isConfirming, countdown]);

  const handleActivate = () => {
    if (!isConfirming) {
      setIsConfirming(true);
      setCountdown(3);
      return;
    }
    if (countdown === 0) {
      onActivate();
      setIsConfirming(false);
      setCountdown(3);
    }
  };

  const handleCancel = () => {
    setIsConfirming(false);
    setCountdown(3);
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      className="relative"
    >
      <button
        onClick={handleActivate}
        disabled={disabled}
        className={`
          relative flex items-center gap-2 px-4 py-2 rounded-lg font-semibold text-sm
          transition-all duration-200
          ${isConfirming 
            ? "bg-red-600 text-white animate-pulse" 
            : "bg-red-500/20 text-red-400 border border-red-500/50 hover:bg-red-500/30"
          }
          ${disabled ? "opacity-50 cursor-not-allowed" : "cursor-pointer"}
        `}
      >
        <AlertTriangle className="w-4 h-4" />
        {isConfirming 
          ? `CONFIRM STOP${countdown > 0 ? ` (${countdown})` : ""}` 
          : "KILL SWITCH"
        }
      </button>
      
      {isConfirming && (
        <motion.button
          initial={{ opacity: 0, x: 10 }}
          animate={{ opacity: 1, x: 0 }}
          onClick={handleCancel}
          className="absolute left-full ml-2 px-3 py-2 bg-gray-700 text-gray-300 
            text-sm rounded-lg hover:bg-gray-600 transition-colors"
        >
          Cancel
        </motion.button>
      )}
    </motion.div>
  );
}

// ============================================
// CIRCUIT BREAKER PANEL
// ============================================

interface CircuitBreaker {
  id: string;
  name: string;
  enabled: boolean;
  threshold: number;
  currentValue: number;
  triggered: boolean;
  lastTriggered?: Date;
}

interface CircuitBreakerPanelProps {
  breakers: CircuitBreaker[];
  onToggle: (id: string) => void;
  onUpdateThreshold: (id: string, value: number) => void;
}

export function CircuitBreakerPanel({ 
  breakers, 
  onToggle, 
  onUpdateThreshold 
}: CircuitBreakerPanelProps) {
  return (
    <div className="bg-gray-900/50 border border-gray-800 rounded-xl p-4">
      <div className="flex items-center gap-2 mb-4">
        <div className="w-8 h-8 rounded-lg bg-orange-500/10 flex items-center justify-center">
          <Shield className="w-4 h-4 text-orange-400" />
        </div>
        <div>
          <h3 className="text-sm font-semibold text-white">Circuit Breakers</h3>
          <p className="text-xs text-gray-500">Auto-stop triggers</p>
        </div>
      </div>

      <div className="space-y-3">
        {breakers.map((breaker) => (
          <div 
            key={breaker.id}
            className={`
              p-3 rounded-lg border transition-all
              ${breaker.triggered 
                ? "bg-red-500/10 border-red-500/50" 
                : "bg-gray-800/30 border-gray-700/50"
              }
            `}
          >
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2">
                <span className={`text-sm font-medium ${breaker.triggered ? "text-red-400" : "text-gray-300"}`}>
                  {breaker.name}
                </span>
                {breaker.triggered && (
                  <span className="px-2 py-0.5 bg-red-500 text-white text-xs rounded-full">
                    TRIGGERED
                  </span>
                )}
              </div>
              <label className="relative inline-flex items-center cursor-pointer">
                <input
                  type="checkbox"
                  checked={breaker.enabled}
                  onChange={() => onToggle(breaker.id)}
                  className="sr-only peer"
                />
                <div className="w-9 h-5 bg-gray-700 peer-focus:outline-none rounded-full peer 
                  peer-checked:after:translate-x-full peer-checked:after:border-white 
                  after:content-[''] after:absolute after:top-[2px] after:left-[2px] 
                  after:bg-white after:border-gray-300 after:border after:rounded-full 
                  after:h-4 after:w-4 after:transition-all peer-checked:bg-blue-600">
                </div>
              </label>
            </div>
            
            <div className="flex items-center gap-3">
              <div className="flex-1">
                <div className="flex justify-between text-xs mb-1">
                  <span className="text-gray-500">Limit</span>
                  <span className={breaker.triggered ? "text-red-400" : "text-gray-400"}>
                    {breaker.threshold}%
                  </span>
                </div>
                <input
                  type="range"
                  min="1"
                  max="50"
                  value={breaker.threshold}
                  onChange={(e) => onUpdateThreshold(breaker.id, parseInt(e.target.value))}
                  disabled={!breaker.enabled}
                  className="w-full h-1.5 bg-gray-700 rounded-lg appearance-none cursor-pointer
                    accent-blue-500 disabled:opacity-50"
                />
              </div>
              <div className="text-right">
                <span className={`text-lg font-bold ${breaker.triggered ? "text-red-400" : "text-gray-300"}`}>
                  {breaker.currentValue.toFixed(1)}%
                </span>
              </div>
            </div>
            
            {breaker.lastTriggered && (
              <p className="text-xs text-red-400 mt-2">
                Last triggered: {breaker.lastTriggered.toLocaleString()}
              </p>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

// ============================================
// PAPER TRADING TOGGLE
// ============================================

interface PaperTradingToggleProps {
  isPaperTrading: boolean;
  onToggle: (value: boolean) => void;
  virtualBalance: number;
  virtualPnL: number;
}

export function PaperTradingToggle({ 
  isPaperTrading, 
  onToggle, 
  virtualBalance,
  virtualPnL 
}: PaperTradingToggleProps) {
  return (
    <motion.div
      whileHover={{ scale: 1.02 }}
      className={`
        relative flex items-center gap-3 p-3 rounded-xl border cursor-pointer
        transition-all duration-200
        ${isPaperTrading 
          ? "bg-emerald-500/10 border-emerald-500/50" 
          : "bg-gray-800/50 border-gray-700/50 hover:border-gray-600"
        }
      `}
      onClick={() => onToggle(!isPaperTrading)}
    >
      <div className={`
        w-10 h-10 rounded-lg flex items-center justify-center
        ${isPaperTrading ? "bg-emerald-500/20" : "bg-gray-700/50"}
      `}>
        <span className={`text-lg ${isPaperTrading ? "text-emerald-400" : "text-gray-400"}`}>
          {isPaperTrading ? "📝" : "💰"}
        </span>
      </div>
      
      <div className="flex-1">
        <div className="flex items-center gap-2">
          <span className={`font-medium ${isPaperTrading ? "text-emerald-400" : "text-gray-300"}`}>
            {isPaperTrading ? "Paper Trading" : "Live Trading"}
          </span>
          <span className={`
            px-2 py-0.5 text-xs rounded-full
            ${isPaperTrading ? "bg-emerald-500/20 text-emerald-400" : "bg-amber-500/20 text-amber-400"}
          `}>
            {isPaperTrading ? "SIMULATION" : "REAL MONEY"}
          </span>
        </div>
        <p className="text-xs text-gray-500">
          {isPaperTrading 
            ? `Virtual: $${virtualBalance.toLocaleString()} | P&L: ${virtualPnL >= 0 ? "+" : ""}${virtualPnL.toFixed(2)}%`
            : "Real money at risk - Use with caution"
          }
        </p>
      </div>
      
      <div className={`
        w-12 h-6 rounded-full relative transition-colors
        ${isPaperTrading ? "bg-emerald-500" : "bg-gray-600"}
      `}>
        <motion.div
          animate={{ x: isPaperTrading ? 24 : 2 }}
          className="absolute top-1 w-4 h-4 bg-white rounded-full"
        />
      </div>
    </motion.div>
  );
}

// ============================================
// POSITION SIZE CALCULATOR
// ============================================

interface PositionSizeCalculatorProps {
  accountBalance: number;
  riskPercent: number;
  entryPrice: number;
  stopLoss: number;
  onCalculate?: (size: number) => void;
}

export function PositionSizeCalculator({
  accountBalance,
  riskPercent,
  entryPrice,
  stopLoss,
  onCalculate,
}: PositionSizeCalculatorProps) {
  const riskAmount = accountBalance * (riskPercent / 100);
  const riskPerShare = Math.abs(entryPrice - stopLoss);
  const positionSize = riskPerShare > 0 ? Math.floor(riskAmount / riskPerShare) : 0;
  const totalValue = positionSize * entryPrice;
  const portfolioPercent = (totalValue / accountBalance) * 100;

  useEffect(() => {
    onCalculate?.(positionSize);
  }, [positionSize, onCalculate]);

  return (
    <div className="bg-gray-900/50 border border-gray-800 rounded-xl p-4">
      <div className="flex items-center gap-2 mb-4">
        <div className="w-8 h-8 rounded-lg bg-blue-500/10 flex items-center justify-center">
          <span className="text-blue-400 font-bold">%</span>
        </div>
        <div>
          <h3 className="text-sm font-semibold text-white">Position Sizing</h3>
          <p className="text-xs text-gray-500">Risk-based calculator</p>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-3 mb-4">
        <div className="bg-gray-800/50 p-3 rounded-lg">
          <p className="text-xs text-gray-500 mb-1">Risk Amount</p>
          <p className="text-sm font-medium text-white">${riskAmount.toFixed(2)}</p>
          <p className="text-xs text-gray-600">({riskPercent}% of account)</p>
        </div>
        <div className="bg-gray-800/50 p-3 rounded-lg">
          <p className="text-xs text-gray-500 mb-1">Risk/Share</p>
          <p className="text-sm font-medium text-white">${riskPerShare.toFixed(2)}</p>
          <p className="text-xs text-gray-600">(${entryPrice} → ${stopLoss})</p>
        </div>
      </div>

      <div className={`
        p-4 rounded-lg border
        ${portfolioPercent > 20 ? "bg-amber-500/10 border-amber-500/30" : "bg-emerald-500/10 border-emerald-500/30"}
      `}>
        <div className="flex items-center justify-between mb-2">
          <span className="text-sm text-gray-400">Recommended Size</span>
          <span className="text-xl font-bold text-white">{positionSize} shares</span>
        </div>
        <div className="flex items-center justify-between text-xs">
          <span className="text-gray-500">Total Value</span>
          <span className="text-gray-400">${totalValue.toLocaleString()}</span>
        </div>
        <div className="flex items-center justify-between text-xs mt-1">
          <span className="text-gray-500">Portfolio %</span>
          <span className={portfolioPercent > 20 ? "text-amber-400" : "text-emerald-400"}>
            {portfolioPercent.toFixed(1)}%
          </span>
        </div>
        
        {portfolioPercent > 20 && (
          <p className="text-xs text-amber-400 mt-2 flex items-center gap-1">
            <AlertTriangle className="w-3 h-3" />
            High concentration risk
          </p>
        )}
      </div>
    </div>
  );
}

// ============================================
// PERFORMANCE METRICS
// ============================================

interface PerformanceMetrics {
  sharpeRatio: number;
  sortinoRatio: number;
  maxDrawdown: number;
  winRate: number;
  profitFactor: number;
  avgTradeReturn: number;
  totalTrades: number;
}

interface PerformanceMetricsPanelProps {
  metrics: PerformanceMetrics;
  benchmarkReturn?: number;
  period?: string;
}

export function PerformanceMetricsPanel({ 
  metrics, 
  benchmarkReturn = 0,
  period = "30D"
}: PerformanceMetricsPanelProps) {
  const MetricCard = ({ 
    label, 
    value, 
    suffix = "", 
    goodThreshold,
    badThreshold,
  }: { 
    label: string; 
    value: number; 
    suffix?: string;
    goodThreshold?: number;
    badThreshold?: number;
  }) => {
    let color = "text-gray-300";
    if (goodThreshold !== undefined && value >= goodThreshold) color = "text-emerald-400";
    if (badThreshold !== undefined && value <= badThreshold) color = "text-red-400";
    if (label.includes("Drawdown") && value < -10) color = "text-red-400";
    if (label.includes("Drawdown") && value > -5) color = "text-emerald-400";

    return (
      <div className="bg-gray-800/50 p-3 rounded-lg">
        <p className="text-xs text-gray-500 mb-1">{label}</p>
        <p className={`text-lg font-bold ${color}`}>
          {value > 0 && (label.includes("Return") || label.includes("Rate")) ? "+" : ""}
          {value.toFixed(2)}{suffix}
        </p>
      </div>
    );
  };

  return (
    <div className="bg-gray-900/50 border border-gray-800 rounded-xl p-4">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <div className="w-8 h-8 rounded-lg bg-purple-500/10 flex items-center justify-center">
            <BarChart3 className="w-4 h-4 text-purple-400" />
          </div>
          <div>
            <h3 className="text-sm font-semibold text-white">Performance Metrics</h3>
            <p className="text-xs text-gray-500">Last {period}</p>
          </div>
        </div>
        {benchmarkReturn !== undefined && (
          <div className="text-right">
            <p className="text-xs text-gray-500">vs Benchmark</p>
            <p className={`text-sm font-medium ${metrics.avgTradeReturn > benchmarkReturn ? "text-emerald-400" : "text-red-400"}`}>
              {metrics.avgTradeReturn > benchmarkReturn ? "+" : ""}
              {(metrics.avgTradeReturn - benchmarkReturn).toFixed(2)}%
            </p>
          </div>
        )}
      </div>

      <div className="grid grid-cols-3 gap-3">
        <MetricCard 
          label="Sharpe Ratio" 
          value={metrics.sharpeRatio} 
          goodThreshold={1.5}
          badThreshold={0.5}
        />
        <MetricCard 
          label="Sortino Ratio" 
          value={metrics.sortinoRatio}
          goodThreshold={2}
          badThreshold={1}
        />
        <MetricCard 
          label="Max Drawdown" 
          value={metrics.maxDrawdown} 
          suffix="%"
        />
        <MetricCard 
          label="Win Rate" 
          value={metrics.winRate} 
          suffix="%"
          goodThreshold={55}
          badThreshold={40}
        />
        <MetricCard 
          label="Profit Factor" 
          value={metrics.profitFactor}
          goodThreshold={1.5}
          badThreshold={1}
        />
        <MetricCard 
          label="Avg Return" 
          value={metrics.avgTradeReturn} 
          suffix="%"
        />
      </div>

      <div className="mt-3 pt-3 border-t border-gray-800">
        <div className="flex items-center justify-between text-xs">
          <span className="text-gray-500">Total Trades</span>
          <span className="text-white font-medium">{metrics.totalTrades}</span>
        </div>
      </div>
    </div>
  );
}

// ============================================
// AUDIT LOG
// ============================================

export type AuditAction = 
  | "mode_change" 
  | "trade_executed" 
  | "trade_rejected" 
  | "kill_switch"
  | "circuit_breaker"
  | "settings_change"
  | "login"
  | "logout";

interface AuditLogEntry {
  id: string;
  timestamp: Date;
  action: AuditAction;
  description: string;
  user: string;
  severity: "info" | "warning" | "critical";
  metadata?: Record<string, any>;
}

interface AuditLogPanelProps {
  entries: AuditLogEntry[];
  maxEntries?: number;
}

const ACTION_ICONS: Record<AuditAction, React.ElementType> = {
  mode_change: Settings,
  trade_executed: CheckCircle2,
  trade_rejected: X,
  kill_switch: AlertTriangle,
  circuit_breaker: Shield,
  settings_change: Settings,
  login: User,
  logout: User,
};

const SEVERITY_COLORS = {
  info: "text-blue-400 bg-blue-500/10 border-blue-500/30",
  warning: "text-amber-400 bg-amber-500/10 border-amber-500/30",
  critical: "text-red-400 bg-red-500/10 border-red-500/30",
};

export function AuditLogPanel({ entries, maxEntries = 50 }: AuditLogPanelProps) {
  const [filter, setFilter] = useState<AuditAction | "all">("all");
  
  const filteredEntries = entries
    .filter(e => filter === "all" || e.action === filter)
    .slice(0, maxEntries);

  return (
    <div className="bg-gray-900/50 border border-gray-800 rounded-xl p-4">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <div className="w-8 h-8 rounded-lg bg-gray-700/50 flex items-center justify-center">
            <Clock className="w-4 h-4 text-gray-400" />
          </div>
          <div>
            <h3 className="text-sm font-semibold text-white">Audit Log</h3>
            <p className="text-xs text-gray-500">System activity history</p>
          </div>
        </div>
        
        <select
          value={filter}
          onChange={(e) => setFilter(e.target.value as AuditAction | "all")}
          className="bg-gray-800 text-gray-300 text-xs rounded-lg px-3 py-1.5 border border-gray-700"
        >
          <option value="all">All Events</option>
          <option value="mode_change">Mode Changes</option>
          <option value="trade_executed">Trades</option>
          <option value="kill_switch">Kill Switch</option>
          <option value="circuit_breaker">Circuit Breakers</option>
        </select>
      </div>

      <div className="space-y-2 max-h-80 overflow-y-auto">
        {filteredEntries.length === 0 ? (
          <p className="text-center text-gray-500 py-8">No events found</p>
        ) : (
          filteredEntries.map((entry) => {
            const Icon = ACTION_ICONS[entry.action];
            return (
              <motion.div
                key={entry.id}
                initial={{ opacity: 0, x: -10 }}
                animate={{ opacity: 1, x: 0 }}
                className={`
                  flex items-start gap-3 p-3 rounded-lg border text-sm
                  ${SEVERITY_COLORS[entry.severity]}
                `}
              >
                <div className="w-8 h-8 rounded-lg bg-white/5 flex items-center justify-center flex-shrink-0">
                  <Icon className="w-4 h-4" />
                </div>
                <div className="flex-1 min-w-0">
                  <p className="font-medium">{entry.description}</p>
                  <div className="flex items-center gap-3 mt-1 text-xs opacity-70">
                    <span>{entry.user}</span>
                    <span>•</span>
                    <span>{entry.timestamp.toLocaleString()}</span>
                  </div>
                  {entry.metadata && (
                    <div className="mt-2 p-2 bg-black/20 rounded text-xs font-mospace">
                      {JSON.stringify(entry.metadata, null, 2)}
                    </div>
                  )}
                </div>
              </motion.div>
            );
          })
        )}
      </div>
    </div>
  );
}

// ============================================
// CORRELATION MATRIX
// ============================================

interface CorrelationData {
  symbols: string[];
  matrix: number[][];
}

interface CorrelationMatrixProps {
  data: CorrelationData;
}

export function CorrelationMatrix({ data }: CorrelationMatrixProps) {
  const getColor = (value: number) => {
    if (value > 0.8) return "bg-red-500";
    if (value > 0.6) return "bg-orange-500";
    if (value > 0.4) return "bg-yellow-500";
    if (value > 0.2) return "bg-lime-500";
    if (value > -0.2) return "bg-gray-600";
    if (value > -0.4) return "bg-cyan-500";
    if (value > -0.6) return "bg-blue-500";
    if (value > -0.8) return "bg-indigo-500";
    return "bg-purple-500";
  };

  return (
    <div className="bg-gray-900/50 border border-gray-800 rounded-xl p-4">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <div className="w-8 h-8 rounded-lg bg-indigo-500/10 flex items-center justify-center">
            <span className="text-indigo-400 font-bold text-xs">Mx</span>
          </div>
          <div>
            <h3 className="text-sm font-semibold text-white">Correlation Matrix</h3>
            <p className="text-xs text-gray-500">Diversification check</p>
          </div>
        </div>
      </div>

      <div className="overflow-x-auto">
        <div className="inline-block">
          {/* Header */}
          <div className="flex">
            <div className="w-14" /> {/* Corner */}
            {data.symbols.map((symbol) => (
              <div key={symbol} className="w-14 text-center text-xs text-gray-400 py-2">
                {symbol}
              </div>
            ))}
          </div>
          
          {/* Matrix */}
          {data.symbols.map((rowSymbol, i) => (
            <div key={rowSymbol} className="flex items-center">
              <div className="w-14 text-xs text-gray-400 text-right pr-2 py-2">
                {rowSymbol}
              </div>
              {data.matrix[i].map((value, j) => (
                <div
                  key={`${i}-${j}`}
                  className={`
                    w-14 h-10 flex items-center justify-center text-xs font-medium
                    ${i === j ? "bg-gray-700 text-gray-500" : "text-white"}
                    ${i !== j ? getColor(value) : ""}
                    ${i !== j ? "bg-opacity-30" : ""}
                  `}
                  title={`${rowSymbol} vs ${data.symbols[j]}: ${(value * 100).toFixed(0)}%`}
                >
                  {i === j ? "—" : (value * 100).toFixed(0)}
                </div>
              ))}
            </div>
          ))}
        </div>
      </div>

      {/* Legend */}
      <div className="flex items-center gap-4 mt-4 pt-4 border-t border-gray-800 text-xs">
        <span className="text-gray-500">Correlation:</span>
        <div className="flex items-center gap-1">
          <div className="w-3 h-3 bg-purple-500 rounded" />
          <span className="text-gray-400">-1.0</span>
        </div>
        <div className="flex items-center gap-1">
          <div className="w-3 h-3 bg-gray-600 rounded" />
          <span className="text-gray-400">0</span>
        </div>
        <div className="flex items-center gap-1">
          <div className="w-3 h-3 bg-red-500 rounded" />
          <span className="text-gray-400">+1.0</span>
        </div>
      </div>
    </div>
  );
}

// ============================================
// MARKET REGIME DETECTOR
// ============================================

export type MarketRegime = 
  | "bull_trend" 
  | "bear_trend" 
  | "range_bound" 
  | "high_volatility" 
  | "low_volatility";

interface MarketRegimeData {
  regime: MarketRegime;
  confidence: number;
  metrics: {
    adx: number;
    volatility: number;
    rsi: number;
    trendStrength: number;
  };
  recommendation: string;
}

interface MarketRegimeDetectorProps {
  data: MarketRegimeData;
}

const REGIME_CONFIG: Record<MarketRegime, {
  label: string;
  emoji: string;
  color: string;
  bgColor: string;
  description: string;
}> = {
  bull_trend: {
    label: "Bull Trend",
    emoji: "📈",
    color: "text-emerald-400",
    bgColor: "bg-emerald-500/10",
    description: "Strong upward trend detected",
  },
  bear_trend: {
    label: "Bear Trend",
    emoji: "📉",
    color: "text-red-400",
    bgColor: "bg-red-500/10",
    description: "Strong downward trend detected",
  },
  range_bound: {
    label: "Range Bound",
    emoji: "↔️",
    color: "text-blue-400",
    bgColor: "bg-blue-500/10",
    description: "Sideways movement, no clear trend",
  },
  high_volatility: {
    label: "High Volatility",
    emoji: "⚡",
    color: "text-amber-400",
    bgColor: "bg-amber-500/10",
    description: "Large price swings, increased risk",
  },
  low_volatility: {
    label: "Low Volatility",
    emoji: "😴",
    color: "text-gray-400",
    bgColor: "bg-gray-500/10",
    description: "Compressed range, breakout likely",
  },
};

export function MarketRegimeDetector({ data }: MarketRegimeDetectorProps) {
  const config = REGIME_CONFIG[data.regime];

  return (
    <div className="bg-gray-900/50 border border-gray-800 rounded-xl p-4">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <div className={`w-8 h-8 rounded-lg ${config.bgColor} flex items-center justify-center`}>
            <span className="text-lg">{config.emoji}</span>
          </div>
          <div>
            <h3 className="text-sm font-semibold text-white">Market Regime</h3>
            <p className="text-xs text-gray-500">Current market condition</p>
          </div>
        </div>
        <div className="text-right">
          <span className={`text-lg font-bold ${config.color}`}>{config.label}</span>
          <p className="text-xs text-gray-500">{(data.confidence * 100).toFixed(0)}% confidence</p>
        </div>
      </div>

      <p className="text-sm text-gray-400 mb-4">{config.description}</p>

      {/* Metrics */}
      <div className="grid grid-cols-2 gap-3 mb-4">
        <div className="bg-gray-800/50 p-2 rounded-lg">
          <p className="text-xs text-gray-500">ADX (Trend)</p>
          <p className={`text-sm font-medium ${data.metrics.adx > 25 ? "text-emerald-400" : "text-gray-400"}`}>
            {data.metrics.adx.toFixed(1)}
          </p>
        </div>
        <div className="bg-gray-800/50 p-2 rounded-lg">
          <p className="text-xs text-gray-500">Volatility</p>
          <p className={`text-sm font-medium ${data.metrics.volatility > 30 ? "text-amber-400" : "text-gray-400"}`}>
            {data.metrics.volatility.toFixed(1)}%
          </p>
        </div>
        <div className="bg-gray-800/50 p-2 rounded-lg">
          <p className="text-xs text-gray-500">RSI</p>
          <p className={`text-sm font-medium ${
            data.metrics.rsi > 70 ? "text-red-400" : data.metrics.rsi < 30 ? "text-emerald-400" : "text-gray-400"
          }`}>
            {data.metrics.rsi.toFixed(1)}
          </p>
        </div>
        <div className="bg-gray-800/50 p-2 rounded-lg">
          <p className="text-xs text-gray-500">Trend Strength</p>
          <p className="text-sm font-medium text-gray-400">
            {data.metrics.trendStrength.toFixed(2)}
          </p>
        </div>
      </div>

      {/* Recommendation */}
      <div className={`p-3 rounded-lg border ${config.bgColor} ${config.color.replace("text-", "border-")}/30`}>
        <p className="text-xs text-gray-500 mb-1">AI Recommendation</p>
        <p className={`text-sm font-medium ${config.color}`}>{data.recommendation}</p>
      </div>
    </div>
  );
}

// ============================================
// DAILY LIMITS PANEL
// ============================================

interface DailyLimits {
  maxTrades: number;
  currentTrades: number;
  maxLoss: number;
  currentLoss: number;
  maxExposure: number;
  currentExposure: number;
}

interface DailyLimitsPanelProps {
  limits: DailyLimits;
  onUpdateLimits: (limits: Partial<DailyLimits>) => void;
}

export function DailyLimitsPanel({ limits, onUpdateLimits }: DailyLimitsPanelProps) {
  const getProgressColor = (current: number, max: number) => {
    const pct = current / max;
    if (pct > 0.9) return "bg-red-500";
    if (pct > 0.7) return "bg-amber-500";
    return "bg-emerald-500";
  };

  const LimitBar = ({ 
    label, 
    current, 
    max, 
    prefix = "" 
  }: { 
    label: string; 
    current: number; 
    max: number; 
    prefix?: string;
  }) => {
    const pct = Math.min((current / max) * 100, 100);
    return (
      <div className="space-y-2">
        <div className="flex justify-between text-sm">
          <span className="text-gray-400">{label}</span>
          <span className={current > max * 0.9 ? "text-red-400" : "text-gray-300"}>
            {prefix}{current.toFixed(0)} / {prefix}{max}
          </span>
        </div>
        <div className="h-2 bg-gray-700 rounded-full overflow-hidden">
          <motion.div
            initial={{ width: 0 }}
            animate={{ width: `${pct}%` }}
            className={`h-full ${getProgressColor(current, max)}`}
          />
        </div>
      </div>
    );
  };

  return (
    <div className="bg-gray-900/50 border border-gray-800 rounded-xl p-4">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <div className="w-8 h-8 rounded-lg bg-teal-500/10 flex items-center justify-center">
            <span className="text-teal-400 font-bold text-xs">24H</span>
          </div>
          <div>
            <h3 className="text-sm font-semibold text-white">Daily Limits</h3>
            <p className="text-xs text-gray-500">Trading restrictions</p>
          </div>
        </div>
      </div>

      <div className="space-y-4">
        <LimitBar label="Max Trades" current={limits.currentTrades} max={limits.maxTrades} />
        <LimitBar label="Max Loss ($)" current={limits.currentLoss} max={limits.maxLoss} prefix="$" />
        <LimitBar label="Max Exposure ($)" current={limits.currentExposure} max={limits.maxExposure} prefix="$" />
      </div>

      {(limits.currentTrades >= limits.maxTrades || 
        limits.currentLoss >= limits.maxLoss || 
        limits.currentExposure >= limits.maxExposure) && (
        <div className="mt-4 p-3 bg-red-500/10 border border-red-500/30 rounded-lg">
          <p className="text-sm text-red-400 flex items-center gap-2">
            <AlertTriangle className="w-4 h-4" />
            Daily limit reached - Trading restricted
          </p>
        </div>
      )}
    </div>
  );
}

// ============================================
// TRADING HOURS
// ============================================

interface TradingHoursConfig {
  enabled: boolean;
  timezone: string;
  sessions: {
    name: string;
    start: string;
    end: string;
    enabled: boolean;
  }[];
}

interface TradingHoursPanelProps {
  config: TradingHoursConfig;
  onUpdate: (config: TradingHoursConfig) => void;
}

export function TradingHoursPanel({ config, onUpdate }: TradingHoursPanelProps) {
  const [currentTime, setCurrentTime] = useState(new Date());

  useEffect(() => {
    const timer = setInterval(() => setCurrentTime(new Date()), 1000);
    return () => clearInterval(timer);
  }, []);

  const isInTradingHours = () => {
    if (!config.enabled) return true;
    const hour = currentTime.getHours();
    const minute = currentTime.getMinutes();
    const timeStr = `${hour.toString().padStart(2, "0")}:${minute.toString().padStart(2, "0")}`;
    
    return config.sessions.some(
      s => s.enabled && timeStr >= s.start && timeStr <= s.end
    );
  };

  const inHours = isInTradingHours();

  return (
    <div className="bg-gray-900/50 border border-gray-800 rounded-xl p-4">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <div className={`w-8 h-8 rounded-lg flex items-center justify-center
            ${inHours ? "bg-emerald-500/10" : "bg-red-500/10"}`}>
            <Clock className={`w-4 h-4 ${inHours ? "text-emerald-400" : "text-red-400"}`} />
          </div>
          <div>
            <h3 className="text-sm font-semibold text-white">Trading Hours</h3>
            <p className="text-xs text-gray-500">{config.timezone}</p>
          </div>
        </div>
        <div className="text-right">
          <span className={`text-sm font-medium ${inHours ? "text-emerald-400" : "text-red-400"}`}>
            {inHours ? "🟢 MARKET OPEN" : "🔴 MARKET CLOSED"}
          </span>
          <p className="text-xs text-gray-500">{currentTime.toLocaleTimeString()}</p>
        </div>
      </div>

      <div className="space-y-2">
        {config.sessions.map((session, i) => (
          <div 
            key={i}
            className={`
              flex items-center justify-between p-2 rounded-lg text-sm
              ${session.enabled ? "bg-gray-800/50" : "bg-gray-800/20 opacity-50"}
            `}
          >
            <span className="text-gray-300">{session.name}</span>
            <span className="text-gray-500 font-mono">{session.start} - {session.end}</span>
          </div>
        ))}
      </div>

      {!inHours && config.enabled && (
        <p className="mt-3 text-xs text-amber-400 text-center">
          Trading is paused outside market hours
        </p>
      )}
    </div>
  );
}
