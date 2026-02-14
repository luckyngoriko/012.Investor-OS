"use client";

import { useState } from "react";
import { motion } from "framer-motion";
import { 
  Lock, Shield, Key, RefreshCw, CheckCircle2, AlertTriangle,
  Eye, EyeOff, Copy, Trash2, Smartphone, History
} from "lucide-react";
import Sidebar from "@/components/sidebar";

const mockApiKeys = [
  { id: "key_1", name: "Trading API", clearance: "Internal", created: "2026-01-15", lastUsed: "2026-02-10", status: "active" },
  { id: "key_2", name: "Dashboard Access", clearance: "Confidential", created: "2026-01-20", lastUsed: "2026-02-09", status: "active" },
];

const mockAuditEvents = [
  { id: 1, event: "Login Success", user: "trader@example.com", ip: "192.168.1.100", timestamp: "2026-02-10 14:30:22", severity: "info" },
  { id: 2, event: "API Key Created", user: "trader@example.com", ip: "192.168.1.100", timestamp: "2026-02-10 14:25:15", severity: "info" },
  { id: 3, event: "2FA Verified", user: "trader@example.com", ip: "192.168.1.100", timestamp: "2026-02-10 14:20:08", severity: "info" },
  { id: 4, event: "Login Failed", user: "unknown", ip: "10.0.0.50", timestamp: "2026-02-10 13:45:33", severity: "warning" },
];

export default function SecurityPage() {
  const [activeTab, setActiveTab] = useState<"overview" | "keys" | "2fa" | "audit">("overview");
  const [generatedKey, setGeneratedKey] = useState<string | null>(null);
  const [showKey, setShowKey] = useState(false);
  const [twoFactorEnabled, setTwoFactorEnabled] = useState(false);

  const handleGenerateKey = async () => {
    const mockKey = "ios_" + Math.random().toString(36).substring(2, 15) + Math.random().toString(36).substring(2, 15);
    setGeneratedKey(mockKey);
    setShowKey(true);
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    alert("Copied to clipboard!");
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0f1c] via-[#111827] to-[#0a0f1c] flex">
      <Sidebar />
      <main className="flex-1 min-h-screen p-6 lg:p-8">
        <div className="max-w-7xl mx-auto space-y-6">
          <motion.div initial={{ opacity: 0, y: -20 }} animate={{ opacity: 1, y: 0 }}>
            <div className="flex items-center gap-3 mb-2">
              <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-emerald-500/20 to-emerald-600/10 flex items-center justify-center">
                <Lock className="w-5 h-5 text-emerald-400" />
              </div>
              <div>
                <h1 className="text-2xl font-bold text-white">Security & Encryption</h1>
                <p className="text-gray-400 text-sm">Sprint 34: HSM-backed API keys, 2FA, and audit trails</p>
              </div>
            </div>
          </motion.div>

          <div className="flex gap-2 p-1 bg-gray-800/30 rounded-xl w-fit">
            {[
              { id: "overview", label: "Overview", icon: Shield },
              { id: "keys", label: "API Keys", icon: Key },
              { id: "2fa", label: "2FA", icon: Smartphone },
              { id: "audit", label: "Audit Log", icon: History },
            ].map((tab) => (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id as any)}
                className={`flex items-center gap-2 px-4 py-2 rounded-lg transition-all
                  ${activeTab === tab.id ? "bg-blue-600 text-white" : "text-gray-400 hover:text-white hover:bg-gray-700/50"}`}
              >
                <tab.icon className="w-4 h-4" />
                <span className="text-sm font-medium">{tab.label}</span>
              </button>
            ))}
          </div>

          {activeTab === "overview" && (
            <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
              <div className="glass-card rounded-2xl p-6">
                <div className="flex items-center gap-3 mb-4">
                  <div className="w-10 h-10 rounded-xl bg-emerald-500/10 flex items-center justify-center">
                    <Shield className="w-5 h-5 text-emerald-400" />
                  </div>
                  <div>
                    <p className="text-sm text-gray-400">Encryption</p>
                    <p className="text-lg font-bold text-white">AES-256-GCM</p>
                  </div>
                </div>
                <p className="text-xs text-gray-500">HSM-backed key storage</p>
              </div>

              <div className="glass-card rounded-2xl p-6">
                <div className="flex items-center gap-3 mb-4">
                  <div className="w-10 h-10 rounded-xl bg-blue-500/10 flex items-center justify-center">
                    <Key className="w-5 h-5 text-blue-400" />
                  </div>
                  <div>
                    <p className="text-sm text-gray-400">Active API Keys</p>
                    <p className="text-lg font-bold text-white">{mockApiKeys.length}</p>
                  </div>
                </div>
                <p className="text-xs text-gray-500">Across all clearance levels</p>
              </div>

              <div className="glass-card rounded-2xl p-6">
                <div className="flex items-center gap-3 mb-4">
                  <div className="w-10 h-10 rounded-xl bg-purple-500/10 flex items-center justify-center">
                    <Smartphone className="w-5 h-5 text-purple-400" />
                  </div>
                  <div>
                    <p className="text-sm text-gray-400">2FA Status</p>
                    <p className="text-lg font-bold text-white">{twoFactorEnabled ? "Enabled" : "Disabled"}</p>
                  </div>
                </div>
                <p className="text-xs text-gray-500">TOTP/HOTP supported</p>
              </div>

              <div className="glass-card rounded-2xl p-6">
                <div className="flex items-center gap-3 mb-4">
                  <div className="w-10 h-10 rounded-xl bg-amber-500/10 flex items-center justify-center">
                    <RefreshCw className="w-5 h-5 text-amber-400" />
                  </div>
                  <div>
                    <p className="text-sm text-gray-400">Key Rotation</p>
                    <p className="text-lg font-bold text-white">Auto (90d)</p>
                  </div>
                </div>
                <p className="text-xs text-gray-500">Grace period: 7 days</p>
              </div>

              <div className="md:col-span-2 lg:col-span-4 glass-card rounded-2xl p-6">
                <h3 className="text-lg font-semibold text-white mb-4">Clearance Levels</h3>
                <div className="grid grid-cols-1 md:grid-cols-5 gap-3">
                  {[
                    { level: "Public", value: 0, desc: "Basic read access", color: "gray" },
                    { level: "Internal", value: 1, desc: "Standard trading", color: "blue" },
                    { level: "Confidential", value: 2, desc: "Sensitive data", color: "amber", requires2fa: true },
                    { level: "Restricted", value: 3, desc: "High-value tx", color: "orange", requires2fa: true },
                    { level: "TopSecret", value: 4, desc: "Admin only", color: "red", requires2fa: true },
                  ].map((item) => (
                    <div key={item.level} className={`p-4 rounded-xl bg-${item.color}-500/10 border border-${item.color}-500/30`}>
                      <div className="flex items-center justify-between mb-2">
                        <span className={`text-sm font-bold text-${item.color}-400`}>{item.level}</span>
                        {item.requires2fa && <Lock className="w-3 h-3 text-amber-400" />}
                      </div>
                      <p className="text-xs text-gray-400">{item.desc}</p>
                    </div>
                  ))}
                </div>
              </div>
            </motion.div>
          )}

          {activeTab === "keys" && (
            <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="space-y-6">
              <div className="glass-card rounded-2xl p-6">
                <h3 className="text-lg font-semibold text-white mb-4">Generate API Key</h3>
                <div className="flex flex-col md:flex-row gap-4">
                  <select className="flex-1 px-4 py-2 bg-gray-800 border border-gray-700 rounded-lg text-white">
                    <option>Internal - Standard trading operations</option>
                    <option>Confidential - Sensitive positions</option>
                    <option>Restricted - High-value transactions</option>
                  </select>
                  <button onClick={handleGenerateKey} className="px-6 py-2 bg-blue-600 hover:bg-blue-500 text-white font-medium rounded-lg transition-colors">
                    Generate Key
                  </button>
                </div>

                {generatedKey && (
                  <div className="mt-4 p-4 bg-amber-500/10 border border-amber-500/30 rounded-xl">
                    <div className="flex items-center gap-2 mb-2">
                      <AlertTriangle className="w-5 h-5 text-amber-400" />
                      <span className="text-amber-400 font-medium">Copy this key now! It will not be shown again.</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <code className="flex-1 p-3 bg-gray-900 rounded-lg text-green-400 font-mono text-sm">
                        {showKey ? generatedKey : "ios_" + "*".repeat(32)}
                      </code>
                      <button onClick={() => setShowKey(!showKey)} className="p-2 hover:bg-gray-800 rounded-lg text-gray-400">
                        {showKey ? <EyeOff className="w-5 h-5" /> : <Eye className="w-5 h-5" />}
                      </button>
                      <button onClick={() => copyToClipboard(generatedKey)} className="p-2 hover:bg-gray-800 rounded-lg text-gray-400">
                        <Copy className="w-5 h-5" />
                      </button>
                    </div>
                  </div>
                )}
              </div>

              <div className="glass-card rounded-2xl overflow-hidden">
                <div className="p-6 border-b border-gray-800">
                  <h3 className="text-lg font-semibold text-white">Active API Keys</h3>
                </div>
                <table className="w-full">
                  <thead>
                    <tr className="text-left text-xs text-gray-500 uppercase">
                      <th className="px-6 py-4">Name</th>
                      <th className="px-6 py-4">Clearance</th>
                      <th className="px-6 py-4">Created</th>
                      <th className="px-6 py-4">Last Used</th>
                      <th className="px-6 py-4">Actions</th>
                    </tr>
                  </thead>
                  <tbody>
                    {mockApiKeys.map((key) => (
                      <tr key={key.id} className="border-t border-gray-800">
                        <td className="px-6 py-4 text-white">{key.name}</td>
                        <td className="px-6 py-4">
                          <span className="px-2 py-1 text-xs rounded-full bg-blue-500/20 text-blue-400">{key.clearance}</span>
                        </td>
                        <td className="px-6 py-4 text-gray-400">{key.created}</td>
                        <td className="px-6 py-4 text-gray-400">{key.lastUsed}</td>
                        <td className="px-6 py-4">
                          <button className="p-2 hover:bg-rose-500/10 rounded-lg text-rose-400">
                            <Trash2 className="w-4 h-4" />
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </motion.div>
          )}

          {activeTab === "2fa" && (
            <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="glass-card rounded-2xl p-6">
              <div className="flex items-center justify-between mb-6">
                <div>
                  <h3 className="text-lg font-semibold text-white">Two-Factor Authentication</h3>
                  <p className="text-gray-400">Add an extra layer of security to your account</p>
                </div>
                <button
                  onClick={() => setTwoFactorEnabled(!twoFactorEnabled)}
                  className={`px-4 py-2 rounded-lg font-medium transition-colors
                    ${twoFactorEnabled ? "bg-rose-500/20 text-rose-400 hover:bg-rose-500/30" : "bg-emerald-500/20 text-emerald-400 hover:bg-emerald-500/30"}`}
                >
                  {twoFactorEnabled ? "Disable 2FA" : "Enable 2FA"}
                </button>
              </div>

              {twoFactorEnabled ? (
                <div className="space-y-4">
                  <div className="flex items-center gap-3 p-4 bg-emerald-500/10 rounded-xl">
                    <CheckCircle2 className="w-6 h-6 text-emerald-400" />
                    <div>
                      <p className="text-white font-medium">2FA is enabled</p>
                      <p className="text-sm text-gray-400">Your account is protected with TOTP</p>
                    </div>
                  </div>
                </div>
              ) : (
                <div className="text-center py-12">
                  <div className="w-16 h-16 rounded-full bg-gray-800 flex items-center justify-center mx-auto mb-4">
                    <Smartphone className="w-8 h-8 text-gray-400" />
                  </div>
                  <p className="text-gray-400 mb-4">Enable 2FA to protect your account</p>
                  <p className="text-sm text-gray-500">Supported: TOTP, HOTP, WebAuthn, SMS, Email</p>
                </div>
              )}
            </motion.div>
          )}

          {activeTab === "audit" && (
            <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="glass-card rounded-2xl overflow-hidden">
              <div className="p-6 border-b border-gray-800 flex items-center justify-between">
                <div>
                  <h3 className="text-lg font-semibold text-white">Security Audit Log</h3>
                  <p className="text-gray-400 text-sm">Immutable event logging with tamper detection</p>
                </div>
                <div className="flex gap-2">
                  <span className="px-3 py-1 text-xs rounded-full bg-emerald-500/20 text-emerald-400">
                    {mockAuditEvents.filter(e => e.severity === "info").length} Info
                  </span>
                  <span className="px-3 py-1 text-xs rounded-full bg-amber-500/20 text-amber-400">
                    {mockAuditEvents.filter(e => e.severity === "warning").length} Warning
                  </span>
                </div>
              </div>
              <table className="w-full">
                <thead>
                  <tr className="text-left text-xs text-gray-500 uppercase">
                    <th className="px-6 py-4">Event</th>
                    <th className="px-6 py-4">User</th>
                    <th className="px-6 py-4">IP Address</th>
                    <th className="px-6 py-4">Timestamp</th>
                    <th className="px-6 py-4">Severity</th>
                  </tr>
                </thead>
                <tbody>
                  {mockAuditEvents.map((event) => (
                    <tr key={event.id} className="border-t border-gray-800">
                      <td className="px-6 py-4 text-white">{event.event}</td>
                      <td className="px-6 py-4 text-gray-400">{event.user}</td>
                      <td className="px-6 py-4 text-gray-400 font-mono">{event.ip}</td>
                      <td className="px-6 py-4 text-gray-400">{event.timestamp}</td>
                      <td className="px-6 py-4">
                        <span className={`px-2 py-1 text-xs rounded-full
                          ${event.severity === "info" ? "bg-blue-500/20 text-blue-400" : ""}
                          ${event.severity === "warning" ? "bg-amber-500/20 text-amber-400" : ""}`}>
                          {event.severity}
                        </span>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </motion.div>
          )}
        </div>
      </main>
    </div>
  );
}
