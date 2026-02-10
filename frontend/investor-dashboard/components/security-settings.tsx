"use client";

import { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  Shield,
  ShieldCheck,
  ShieldAlert,
  Key,
  Smartphone,
  Fingerprint,
  Lock,
  Unlock,
  Eye,
  EyeOff,
  RefreshCw,
  Trash2,
  CheckCircle2,
  XCircle,
  AlertTriangle,
  Globe,
  Monitor,
  SmartphoneIcon,
  History,
  UserX,
  Copy,
  QrCode,
  Clock,
  MapPin,
  ChevronRight,
  Loader2,
} from "lucide-react";

// ============================================
// TYPES
// ============================================

interface SecuritySettings {
  twoFactorEnabled: boolean;
  twoFactorMethod: "totp" | "sms" | "email" | null;
  biometricEnabled: boolean;
  hardwareKeyEnabled: boolean;
  sessionTimeout: number; // minutes
  requirePasswordForTrading: boolean;
  ipWhitelist: string[];
  apiKeyRotation: number; // days
}

interface Device {
  id: string;
  name: string;
  type: "desktop" | "mobile" | "tablet";
  browser: string;
  os: string;
  ip: string;
  location: string;
  lastActive: Date;
  isCurrent: boolean;
  isTrusted: boolean;
}

interface LoginAttempt {
  id: string;
  timestamp: Date;
  ip: string;
  location: string;
  device: string;
  browser: string;
  status: "success" | "failed" | "blocked";
  reason?: string;
}

interface ApiKey {
  id: string;
  name: string;
  key: string;
  createdAt: Date;
  lastUsed: Date | null;
  permissions: string[];
  ipRestrictions: string[];
}

// ============================================
// MOCK DATA
// ============================================

const MOCK_DEVICES: Device[] = [
  {
    id: "1",
    name: "MacBook Pro - Chrome",
    type: "desktop",
    browser: "Chrome 120",
    os: "macOS Sonoma",
    ip: "192.168.1.105",
    location: "Sofia, Bulgaria",
    lastActive: new Date(),
    isCurrent: true,
    isTrusted: true,
  },
  {
    id: "2",
    name: "iPhone 15 Pro",
    type: "mobile",
    browser: "Safari",
    os: "iOS 17.2",
    ip: "10.0.0.23",
    location: "Sofia, Bulgaria",
    lastActive: new Date(Date.now() - 3600000), // 1 hour ago
    isCurrent: false,
    isTrusted: true,
  },
  {
    id: "3",
    name: "Windows PC - Edge",
    type: "desktop",
    browser: "Edge 118",
    os: "Windows 11",
    ip: "203.0.113.45",
    location: "London, UK",
    lastActive: new Date(Date.now() - 86400000 * 2), // 2 days ago
    isCurrent: false,
    isTrusted: false,
  },
];

const MOCK_LOGIN_HISTORY: LoginAttempt[] = [
  {
    id: "1",
    timestamp: new Date(),
    ip: "192.168.1.105",
    location: "Sofia, Bulgaria",
    device: "MacBook Pro",
    browser: "Chrome",
    status: "success",
  },
  {
    id: "2",
    timestamp: new Date(Date.now() - 3600000),
    ip: "10.0.0.23",
    location: "Sofia, Bulgaria",
    device: "iPhone 15 Pro",
    browser: "Safari",
    status: "success",
  },
  {
    id: "3",
    timestamp: new Date(Date.now() - 7200000),
    ip: "198.51.100.22",
    location: "Unknown Location",
    device: "Unknown Device",
    browser: "Firefox",
    status: "blocked",
    reason: "IP not in whitelist",
  },
];

const MOCK_API_KEYS: ApiKey[] = [
  {
    id: "1",
    name: "Trading Bot",
    key: "ak_live_********************************",
    createdAt: new Date(Date.now() - 86400000 * 30),
    lastUsed: new Date(Date.now() - 3600000),
    permissions: ["read", "trade"],
    ipRestrictions: ["192.168.1.0/24"],
  },
  {
    id: "2",
    name: "Mobile App",
    key: "ak_live_********************************",
    createdAt: new Date(Date.now() - 86400000 * 60),
    lastUsed: new Date(Date.now() - 7200000),
    permissions: ["read"],
    ipRestrictions: [],
  },
];

// ============================================
// 2FA SETUP COMPONENT
// ============================================

function TwoFactorSetup({ 
  onComplete, 
  onCancel 
}: { 
  onComplete: () => void;
  onCancel: () => void;
}) {
  const [step, setStep] = useState<"method" | "verify" | "backup">("method");
  const [method, setMethod] = useState<"totp" | "sms" | "email">("totp");
  const [verificationCode, setVerificationCode] = useState("");
  const [isVerifying, setIsVerifying] = useState(false);

  const handleVerify = async () => {
    setIsVerifying(true);
    // Simulate API call
    await new Promise((resolve) => setTimeout(resolve, 1500));
    setIsVerifying(false);
    if (step === "verify") {
      setStep("backup");
    } else {
      onComplete();
    }
  };

  return (
    <div className="p-6">
      {/* Progress */}
      <div className="flex items-center gap-2 mb-6">
        {["method", "verify", "backup"].map((s, i) => (
          <div key={s} className="flex items-center gap-2">
            <div
              className={`w-8 h-8 rounded-full flex items-center justify-center text-sm font-medium
                ${step === s ? "bg-blue-600 text-white" : ""}
                ${["method", "verify", "backup"].indexOf(step) > i ? "bg-emerald-600 text-white" : ""}
                ${["method", "verify", "backup"].indexOf(step) < i ? "bg-gray-700 text-gray-400" : ""}`}
            >
              {["method", "verify", "backup"].indexOf(step) > i ? (
                <CheckCircle2 className="w-4 h-4" />
              ) : (
                i + 1
              )}
            </div>
            {i < 2 && <div className="w-8 h-0.5 bg-gray-700" />}
          </div>
        ))}
      </div>

      {/* Step Content */}
      <AnimatePresence mode="wait">
        {step === "method" && (
          <motion.div
            key="method"
            initial={{ opacity: 0, x: 20 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: -20 }}
            className="space-y-4"
          >
            <h3 className="text-lg font-semibold text-white">Choose 2FA Method</h3>
            
            {[
              { 
                id: "totp", 
                name: "Authenticator App", 
                desc: "Google Authenticator, Authy, 1Password",
                icon: Smartphone,
                recommended: true 
              },
              { 
                id: "sms", 
                name: "SMS", 
                desc: "Text message to your phone",
                icon: SmartphoneIcon,
                recommended: false 
              },
              { 
                id: "email", 
                name: "Email", 
                desc: "Code sent to your email",
                icon: Shield,
                recommended: false 
              },
            ].map((m) => (
              <button
                key={m.id}
                onClick={() => setMethod(m.id as any)}
                className={`w-full flex items-center gap-4 p-4 rounded-xl border transition-all
                  ${method === m.id 
                    ? "bg-blue-500/10 border-blue-500/50" 
                    : "bg-gray-800/30 border-gray-700/50 hover:border-gray-600"}`}
              >
                <div className={`w-12 h-12 rounded-lg flex items-center justify-center
                  ${method === m.id ? "bg-blue-500/20 text-blue-400" : "bg-gray-700 text-gray-400"}`}>
                  <m.icon className="w-6 h-6" />
                </div>
                <div className="flex-1 text-left">
                  <div className="flex items-center gap-2">
                    <span className="font-medium text-white">{m.name}</span>
                    {m.recommended && (
                      <span className="px-2 py-0.5 bg-emerald-500/20 text-emerald-400 text-xs rounded-full">
                        Recommended
                      </span>
                    )}
                  </div>
                  <p className="text-sm text-gray-400">{m.desc}</p>
                </div>
                {method === m.id && <CheckCircle2 className="w-5 h-5 text-blue-400" />}
              </button>
            ))}
          </motion.div>
        )}

        {step === "verify" && (
          <motion.div
            key="verify"
            initial={{ opacity: 0, x: 20 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: -20 }}
            className="space-y-6"
          >
            <h3 className="text-lg font-semibold text-white">Verify Setup</h3>
            
            {method === "totp" && (
              <div className="text-center">
                <div className="w-48 h-48 mx-auto mb-4 bg-white rounded-xl p-4">
                  {/* QR Code Placeholder */}
                  <div className="w-full h-full bg-gray-900 rounded-lg flex items-center justify-center">
                    <QrCode className="w-32 h-32 text-white" />
                  </div>
                </div>
                <p className="text-sm text-gray-400 mb-4">
                  Scan this QR code with your authenticator app
                </p>
                <div className="flex items-center justify-center gap-2 p-3 bg-gray-800 rounded-lg">
                  <code className="text-sm text-gray-300">ABCD-EFGH-IJKL-MNOP</code>
                  <button className="p-1 hover:bg-gray-700 rounded">
                    <Copy className="w-4 h-4 text-gray-400" />
                  </button>
                </div>
              </div>
            )}

            <div>
              <label className="block text-sm text-gray-400 mb-2">
                Enter verification code
              </label>
              <div className="flex gap-2">
                {Array(6).fill(0).map((_, i) => (
                  <input
                    key={i}
                    type="text"
                    maxLength={1}
                    className="w-12 h-12 text-center bg-gray-800 border border-gray-700 rounded-lg 
                      text-white font-mono text-lg focus:border-blue-500 focus:outline-none"
                  />
                ))}
              </div>
            </div>
          </motion.div>
        )}

        {step === "backup" && (
          <motion.div
            key="backup"
            initial={{ opacity: 0, x: 20 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: -20 }}
            className="space-y-4"
          >
            <h3 className="text-lg font-semibold text-white">Save Backup Codes</h3>
            <p className="text-sm text-gray-400">
              If you lose access to your 2FA device, you can use these backup codes to sign in.
              Each code can only be used once.
            </p>
            
            <div className="p-4 bg-amber-500/10 border border-amber-500/20 rounded-xl">
              <div className="flex items-center gap-2 text-amber-400 mb-3">
                <AlertTriangle className="w-5 h-5" />
                <span className="font-medium">Important</span>
              </div>
              <p className="text-sm text-amber-200/80">
                Save these codes in a secure location. They will not be shown again.
              </p>
            </div>

            <div className="grid grid-cols-2 gap-2">
              {Array(8).fill(0).map((_, i) => (
                <code
                  key={i}
                  className="p-2 bg-gray-800 rounded-lg text-center font-mono text-sm text-gray-300"
                >
                  {Math.random().toString(36).substring(2, 8).toUpperCase()}
                </code>
              ))}
            </div>

            <button className="w-full py-2 bg-gray-700 hover:bg-gray-600 text-white rounded-lg 
              transition-colors flex items-center justify-center gap-2">
              <Copy className="w-4 h-4" />
              Copy Codes
            </button>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Actions */}
      <div className="flex justify-end gap-3 mt-6 pt-4 border-t border-gray-800">
        <button
          onClick={onCancel}
          className="px-4 py-2 text-gray-400 hover:text-white transition-colors"
        >
          Cancel
        </button>
        <button
          onClick={handleVerify}
          disabled={isVerifying}
          className="px-6 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded-lg 
            transition-colors flex items-center gap-2 disabled:opacity-50"
        >
          {isVerifying && <Loader2 className="w-4 h-4 animate-spin" />}
          {step === "backup" ? "Complete Setup" : "Continue"}
        </button>
      </div>
    </div>
  );
}

// ============================================
// DEVICE MANAGEMENT
// ============================================

function DeviceManagement() {
  const [devices, setDevices] = useState<Device[]>(MOCK_DEVICES);
  const [showTerminateAll, setShowTerminateAll] = useState(false);

  const handleTerminate = (deviceId: string) => {
    setDevices(devices.filter((d) => d.id !== deviceId));
  };

  const handleTerminateAll = () => {
    setDevices(devices.filter((d) => d.isCurrent));
    setShowTerminateAll(false);
  };

  const getDeviceIcon = (type: Device["type"]) => {
    switch (type) {
      case "mobile": return SmartphoneIcon;
      case "tablet": return Smartphone;
      default: return Monitor;
    }
  };

  return (
    <div className="space-y-4">
      {/* Current Device */}
      <div className="p-4 rounded-xl bg-emerald-500/10 border border-emerald-500/20">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-lg bg-emerald-500/20 flex items-center justify-center">
              <CheckCircle2 className="w-5 h-5 text-emerald-400" />
            </div>
            <div>
              <p className="font-medium text-white">Current Session</p>
              <p className="text-sm text-gray-400">
                {devices.find((d) => d.isCurrent)?.name}
              </p>
            </div>
          </div>
          <span className="px-2 py-1 bg-emerald-500/20 text-emerald-400 text-xs rounded-full">
            Active Now
          </span>
        </div>
      </div>

      {/* Other Devices */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <h4 className="text-sm font-medium text-gray-400">Other Devices</h4>
          <button
            onClick={() => setShowTerminateAll(true)}
            className="text-sm text-rose-400 hover:text-rose-300 flex items-center gap-1"
          >
            <UserX className="w-4 h-4" />
            Terminate All
          </button>
        </div>

        {devices
          .filter((d) => !d.isCurrent)
          .map((device) => {
            const Icon = getDeviceIcon(device.type);
            return (
              <div
                key={device.id}
                className="flex items-center justify-between p-4 rounded-xl bg-gray-800/30 
                  border border-gray-700/50"
              >
                <div className="flex items-center gap-3">
                  <div className="w-10 h-10 rounded-lg bg-gray-700 flex items-center justify-center">
                    <Icon className="w-5 h-5 text-gray-400" />
                  </div>
                  <div>
                    <p className="font-medium text-white">{device.name}</p>
                    <p className="text-sm text-gray-500">
                      {device.location} • {device.ip}
                    </p>
                    <p className="text-xs text-gray-600">
                      Last active: {device.lastActive.toLocaleString()}
                    </p>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  {!device.isTrusted && (
                    <span className="px-2 py-1 bg-amber-500/10 text-amber-400 text-xs rounded-full">
                      Untrusted
                    </span>
                  )}
                  <button
                    onClick={() => handleTerminate(device.id)}
                    className="p-2 text-gray-400 hover:text-rose-400 hover:bg-rose-500/10 
                      rounded-lg transition-colors"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
              </div>
            );
          })}
      </div>

      {/* Terminate All Confirmation */}
      <AnimatePresence>
        {showTerminateAll && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm"
            onClick={() => setShowTerminateAll(false)}
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
                  <h3 className="text-lg font-bold text-white">Terminate All Sessions?</h3>
                  <p className="text-sm text-gray-400">This will sign you out everywhere</p>
                </div>
              </div>
              <p className="text-sm text-gray-300 mb-6">
                All other devices will be immediately signed out. You will need to sign in again on those devices.
              </p>
              <div className="flex gap-3">
                <button
                  onClick={() => setShowTerminateAll(false)}
                  className="flex-1 py-2.5 bg-gray-700 hover:bg-gray-600 text-white rounded-lg transition-colors"
                >
                  Cancel
                </button>
                <button
                  onClick={handleTerminateAll}
                  className="flex-1 py-2.5 bg-rose-600 hover:bg-rose-500 text-white rounded-lg transition-colors"
                >
                  Terminate All
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
// LOGIN HISTORY
// ============================================

function LoginHistory() {
  const [history] = useState<LoginAttempt[]>(MOCK_LOGIN_HISTORY);
  const [filter, setFilter] = useState<"all" | "success" | "failed">("all");

  const filtered = history.filter((h) => {
    if (filter === "success") return h.status === "success";
    if (filter === "failed") return h.status === "failed" || h.status === "blocked";
    return true;
  });

  return (
    <div className="space-y-4">
      {/* Filters */}
      <div className="flex gap-2">
        {["all", "success", "failed"].map((f) => (
          <button
            key={f}
            onClick={() => setFilter(f as any)}
            className={`px-3 py-1.5 text-sm font-medium rounded-lg transition-colors
              ${filter === f 
                ? "bg-blue-600 text-white" 
                : "bg-gray-800 text-gray-400 hover:text-white"}`}
          >
            {f.charAt(0).toUpperCase() + f.slice(1)}
          </button>
        ))}
      </div>

      {/* List */}
      <div className="space-y-2 max-h-80 overflow-y-auto">
        {filtered.map((attempt) => (
          <div
            key={attempt.id}
            className="flex items-center justify-between p-4 rounded-xl bg-gray-800/30 
              border border-gray-700/50"
          >
            <div className="flex items-center gap-3">
              <div
                className={`w-10 h-10 rounded-lg flex items-center justify-center
                  ${attempt.status === "success" ? "bg-emerald-500/10" : ""}
                  ${attempt.status === "failed" ? "bg-amber-500/10" : ""}
                  ${attempt.status === "blocked" ? "bg-rose-500/10" : ""}`}
              >
                {attempt.status === "success" && <CheckCircle2 className="w-5 h-5 text-emerald-400" />}
                {attempt.status === "failed" && <XCircle className="w-5 h-5 text-amber-400" />}
                {attempt.status === "blocked" && <ShieldAlert className="w-5 h-5 text-rose-400" />}
              </div>
              <div>
                <div className="flex items-center gap-2">
                  <p className="font-medium text-white">{attempt.device}</p>
                  <span
                    className={`px-2 py-0.5 text-xs rounded-full
                      ${attempt.status === "success" ? "bg-emerald-500/20 text-emerald-400" : ""}
                      ${attempt.status === "failed" ? "bg-amber-500/20 text-amber-400" : ""}
                      ${attempt.status === "blocked" ? "bg-rose-500/20 text-rose-400" : ""}`}
                  >
                    {attempt.status}
                  </span>
                </div>
                <p className="text-sm text-gray-500">
                  {attempt.browser} • {attempt.location}
                </p>
                {attempt.reason && (
                  <p className="text-xs text-rose-400">{attempt.reason}</p>
                )}
              </div>
            </div>
            <div className="text-right">
              <p className="text-sm text-gray-400">
                {attempt.timestamp.toLocaleTimeString()}
              </p>
              <p className="text-xs text-gray-600">{attempt.ip}</p>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

// ============================================
// API KEY MANAGEMENT
// ============================================

function ApiKeyManagement() {
  const [apiKeys, setApiKeys] = useState<ApiKey[]>(MOCK_API_KEYS);
  const [showCreate, setShowCreate] = useState(false);
  const [showKey, setShowKey] = useState<string | null>(null);
  const [newKeyName, setNewKeyName] = useState("");
  const [createdKey, setCreatedKey] = useState<string | null>(null);

  const handleCreate = () => {
    const newKey: ApiKey = {
      id: Math.random().toString(36),
      name: newKeyName,
      key: "ak_live_" + Math.random().toString(36).substring(2, 34),
      createdAt: new Date(),
      lastUsed: null,
      permissions: ["read"],
      ipRestrictions: [],
    };
    setApiKeys([...apiKeys, newKey]);
    setCreatedKey(newKey.key);
    setShowCreate(false);
    setNewKeyName("");
  };

  const handleDelete = (id: string) => {
    setApiKeys(apiKeys.filter((k) => k.id !== id));
  };

  const handleCopy = (key: string) => {
    navigator.clipboard.writeText(key);
  };

  return (
    <div className="space-y-4">
      {/* Create Button */}
      <button
        onClick={() => setShowCreate(true)}
        className="w-full py-3 border-2 border-dashed border-gray-700 rounded-xl 
          text-gray-400 hover:text-white hover:border-gray-500 transition-colors
          flex items-center justify-center gap-2"
      >
        <Key className="w-5 h-5" />
        Create New API Key
      </button>

      {/* Keys List */}
      <div className="space-y-2">
        {apiKeys.map((key) => (
          <div
            key={key.id}
            className="p-4 rounded-xl bg-gray-800/30 border border-gray-700/50"
          >
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-lg bg-gray-700 flex items-center justify-center">
                  <Key className="w-5 h-5 text-gray-400" />
                </div>
                <div>
                  <p className="font-medium text-white">{key.name}</p>
                  <p className="text-xs text-gray-500">
                    Created {key.createdAt.toLocaleDateString()}
                    {key.lastUsed && ` • Last used ${key.lastUsed.toLocaleDateString()}`}
                  </p>
                </div>
              </div>
              <button
                onClick={() => handleDelete(key.id)}
                className="p-2 text-gray-400 hover:text-rose-400 hover:bg-rose-500/10 
                  rounded-lg transition-colors"
              >
                <Trash2 className="w-4 h-4" />
              </button>
            </div>

            {/* Key Display */}
            <div className="flex items-center gap-2 p-3 bg-gray-900 rounded-lg">
              <code className="flex-1 text-sm text-gray-400 font-mono">
                {showKey === key.id ? key.key : key.key.replace(/./g, "*")}
              </code>
              <button
                onClick={() => setShowKey(showKey === key.id ? null : key.id)}
                className="p-1.5 text-gray-400 hover:text-white hover:bg-gray-700 rounded transition-colors"
              >
                {showKey === key.id ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
              </button>
              <button
                onClick={() => handleCopy(key.key)}
                className="p-1.5 text-gray-400 hover:text-white hover:bg-gray-700 rounded transition-colors"
              >
                <Copy className="w-4 h-4" />
              </button>
            </div>

            {/* Permissions */}
            <div className="flex gap-2 mt-3">
              {key.permissions.map((perm) => (
                <span
                  key={perm}
                  className="px-2 py-1 bg-blue-500/10 text-blue-400 text-xs rounded-full"
                >
                  {perm}
                </span>
              ))}
              {key.ipRestrictions.length > 0 && (
                <span className="px-2 py-1 bg-amber-500/10 text-amber-400 text-xs rounded-full">
                  IP Restricted
                </span>
              )}
            </div>
          </div>
        ))}
      </div>

      {/* Create Modal */}
      <AnimatePresence>
        {showCreate && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm"
            onClick={() => setShowCreate(false)}
          >
            <motion.div
              initial={{ opacity: 0, scale: 0.9 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.9 }}
              onClick={(e) => e.stopPropagation()}
              className="w-full max-w-md glass-card rounded-2xl p-6"
            >
              <h3 className="text-lg font-bold text-white mb-4">Create API Key</h3>
              <div className="space-y-4">
                <div>
                  <label className="block text-sm text-gray-400 mb-2">Key Name</label>
                  <input
                    type="text"
                    value={newKeyName}
                    onChange={(e) => setNewKeyName(e.target.value)}
                    placeholder="e.g., Trading Bot"
                    className="w-full px-4 py-2 bg-gray-800 border border-gray-700 rounded-lg 
                      text-white placeholder-gray-600 focus:border-blue-500 focus:outline-none"
                  />
                </div>
                <div className="p-4 bg-amber-500/10 border border-amber-500/20 rounded-xl">
                  <p className="text-sm text-amber-200">
                    <AlertTriangle className="w-4 h-4 inline mr-2" />
                    Keep your API keys secure. Never share them in public repositories or client-side code.
                  </p>
                </div>
              </div>
              <div className="flex justify-end gap-3 mt-6">
                <button
                  onClick={() => setShowCreate(false)}
                  className="px-4 py-2 text-gray-400 hover:text-white transition-colors"
                >
                  Cancel
                </button>
                <button
                  onClick={handleCreate}
                  disabled={!newKeyName}
                  className="px-6 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded-lg 
                    transition-colors disabled:opacity-50"
                >
                  Create
                </button>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Show Created Key */}
      <AnimatePresence>
        {createdKey && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm"
            onClick={() => setCreatedKey(null)}
          >
            <motion.div
              initial={{ opacity: 0, scale: 0.9 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.9 }}
              onClick={(e) => e.stopPropagation()}
              className="w-full max-w-lg glass-card rounded-2xl p-6"
            >
              <div className="text-center mb-6">
                <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-emerald-500/20 flex items-center justify-center">
                  <Key className="w-8 h-8 text-emerald-400" />
                </div>
                <h3 className="text-lg font-bold text-white">API Key Created</h3>
                <p className="text-sm text-gray-400">Copy this key now. You will not be able to see it again.</p>
              </div>
              <div className="p-4 bg-gray-900 rounded-lg mb-4">
                <code className="block text-sm text-white font-mono break-all">{createdKey}</code>
              </div>
              <button
                onClick={() => {
                  handleCopy(createdKey);
                  setCreatedKey(null);
                }}
                className="w-full py-2.5 bg-emerald-600 hover:bg-emerald-500 text-white rounded-lg transition-colors"
              >
                Copy & Close
              </button>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

// ============================================
// MAIN SECURITY SETTINGS COMPONENT
// ============================================

export function SecuritySettingsPanel() {
  const [activeTab, setActiveTab] = useState<"2fa" | "devices" | "history" | "api">("2fa");
  const [show2FASetup, setShow2FASetup] = useState(false);
  const [securitySettings, setSecuritySettings] = useState<SecuritySettings>({
    twoFactorEnabled: false,
    twoFactorMethod: null,
    biometricEnabled: false,
    hardwareKeyEnabled: false,
    sessionTimeout: 30,
    requirePasswordForTrading: true,
    ipWhitelist: [],
    apiKeyRotation: 90,
  });

  const tabs = [
    { id: "2fa" as const, name: "2FA & Auth", icon: Shield },
    { id: "devices" as const, name: "Devices", icon: Monitor },
    { id: "history" as const, name: "Login History", icon: History },
    { id: "api" as const, name: "API Keys", icon: Key },
  ];

  return (
    <div className="space-y-6">
      {/* Security Score */}
      <div className="p-6 rounded-2xl bg-gradient-to-r from-emerald-500/10 to-blue-500/10 border border-emerald-500/20">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <div className="w-14 h-14 rounded-xl bg-emerald-500/20 flex items-center justify-center">
              <ShieldCheck className="w-7 h-7 text-emerald-400" />
            </div>
            <div>
              <h3 className="text-lg font-bold text-white">Security Score: 85%</h3>
              <p className="text-sm text-gray-400">Good! Enable 2FA to reach 100%</p>
            </div>
          </div>
          <div className="w-32 h-2 bg-gray-700 rounded-full overflow-hidden">
            <div className="h-full w-[85%] bg-gradient-to-r from-emerald-500 to-blue-500 rounded-full" />
          </div>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-2 border-b border-gray-800">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`flex items-center gap-2 px-4 py-3 text-sm font-medium transition-colors
              ${activeTab === tab.id 
                ? "text-blue-400 border-b-2 border-blue-400" 
                : "text-gray-400 hover:text-white"}`}
          >
            <tab.icon className="w-4 h-4" />
            {tab.name}
          </button>
        ))}
      </div>

      {/* Tab Content */}
      <AnimatePresence mode="wait">
        {activeTab === "2fa" && !show2FASetup && (
          <motion.div
            key="2fa"
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
            className="space-y-4"
          >
            {/* 2FA Status */}
            <div className="p-4 rounded-xl bg-gray-800/30 border border-gray-700/50">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <div className={`w-10 h-10 rounded-lg flex items-center justify-center
                    ${securitySettings.twoFactorEnabled ? "bg-emerald-500/20" : "bg-amber-500/20"}`}>
                    {securitySettings.twoFactorEnabled ? (
                      <Lock className="w-5 h-5 text-emerald-400" />
                    ) : (
                      <Unlock className="w-5 h-5 text-amber-400" />
                    )}
                  </div>
                  <div>
                    <p className="font-medium text-white">Two-Factor Authentication</p>
                    <p className="text-sm text-gray-500">
                      {securitySettings.twoFactorEnabled 
                        ? `Enabled (${securitySettings.twoFactorMethod})`
                        : "Not enabled - your account is less secure"}
                    </p>
                  </div>
                </div>
                <button
                  onClick={() => setShow2FASetup(true)}
                  className={`px-4 py-2 rounded-lg font-medium transition-colors
                    ${securitySettings.twoFactorEnabled 
                      ? "bg-gray-700 text-gray-300 hover:bg-gray-600"
                      : "bg-blue-600 text-white hover:bg-blue-500"}`}
                >
                  {securitySettings.twoFactorEnabled ? "Manage" : "Enable"}
                </button>
              </div>
            </div>

            {/* Other Security Options */}
            {[
              { 
                key: "biometricEnabled", 
                name: "Biometric Authentication", 
                desc: "Use fingerprint or Face ID",
                icon: Fingerprint 
              },
              { 
                key: "requirePasswordForTrading", 
                name: "Password Required for Trading", 
                desc: "Extra confirmation before executing trades",
                icon: Lock 
              },
            ].map((option) => (
              <div
                key={option.key}
                className="flex items-center justify-between p-4 rounded-xl bg-gray-800/30 
                  border border-gray-700/50"
              >
                <div className="flex items-center gap-3">
                  <div className="w-10 h-10 rounded-lg bg-gray-700 flex items-center justify-center">
                    <option.icon className="w-5 h-5 text-gray-400" />
                  </div>
                  <div>
                    <p className="font-medium text-white">{option.name}</p>
                    <p className="text-sm text-gray-500">{option.desc}</p>
                  </div>
                </div>
                <label className="relative inline-flex items-center cursor-pointer">
                  <input
                    type="checkbox"
                    checked={securitySettings[option.key as keyof SecuritySettings] as boolean}
                    onChange={(e) => setSecuritySettings({
                      ...securitySettings,
                      [option.key]: e.target.checked
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
            ))}

            {/* Session Timeout */}
            <div className="p-4 rounded-xl bg-gray-800/30 border border-gray-700/50">
              <div className="flex items-center justify-between mb-3">
                <div className="flex items-center gap-3">
                  <div className="w-10 h-10 rounded-lg bg-gray-700 flex items-center justify-center">
                    <Clock className="w-5 h-5 text-gray-400" />
                  </div>
                  <div>
                    <p className="font-medium text-white">Session Timeout</p>
                    <p className="text-sm text-gray-500">Automatically sign out after inactivity</p>
                  </div>
                </div>
                <span className="text-lg font-bold text-white">{securitySettings.sessionTimeout} min</span>
              </div>
              <input
                type="range"
                min="5"
                max="120"
                value={securitySettings.sessionTimeout}
                onChange={(e) => setSecuritySettings({
                  ...securitySettings,
                  sessionTimeout: parseInt(e.target.value)
                })}
                className="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-blue-500"
              />
            </div>
          </motion.div>
        )}

        {activeTab === "2fa" && show2FASetup && (
          <motion.div
            key="2fa-setup"
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
          >
            <TwoFactorSetup
              onComplete={() => {
                setSecuritySettings({ ...securitySettings, twoFactorEnabled: true, twoFactorMethod: "totp" });
                setShow2FASetup(false);
              }}
              onCancel={() => setShow2FASetup(false)}
            />
          </motion.div>
        )}

        {activeTab === "devices" && (
          <motion.div
            key="devices"
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
          >
            <DeviceManagement />
          </motion.div>
        )}

        {activeTab === "history" && (
          <motion.div
            key="history"
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
          >
            <LoginHistory />
          </motion.div>
        )}

        {activeTab === "api" && (
          <motion.div
            key="api"
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
          >
            <ApiKeyManagement />
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

export default SecuritySettingsPanel;
