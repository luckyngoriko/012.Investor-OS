"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import { motion, AnimatePresence } from "framer-motion";
import {
  Settings,
  Shield,
  TrendingUp,
  Users,
  Database,
  Server,
  Bell,
  Key,
  Activity,
  AlertTriangle,
  CheckCircle2,
  XCircle,
  RefreshCw,
  Save,
  ChevronRight,
  Lock,
  Unlock,
  Cpu,
  HardDrive,
  Globe,
  Clock,
  Zap,
  BarChart3,
  Terminal,
  FileText,
  Trash2,
  Plus,
  Search,
  Filter,
  Download,
  Upload,
  Moon,
  Sun,
  Laptop,
  Smartphone,
  Tablet,
  Wallet,
  PieChart,
  Target,
  Layers,
  CpuIcon,
} from "lucide-react";
import { SecuritySettingsPanel } from "@/components/security-settings";
import { useAuth } from "@/lib/auth-context";
import { BackButton } from "@/components/back-button";
import { 
  TradingModeCard,
  ModeSettingsPanel,
  TRADING_MODES,
  type TradingMode,
  DEFAULT_MODE_CONFIG,
  type TradingModeConfig,
} from "@/components/trading-mode";

// ============================================
// TYPES
// ============================================

interface SystemSettings {
  tradingMode: TradingModeConfig;
  riskLimits: {
    maxPositionPct: number;
    maxDailyLoss: number;
    maxDrawdown: number;
    maxVar95: number;
    killSwitchEnabled: boolean;
  };
  apiConfig: {
    rateLimitPerMinute: number;
    requestTimeoutSecs: number;
  };
  notifications: {
    emailEnabled: boolean;
    pushEnabled: boolean;
    smsEnabled: boolean;
    webhookUrl: string;
  };
  appearance: {
    theme: "dark" | "light" | "auto";
    density: "compact" | "normal" | "comfortable";
    chartType: "candlestick" | "line" | "area";
  };
}

interface SystemStatus {
  cpu: number;
  memory: number;
  disk: number;
  uptime: string;
  activeConnections: number;
  lastBackup: Date;
}

// ============================================
// MOCK DATA
// ============================================

const DEFAULT_SETTINGS: SystemSettings = {
  tradingMode: DEFAULT_MODE_CONFIG,
  riskLimits: {
    maxPositionPct: 10,
    maxDailyLoss: 5,
    maxDrawdown: 20,
    maxVar95: 2,
    killSwitchEnabled: true,
  },
  apiConfig: {
    rateLimitPerMinute: 100,
    requestTimeoutSecs: 30,
  },
  notifications: {
    emailEnabled: true,
    pushEnabled: true,
    smsEnabled: false,
    webhookUrl: "",
  },
  appearance: {
    theme: "dark",
    density: "normal",
    chartType: "candlestick",
  },
};

// ============================================
// NAVIGATION TABS
// ============================================

const ADMIN_TABS = [
  { id: "trading", name: "Trading", icon: TrendingUp, color: "blue" },
  { id: "risk", name: "Risk Management", icon: Shield, color: "rose" },
  { id: "security", name: "Security", icon: Lock, color: "amber" },
  { id: "notifications", name: "Notifications", icon: Bell, color: "purple" },
  { id: "system", name: "System", icon: Server, color: "emerald" },
  { id: "api", name: "API & Keys", icon: Key, color: "cyan" },
  { id: "logs", name: "Logs & Audit", icon: FileText, color: "gray" },
] as const;

// ============================================
// TRADING SETTINGS PANEL
// ============================================

function TradingSettingsPanel({ 
  settings, 
  onChange 
}: { 
  settings: SystemSettings["tradingMode"];
  onChange: (settings: SystemSettings["tradingMode"]) => void;
}) {
  return (
    <div className="space-y-6">
      {/* Mode Selection */}
      <div>
        <h3 className="text-lg font-semibold text-white mb-4">Trading Mode</h3>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {(Object.keys(TRADING_MODES) as TradingMode[]).map((mode) => (
            <TradingModeCard
              key={mode}
              mode={mode}
              isActive={settings.mode === mode}
              onSelect={(selectedMode) => onChange({ ...settings, mode: selectedMode })}
            />
          ))}
        </div>
      </div>

      {/* Mode Configuration */}
      <div className="p-6 rounded-2xl glass-card">
        <ModeSettingsPanel
          config={settings}
          onConfigChange={onChange}
        />
      </div>

      {/* CQ Threshold Explanation */}
      <div className="p-4 rounded-xl bg-blue-500/10 border border-blue-500/20">
        <h4 className="font-medium text-blue-400 mb-2 flex items-center gap-2">
          <Target className="w-5 h-5" />
          About CQ (Conviction Quotient) Score
        </h4>
        <p className="text-sm text-blue-200/80">
          CQ Score ranges from 0-100% and represents AI&apos;s confidence in a trade proposal. 
          Higher scores indicate stronger signals across PEGY, Insider, Sentiment, and Technical factors.
        </p>
        <div className="mt-3 grid grid-cols-4 gap-2">
          {[
            { range: "0-50%", label: "Weak", color: "text-rose-400" },
            { range: "50-70%", label: "Moderate", color: "text-amber-400" },
            { range: "70-85%", label: "Strong", color: "text-blue-400" },
            { range: "85-100%", label: "Very Strong", color: "text-emerald-400" },
          ].map((item) => (
            <div key={item.range} className="text-center p-2 bg-gray-800/50 rounded-lg">
              <p className="text-xs text-gray-500">{item.range}</p>
              <p className={`text-sm font-medium ${item.color}`}>{item.label}</p>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

// ============================================
// RISK MANAGEMENT PANEL
// ============================================

function RiskManagementPanel({ 
  settings, 
  onChange 
}: { 
  settings: SystemSettings["riskLimits"];
  onChange: (settings: SystemSettings["riskLimits"]) => void;
}) {
  const [showKillSwitchConfirm, setShowKillSwitchConfirm] = useState(false);

  const handleKillSwitchToggle = (enabled: boolean) => {
    if (!enabled) {
      setShowKillSwitchConfirm(true);
    } else {
      onChange({ ...settings, killSwitchEnabled: true });
    }
  };

  return (
    <div className="space-y-6">
      {/* Kill Switch Status */}
      <div className={`p-6 rounded-2xl border ${settings.killSwitchEnabled 
        ? "bg-emerald-500/10 border-emerald-500/20" 
        : "bg-rose-500/10 border-rose-500/20"}`}>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <div className={`w-14 h-14 rounded-xl flex items-center justify-center
              ${settings.killSwitchEnabled ? "bg-emerald-500/20" : "bg-rose-500/20"}`}>
              {settings.killSwitchEnabled ? (
                <Shield className="w-7 h-7 text-emerald-400" />
              ) : (
                <AlertTriangle className="w-7 h-7 text-rose-400" />
              )}
            </div>
            <div>
              <h3 className="text-lg font-bold text-white">
                Kill Switch {settings.killSwitchEnabled ? "Armed" : "Disarmed"}
              </h3>
              <p className={`text-sm ${settings.killSwitchEnabled ? "text-emerald-400" : "text-rose-400"}`}>
                {settings.killSwitchEnabled 
                  ? "Emergency stop will trigger if risk limits are breached"
                  : "WARNING: System will continue trading through risk breaches"}
              </p>
            </div>
          </div>
          <label className="relative inline-flex items-center cursor-pointer">
            <input
              type="checkbox"
              checked={settings.killSwitchEnabled}
              onChange={(e) => handleKillSwitchToggle(e.target.checked)}
              className="sr-only peer"
            />
            <div className="w-14 h-7 bg-gray-700 peer-focus:outline-none rounded-full peer 
              peer-checked:after:translate-x-full peer-checked:after:border-white 
              after:content-[''] after:absolute after:top-[2px] after:left-[2px] 
              after:bg-white after:border-gray-300 after:border after:rounded-full 
              after:h-6 after:w-6 after:transition-all peer-checked:bg-emerald-600"></div>
          </label>
        </div>
      </div>

      {/* Risk Limits */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {[
          {
            key: "maxPositionPct",
            name: "Max Position Size",
            description: "Maximum % of portfolio in single position",
            min: 1,
            max: 50,
            step: 1,
            unit: "%",
            icon: PieChart,
          },
          {
            key: "maxDailyLoss",
            name: "Max Daily Loss",
            description: "Stop trading after losing this % in a day",
            min: 1,
            max: 20,
            step: 0.5,
            unit: "%",
            icon: TrendingUp,
          },
          {
            key: "maxDrawdown",
            name: "Max Drawdown",
            description: "Maximum peak-to-trough decline allowed",
            min: 5,
            max: 50,
            step: 5,
            unit: "%",
            icon: Activity,
          },
          {
            key: "maxVar95",
            name: "VaR Limit (95%)",
            description: "Maximum Value at Risk at 95% confidence",
            min: 0.5,
            max: 10,
            step: 0.5,
            unit: "%",
            icon: BarChart3,
          },
        ].map((limit) => (
          <div key={limit.key} className="p-4 rounded-xl glass-card">
            <div className="flex items-start gap-3 mb-4">
              <div className="w-10 h-10 rounded-lg bg-gray-700 flex items-center justify-center">
                <limit.icon className="w-5 h-5 text-gray-400" />
              </div>
              <div>
                <p className="font-medium text-white">{limit.name}</p>
                <p className="text-xs text-gray-500">{limit.description}</p>
              </div>
            </div>
            <div className="flex items-center justify-between mb-2">
              <span className="text-2xl font-bold text-white">
                {settings[limit.key as keyof typeof settings]}{limit.unit}
              </span>
              <span className="text-xs text-gray-500">
                {limit.min}{limit.unit} - {limit.max}{limit.unit}
              </span>
            </div>
            <input
              type="range"
              min={limit.min}
              max={limit.max}
              step={limit.step}
              value={settings[limit.key as keyof typeof settings] as number}
              onChange={(e) => onChange({
                ...settings,
                [limit.key]: parseFloat(e.target.value)
              })}
              className="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-rose-500"
            />
          </div>
        ))}
      </div>

      {/* Circuit Breaker Settings */}
      <div className="p-6 rounded-2xl glass-card">
        <h3 className="text-lg font-semibold text-white mb-4 flex items-center gap-2">
          <Zap className="w-5 h-5 text-amber-400" />
          Circuit Breaker Settings
        </h3>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {[
            { name: "Failure Threshold", value: 5, desc: "Failures before opening" },
            { name: "Success Threshold", value: 3, desc: "Successes to close" },
            { name: "Timeout", value: 30, unit: "s", desc: "Reset timeout" },
          ].map((item) => (
            <div key={item.name} className="p-4 bg-gray-800/30 rounded-xl">
              <p className="text-sm text-gray-500">{item.name}</p>
              <p className="text-2xl font-bold text-white">{item.value}{item.unit}</p>
              <p className="text-xs text-gray-600">{item.desc}</p>
            </div>
          ))}
        </div>
      </div>

      {/* Kill Switch Confirmation Modal */}
      <AnimatePresence>
        {showKillSwitchConfirm && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm"
            onClick={() => setShowKillSwitchConfirm(false)}
          >
            <motion.div
              initial={{ opacity: 0, scale: 0.9 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.9 }}
              onClick={(e) => e.stopPropagation()}
              className="w-full max-w-md glass-card rounded-2xl p-6"
            >
              <div className="flex items-center gap-3 mb-4">
                <div className="w-12 h-12 rounded-xl bg-rose-500/20 flex items-center justify-center">
                  <AlertTriangle className="w-6 h-6 text-rose-400" />
                </div>
                <div>
                  <h3 className="text-lg font-bold text-white">Disable Kill Switch?</h3>
                  <p className="text-sm text-gray-400">This is not recommended</p>
                </div>
              </div>
              <p className="text-sm text-gray-300 mb-6">
                Disabling the kill switch means the system will continue trading even if risk limits are breached. 
                This could result in significant losses.
              </p>
              <div className="flex gap-3">
                <button
                  onClick={() => setShowKillSwitchConfirm(false)}
                  className="flex-1 py-2.5 bg-gray-700 hover:bg-gray-600 text-white rounded-lg transition-colors"
                >
                  Cancel
                </button>
                <button
                  onClick={() => {
                    onChange({ ...settings, killSwitchEnabled: false });
                    setShowKillSwitchConfirm(false);
                  }}
                  className="flex-1 py-2.5 bg-rose-600 hover:bg-rose-500 text-white rounded-lg transition-colors"
                >
                  Disable
                </button>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

// ============================================
// NOTIFICATION SETTINGS PANEL
// ============================================

function NotificationSettingsPanel({ 
  settings, 
  onChange 
}: { 
  settings: SystemSettings["notifications"];
  onChange: (settings: SystemSettings["notifications"]) => void;
}) {
  const [testNotification, setTestNotification] = useState<string | null>(null);

  const handleTest = (type: string) => {
    setTestNotification(type);
    setTimeout(() => setTestNotification(null), 2000);
  };

  return (
    <div className="space-y-6">
      {/* Channels */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {[
          { 
            key: "emailEnabled", 
            name: "Email Notifications", 
            desc: "Send alerts to your registered email",
            icon: Laptop,
            color: "blue"
          },
          { 
            key: "pushEnabled", 
            name: "Push Notifications", 
            desc: "Browser push notifications",
            icon: Smartphone,
            color: "purple"
          },
          { 
            key: "smsEnabled", 
            name: "SMS Notifications", 
            desc: "Text messages for critical alerts",
            icon: Smartphone,
            color: "emerald"
          },
        ].map((channel) => (
          <div
            key={channel.key}
            className={`p-6 rounded-2xl border transition-all
              ${settings[channel.key as keyof typeof settings] 
                ? `bg-${channel.color}-500/10 border-${channel.color}-500/30` 
                : "bg-gray-800/30 border-gray-700/50"}`}
          >
            <div className="flex items-start justify-between mb-4">
              <div className={`w-12 h-12 rounded-xl flex items-center justify-center
                ${settings[channel.key as keyof typeof settings] 
                  ? `bg-${channel.color}-500/20` 
                  : "bg-gray-700"}`}>
                <channel.icon className={`w-6 h-6 
                  ${settings[channel.key as keyof typeof settings] 
                    ? `text-${channel.color}-400` 
                    : "text-gray-400"}`} />
              </div>
              <label className="relative inline-flex items-center cursor-pointer">
                <input
                  type="checkbox"
                  checked={settings[channel.key as keyof typeof settings] as boolean}
                  onChange={(e) => onChange({
                    ...settings,
                    [channel.key]: e.target.checked
                  })}
                  className="sr-only peer"
                />
                <div className="w-11 h-6 bg-gray-700 peer-focus:outline-none rounded-full peer 
                  peer-checked:after:translate-x-full peer-checked:after:border-white 
                  after:content-[''] after:absolute after:top-[2px] after:left-[2px] 
                  after:bg-white after:border-gray-300 after:border after:rounded-full 
                  after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600"></div>
              </label>
            </div>
            <h4 className="font-semibold text-white mb-1">{channel.name}</h4>
            <p className="text-sm text-gray-500 mb-4">{channel.desc}</p>
            <button
              onClick={() => handleTest(channel.key)}
              disabled={!settings[channel.key as keyof typeof settings]}
              className="w-full py-2 bg-gray-700 hover:bg-gray-600 disabled:opacity-50 
                disabled:cursor-not-allowed text-white text-sm rounded-lg transition-colors
                flex items-center justify-center gap-2"
            >
              {testNotification === channel.key ? (
                <>
                  <CheckCircle2 className="w-4 h-4" />
                  Sent!
                </>
              ) : (
                <>
                  <Bell className="w-4 h-4" />
                  Test
                </>
              )}
            </button>
          </div>
        ))}
      </div>

      {/* Event Types */}
      <div className="p-6 rounded-2xl glass-card">
        <h3 className="text-lg font-semibold text-white mb-4">Notification Events</h3>
        <div className="space-y-3">
          {[
            { event: "New Trade Proposal", email: true, push: true, sms: false },
            { event: "Trade Executed", email: true, push: true, sms: true },
            { event: "Position Closed", email: true, push: false, sms: false },
            { event: "Risk Alert", email: true, push: true, sms: true },
            { event: "Kill Switch Triggered", email: true, push: true, sms: true },
            { event: "Daily Summary", email: true, push: false, sms: false },
          ].map((item) => (
            <div
              key={item.event}
              className="flex items-center justify-between p-3 rounded-lg bg-gray-800/30"
            >
              <span className="text-white">{item.event}</span>
              <div className="flex gap-4">
                <div className="flex items-center gap-2" title="Email">
                  <Laptop className={`w-4 h-4 ${item.email ? "text-blue-400" : "text-gray-600"}`} />
                </div>
                <div className="flex items-center gap-2" title="Push">
                  <Smartphone className={`w-4 h-4 ${item.push ? "text-purple-400" : "text-gray-600"}`} />
                </div>
                <div className="flex items-center gap-2" title="SMS">
                  <Smartphone className={`w-4 h-4 ${item.sms ? "text-emerald-400" : "text-gray-600"}`} />
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Webhook Configuration */}
      <div className="p-6 rounded-2xl glass-card">
        <h3 className="text-lg font-semibold text-white mb-4">Webhook Integration</h3>
        <div className="space-y-4">
          <div>
            <label className="block text-sm text-gray-400 mb-2">Webhook URL</label>
            <input
              type="url"
              value={settings.webhookUrl}
              onChange={(e) => onChange({ ...settings, webhookUrl: e.target.value })}
              placeholder="https://your-server.com/webhook"
              className="w-full px-4 py-2 bg-gray-800 border border-gray-700 rounded-lg 
                text-white placeholder-gray-600 focus:border-blue-500 focus:outline-none"
            />
            <p className="text-xs text-gray-500 mt-1">
              Receive real-time notifications via HTTP POST requests
            </p>
          </div>
          {settings.webhookUrl && (
            <div className="p-4 bg-gray-800/50 rounded-xl">
              <p className="text-sm text-gray-400 mb-2">Webhook Secret (for verification)</p>
              <div className="flex items-center gap-2">
                <code className="flex-1 p-2 bg-gray-900 rounded text-sm text-gray-300 font-mono">
                  whsec_********************************
                </code>
                <button className="p-2 text-gray-400 hover:text-white hover:bg-gray-700 rounded-lg">
                  <RefreshCw className="w-4 h-4" />
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

// ============================================
// SYSTEM SETTINGS PANEL
// ============================================

function SystemSettingsPanel() {
  const [status, setStatus] = useState<SystemStatus>({
    cpu: 45,
    memory: 62,
    disk: 78,
    uptime: "14d 7h 23m",
    activeConnections: 12,
    lastBackup: new Date(Date.now() - 86400000),
  });

  const [isRestarting, setIsRestarting] = useState(false);

  const handleRestart = async () => {
    setIsRestarting(true);
    await new Promise((resolve) => setTimeout(resolve, 3000));
    setIsRestarting(false);
  };

  return (
    <div className="space-y-6">
      {/* System Status */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        {[
          { name: "CPU Usage", value: status.cpu, icon: Cpu, color: "blue" },
          { name: "Memory", value: status.memory, icon: Layers, color: "purple" },
          { name: "Disk", value: status.disk, icon: HardDrive, color: "amber" },
          { name: "Uptime", value: status.uptime, icon: Clock, color: "emerald", isText: true },
        ].map((metric) => (
          <div key={metric.name} className="p-4 rounded-xl glass-card">
            <div className="flex items-center gap-2 mb-2">
              <metric.icon className={`w-4 h-4 text-${metric.color}-400`} />
              <span className="text-sm text-gray-400">{metric.name}</span>
            </div>
            {metric.isText ? (
              <p className="text-xl font-bold text-white">{metric.value}</p>
            ) : (
              <>
                <p className="text-2xl font-bold text-white">{metric.value}%</p>
                <div className="w-full h-1.5 bg-gray-700 rounded-full mt-2">
                  <div
                    className={`h-full rounded-full bg-${metric.color}-500`}
                    style={{ width: `${metric.value}%` }}
                  />
                </div>
              </>
            )}
          </div>
        ))}
      </div>

      {/* Database & Cache */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div className="p-6 rounded-2xl glass-card">
          <h3 className="text-lg font-semibold text-white mb-4 flex items-center gap-2">
            <Database className="w-5 h-5 text-blue-400" />
            Database
          </h3>
          <div className="space-y-3">
            <div className="flex justify-between text-sm">
              <span className="text-gray-400">Connection Status</span>
              <span className="text-emerald-400 flex items-center gap-1">
                <CheckCircle2 className="w-4 h-4" />
                Connected
              </span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-gray-400">Active Connections</span>
              <span className="text-white">{status.activeConnections}</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-gray-400">Last Backup</span>
              <span className="text-white">{status.lastBackup.toLocaleString()}</span>
            </div>
            <button className="w-full mt-4 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded-lg transition-colors
              flex items-center justify-center gap-2">
              <Download className="w-4 h-4" />
              Backup Now
            </button>
          </div>
        </div>

        <div className="p-6 rounded-2xl glass-card">
          <h3 className="text-lg font-semibold text-white mb-4 flex items-center gap-2">
            <Zap className="w-5 h-5 text-amber-400" />
            Cache & Performance
          </h3>
          <div className="space-y-3">
            <div className="flex justify-between text-sm">
              <span className="text-gray-400">Redis Status</span>
              <span className="text-emerald-400 flex items-center gap-1">
                <CheckCircle2 className="w-4 h-4" />
                Running
              </span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-gray-400">Cache Hit Rate</span>
              <span className="text-white">94.2%</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-gray-400">Avg Response Time</span>
              <span className="text-white">23ms</span>
            </div>
            <button className="w-full mt-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded-lg transition-colors
              flex items-center justify-center gap-2">
              <RefreshCw className="w-4 h-4" />
              Clear Cache
            </button>
          </div>
        </div>
      </div>

      {/* System Actions */}
      <div className="p-6 rounded-2xl glass-card">
        <h3 className="text-lg font-semibold text-white mb-4">System Actions</h3>
        <div className="flex flex-wrap gap-3">
          <button
            onClick={handleRestart}
            disabled={isRestarting}
            className="px-4 py-2 bg-amber-600 hover:bg-amber-500 disabled:opacity-50 text-white 
              rounded-lg transition-colors flex items-center gap-2"
          >
            {isRestarting ? (
              <RefreshCw className="w-4 h-4 animate-spin" />
            ) : (
              <RefreshCw className="w-4 h-4" />
            )}
            Restart Services
          </button>
          <button className="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded-lg transition-colors
            flex items-center gap-2">
            <Download className="w-4 h-4" />
            Export Logs
          </button>
          <button className="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded-lg transition-colors
            flex items-center gap-2">
            <Terminal className="w-4 h-4" />
            System Shell
          </button>
        </div>
      </div>
    </div>
  );
}

// ============================================
// API KEYS PANEL
// ============================================

function ApiKeysPanel() {
  return (
    <div className="space-y-6">
      <div className="p-6 rounded-2xl glass-card">
        <h3 className="text-lg font-semibold text-white mb-4">API Configuration</h3>
        <div className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label className="block text-sm text-gray-400 mb-2">Rate Limit (requests/min)</label>
              <input
                type="number"
                defaultValue={100}
                className="w-full px-4 py-2 bg-gray-800 border border-gray-700 rounded-lg 
                  text-white focus:border-blue-500 focus:outline-none"
              />
            </div>
            <div>
              <label className="block text-sm text-gray-400 mb-2">Request Timeout (seconds)</label>
              <input
                type="number"
                defaultValue={30}
                className="w-full px-4 py-2 bg-gray-800 border border-gray-700 rounded-lg 
                  text-white focus:border-blue-500 focus:outline-none"
              />
            </div>
          </div>

          <div>
            <label className="block text-sm text-gray-400 mb-2">CORS Origins</label>
            <textarea
              defaultValue={"http://localhost:3000\nhttps://app.investor-os.com"}
              rows={3}
              className="w-full px-4 py-2 bg-gray-800 border border-gray-700 rounded-lg 
                text-white focus:border-blue-500 focus:outline-none font-mono text-sm"
            />
          </div>

          <div>
            <label className="block text-sm text-gray-400 mb-2">IP Whitelist</label>
            <textarea
              placeholder="Enter IP addresses, one per line..."
              rows={3}
              className="w-full px-4 py-2 bg-gray-800 border border-gray-700 rounded-lg 
                text-white placeholder-gray-600 focus:border-blue-500 focus:outline-none font-mono text-sm"
            />
          </div>
        </div>
      </div>

      <div className="p-6 rounded-2xl glass-card">
        <h3 className="text-lg font-semibold text-white mb-4">Broker API Settings</h3>
        <div className="space-y-4">
          {["Alpaca", "Interactive Brokers", "Binance"].map((broker) => (
            <div
              key={broker}
              className="flex items-center justify-between p-4 bg-gray-800/30 rounded-xl"
            >
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-lg bg-gray-700 flex items-center justify-center">
                  <Wallet className="w-5 h-5 text-gray-400" />
                </div>
                <div>
                  <p className="font-medium text-white">{broker}</p>
                  <p className="text-sm text-emerald-400 flex items-center gap-1">
                    <CheckCircle2 className="w-3 h-3" />
                    Connected
                  </p>
                </div>
              </div>
              <button className="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded-lg transition-colors">
                Configure
              </button>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

// ============================================
// LOGS & AUDIT PANEL
// ============================================

function LogsPanel() {
  const [filter, setFilter] = useState("all");

  const logs = [
    { timestamp: new Date(), level: "info", message: "Trade executed: BUY AAPL @ 185.50", source: "Trading Engine" },
    { timestamp: new Date(Date.now() - 60000), level: "warn", message: "High volatility detected in NVDA", source: "Risk Engine" },
    { timestamp: new Date(Date.now() - 120000), level: "error", message: "API rate limit approached for Alpaca", source: "Broker API" },
    { timestamp: new Date(Date.now() - 180000), level: "info", message: "Daily portfolio rebalance completed", source: "Portfolio Manager" },
    { timestamp: new Date(Date.now() - 300000), level: "info", message: "Market regime changed to Risk On", source: "AI Engine" },
  ];

  return (
    <div className="space-y-6">
      {/* Filters */}
      <div className="flex flex-wrap gap-3">
        {["all", "info", "warn", "error"].map((f) => (
          <button
            key={f}
            onClick={() => setFilter(f)}
            className={`px-4 py-2 rounded-lg font-medium transition-colors
              ${filter === f ? "bg-blue-600 text-white" : "bg-gray-800 text-gray-400 hover:text-white"}`}
          >
            {f.charAt(0).toUpperCase() + f.slice(1)}
          </button>
        ))}
        <div className="flex-1" />
        <button className="px-4 py-2 bg-gray-800 hover:bg-gray-700 text-white rounded-lg transition-colors
          flex items-center gap-2">
          <Download className="w-4 h-4" />
          Export
        </button>
      </div>

      {/* Logs Table */}
      <div className="rounded-2xl glass-card overflow-hidden">
        <table className="w-full">
          <thead>
            <tr className="border-b border-gray-800">
              <th className="text-left py-3 px-4 text-sm font-medium text-gray-400">Timestamp</th>
              <th className="text-left py-3 px-4 text-sm font-medium text-gray-400">Level</th>
              <th className="text-left py-3 px-4 text-sm font-medium text-gray-400">Message</th>
              <th className="text-left py-3 px-4 text-sm font-medium text-gray-400">Source</th>
            </tr>
          </thead>
          <tbody>
            {logs.map((log, idx) => (
              <tr key={idx} className="border-b border-gray-800/50 hover:bg-gray-800/30">
                <td className="py-3 px-4 text-sm text-gray-400">
                  {log.timestamp.toLocaleTimeString()}
                </td>
                <td className="py-3 px-4">
                  <span
                    className={`px-2 py-1 text-xs rounded-full
                      ${log.level === "info" ? "bg-blue-500/20 text-blue-400" : ""}
                      ${log.level === "warn" ? "bg-amber-500/20 text-amber-400" : ""}
                      ${log.level === "error" ? "bg-rose-500/20 text-rose-400" : ""}`}
                  >
                    {log.level}
                  </span>
                </td>
                <td className="py-3 px-4 text-sm text-white">{log.message}</td>
                <td className="py-3 px-4 text-sm text-gray-500">{log.source}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

// ============================================
// MAIN ADMIN PAGE
// ============================================

export default function AdminPage() {
  const router = useRouter();
  const { user, hasRole, isLoading } = useAuth();
  const [activeTab, setActiveTab] = useState<typeof ADMIN_TABS[number]["id"]>("trading");
  const [settings, setSettings] = useState<SystemSettings>(DEFAULT_SETTINGS);
  const [hasChanges, setHasChanges] = useState(false);
  const [isSaving, setIsSaving] = useState(false);

  // Protect admin page - only admins allowed
  useEffect(() => {
    if (!isLoading && !hasRole("admin")) {
      router.push("/");
    }
  }, [isLoading, hasRole, router]);

  // Show loading while checking auth
  if (isLoading) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-[#0a0f1c] via-[#111827] to-[#0a0f1c] flex items-center justify-center">
        <div className="w-8 h-8 border-2 border-blue-500/30 border-t-blue-500 rounded-full animate-spin" />
      </div>
    );
  }

  // Don't render if not admin (will redirect)
  if (!hasRole("admin")) {
    return null;
  }

  const handleSave = async () => {
    setIsSaving(true);
    // Simulate API call
    await new Promise((resolve) => setTimeout(resolve, 1500));
    setIsSaving(false);
    setHasChanges(false);
  };

  const updateSettings = (section: keyof SystemSettings, value: any) => {
    setSettings({ ...settings, [section]: value });
    setHasChanges(true);
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0f1c] via-[#111827] to-[#0a0f1c]">
      {/* Header */}
      <header className="border-b border-gray-800 bg-gray-900/50 backdrop-blur-lg">
        <div className="max-w-7xl mx-auto px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <BackButton className="mr-2" />
              <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-blue-600 to-cyan-500 
                flex items-center justify-center shadow-lg shadow-blue-500/20">
                <Settings className="w-6 h-6 text-white" />
              </div>
              <div>
                <h1 className="text-2xl font-bold text-white">Administration</h1>
                <p className="text-sm text-gray-400">System configuration and management</p>
              </div>
            </div>

            {hasChanges && (
              <motion.div
                initial={{ opacity: 0, y: -10 }}
                animate={{ opacity: 1, y: 0 }}
                className="flex items-center gap-3"
              >
                <span className="text-sm text-amber-400">Unsaved changes</span>
                <button
                  onClick={handleSave}
                  disabled={isSaving}
                  className="flex items-center gap-2 px-6 py-2.5 bg-emerald-600 hover:bg-emerald-500 
                    disabled:opacity-50 text-white font-medium rounded-lg transition-colors"
                >
                  {isSaving ? (
                    <RefreshCw className="w-4 h-4 animate-spin" />
                  ) : (
                    <Save className="w-4 h-4" />
                  )}
                  Save Changes
                </button>
              </motion.div>
            )}
          </div>
        </div>
      </header>

      <div className="max-w-7xl mx-auto px-6 py-8">
        <div className="grid grid-cols-1 lg:grid-cols-4 gap-8">
          {/* Sidebar Navigation */}
          <div className="lg:col-span-1">
            <nav className="space-y-1 sticky top-8">
              {ADMIN_TABS.map((tab) => (
                <button
                  key={tab.id}
                  onClick={() => setActiveTab(tab.id)}
                  className={`w-full flex items-center gap-3 px-4 py-3 rounded-xl transition-all
                    ${activeTab === tab.id 
                      ? `bg-${tab.color}-500/10 text-${tab.color}-400 border border-${tab.color}-500/30` 
                      : "text-gray-400 hover:text-white hover:bg-gray-800/50"}`}
                >
                  <tab.icon className="w-5 h-5" />
                  <span className="font-medium">{tab.name}</span>
                  {activeTab === tab.id && (
                    <ChevronRight className="w-4 h-4 ml-auto" />
                  )}
                </button>
              ))}
            </nav>

            {/* Quick Stats */}
            <div className="mt-8 p-4 rounded-xl glass-card">
              <h3 className="text-sm font-medium text-gray-400 mb-4">System Status</h3>
              <div className="space-y-3">
                <div className="flex items-center justify-between text-sm">
                  <span className="text-gray-500">API</span>
                  <span className="text-emerald-400 flex items-center gap-1">
                    <CheckCircle2 className="w-3 h-3" />
                    Online
                  </span>
                </div>
                <div className="flex items-center justify-between text-sm">
                  <span className="text-gray-500">Database</span>
                  <span className="text-emerald-400 flex items-center gap-1">
                    <CheckCircle2 className="w-3 h-3" />
                    Connected
                  </span>
                </div>
                <div className="flex items-center justify-between text-sm">
                  <span className="text-gray-500">AI Engine</span>
                  <span className="text-emerald-400 flex items-center gap-1">
                    <CheckCircle2 className="w-3 h-3" />
                    Running
                  </span>
                </div>
              </div>
            </div>
          </div>

          {/* Main Content */}
          <div className="lg:col-span-3">
            <AnimatePresence mode="wait">
              {activeTab === "trading" && (
                <motion.div
                  key="trading"
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -10 }}
                >
                  <TradingSettingsPanel
                    settings={settings.tradingMode}
                    onChange={(tradingMode) => updateSettings("tradingMode", tradingMode)}
                  />
                </motion.div>
              )}

              {activeTab === "risk" && (
                <motion.div
                  key="risk"
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -10 }}
                >
                  <RiskManagementPanel
                    settings={settings.riskLimits}
                    onChange={(riskLimits) => updateSettings("riskLimits", riskLimits)}
                  />
                </motion.div>
              )}

              {activeTab === "security" && (
                <motion.div
                  key="security"
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -10 }}
                >
                  <SecuritySettingsPanel />
                </motion.div>
              )}

              {activeTab === "notifications" && (
                <motion.div
                  key="notifications"
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -10 }}
                >
                  <NotificationSettingsPanel
                    settings={settings.notifications}
                    onChange={(notifications) => updateSettings("notifications", notifications)}
                  />
                </motion.div>
              )}

              {activeTab === "system" && (
                <motion.div
                  key="system"
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -10 }}
                >
                  <SystemSettingsPanel />
                </motion.div>
              )}

              {activeTab === "api" && (
                <motion.div
                  key="api"
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -10 }}
                >
                  <ApiKeysPanel />
                </motion.div>
              )}

              {activeTab === "logs" && (
                <motion.div
                  key="logs"
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -10 }}
                >
                  <LogsPanel />
                </motion.div>
              )}
            </AnimatePresence>
          </div>
        </div>
      </div>
    </div>
  );
}
