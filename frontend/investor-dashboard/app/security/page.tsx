"use client";

import { useEffect, useMemo, useState } from "react";
import { motion } from "framer-motion";
import {
  AlertTriangle,
  CheckCircle2,
  Copy,
  Eye,
  EyeOff,
  History,
  Key,
  Lock,
  RefreshCw,
  Shield,
  Smartphone,
  Trash2,
} from "lucide-react";
import Sidebar from "@/components/sidebar";
import {
  type ClearanceLevel,
  type ClearanceLevelsResponse,
  type GenerateApiKeyResponse,
  type SecurityStatusResponse,
  fetchClearanceLevels,
  fetchSecurityStatus,
  generateSecurityApiKey,
} from "@/lib/domain-api";

type SecurityTab = "overview" | "keys" | "2fa" | "audit";
type Severity = "info" | "warning";

interface SecurityAuditEvent {
  id: string;
  event: string;
  details: string;
  severity: Severity;
  timestamp: string;
}

interface SecurityApiKeyRow {
  id: string;
  name: string;
  clearance: string;
  created: string;
  lastUsed: string;
  status: "active";
}

const LEVEL_STYLE_MAP: Record<string, { card: string; label: string }> = {
  Public: {
    card: "bg-gray-500/10 border border-gray-500/30",
    label: "text-gray-300",
  },
  Internal: {
    card: "bg-blue-500/10 border border-blue-500/30",
    label: "text-blue-400",
  },
  Confidential: {
    card: "bg-amber-500/10 border border-amber-500/30",
    label: "text-amber-400",
  },
  Restricted: {
    card: "bg-orange-500/10 border border-orange-500/30",
    label: "text-orange-400",
  },
  TopSecret: {
    card: "bg-rose-500/10 border border-rose-500/30",
    label: "text-rose-400",
  },
};

function nowDateLabel(): string {
  return new Date().toISOString().slice(0, 10);
}

function nowTimestampLabel(): string {
  return new Date().toISOString().replace("T", " ").slice(0, 19);
}

function buildAuditEvents(status: SecurityStatusResponse | null): SecurityAuditEvent[] {
  if (!status) {
    return [];
  }

  const now = nowTimestampLabel();
  const featureEvents: SecurityAuditEvent[] = status.features.map((feature, index) => ({
    id: `evt_feature_${index}`,
    event: `${feature.name} Status`,
    details: feature.description,
    severity: (feature.name.includes("Policies") ? "warning" : "info") as Severity,
    timestamp: now,
  }));

  return [
    {
      id: "evt_module",
      event: "Security Module Health",
      details: `Module state: ${status.status}`,
      severity: status.status === "active" ? "info" : "warning",
      timestamp: now,
    },
    ...featureEvents,
  ];
}

function mapGeneratedKeyToRow(payload: GenerateApiKeyResponse): SecurityApiKeyRow {
  return {
    id: payload.key_id,
    name: `Generated Key ${payload.key_id.slice(0, 8)}`,
    clearance: payload.clearance,
    created: nowDateLabel(),
    lastUsed: "Just now",
    status: "active",
  };
}

function resolveClearanceLevels(
  payload: ClearanceLevelsResponse | null,
): ClearanceLevel[] {
  if (!payload) {
    return [];
  }
  return payload.levels;
}

export default function SecurityPage() {
  const [activeTab, setActiveTab] = useState<SecurityTab>("overview");
  const [statusPayload, setStatusPayload] = useState<SecurityStatusResponse | null>(null);
  const [clearancePayload, setClearancePayload] = useState<ClearanceLevelsResponse | null>(null);
  const [apiKeys, setApiKeys] = useState<SecurityApiKeyRow[]>([]);
  const [generatedKey, setGeneratedKey] = useState<string | null>(null);
  const [showKey, setShowKey] = useState(false);
  const [twoFactorEnabled, setTwoFactorEnabled] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [isGeneratingKey, setIsGeneratingKey] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;

    const loadSecurityData = async () => {
      setIsLoading(true);
      setErrorMessage(null);
      try {
        const [status, clearance] = await Promise.all([
          fetchSecurityStatus(),
          fetchClearanceLevels(),
        ]);
        if (!mounted) return;

        setStatusPayload(status);
        setClearancePayload(clearance);
      } catch (error) {
        if (!mounted) return;
        setErrorMessage(
          error instanceof Error ? error.message : "Failed to load security data",
        );
      } finally {
        if (mounted) {
          setIsLoading(false);
        }
      }
    };

    void loadSecurityData();

    return () => {
      mounted = false;
    };
  }, []);

  const auditEvents = useMemo(
    () => buildAuditEvents(statusPayload),
    [statusPayload],
  );
  const clearanceLevels = useMemo(
    () => resolveClearanceLevels(clearancePayload),
    [clearancePayload],
  );
  const infoEvents = auditEvents.filter((event) => event.severity === "info").length;
  const warningEvents = auditEvents.filter((event) => event.severity === "warning").length;

  const encryptionFeature = statusPayload?.features.find((feature) =>
    feature.name.includes("Encryption"),
  );
  const twoFactorFeature = statusPayload?.features.find((feature) =>
    feature.name.includes("Two-Factor"),
  );

  const handleGenerateKey = async () => {
    setIsGeneratingKey(true);
    setErrorMessage(null);

    try {
      const payload = await generateSecurityApiKey();
      setGeneratedKey(payload.api_key);
      setShowKey(true);
      setApiKeys((current) => [mapGeneratedKeyToRow(payload), ...current]);
    } catch (error) {
      setErrorMessage(
        error instanceof Error ? error.message : "Failed to generate API key",
      );
    } finally {
      setIsGeneratingKey(false);
    }
  };

  const copyToClipboard = (text: string) => {
    void navigator.clipboard.writeText(text);
  };

  const removeKey = (id: string) => {
    setApiKeys((current) => current.filter((row) => row.id !== id));
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
                <p className="text-gray-400 text-sm">
                  Backend-driven security status, keys, and audit visibility
                </p>
              </div>
            </div>
          </motion.div>

          {errorMessage && (
            <div className="rounded-xl border border-rose-500/40 bg-rose-500/10 p-4 text-sm text-rose-300">
              {errorMessage}
            </div>
          )}

          <div className="flex gap-2 p-1 bg-gray-800/30 rounded-xl w-fit">
            {[
              { id: "overview", label: "Overview", icon: Shield },
              { id: "keys", label: "API Keys", icon: Key },
              { id: "2fa", label: "2FA", icon: Smartphone },
              { id: "audit", label: "Audit Log", icon: History },
            ].map((tab) => (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id as SecurityTab)}
                className={`flex items-center gap-2 px-4 py-2 rounded-lg transition-all ${
                  activeTab === tab.id
                    ? "bg-blue-600 text-white"
                    : "text-gray-400 hover:text-white hover:bg-gray-700/50"
                }`}
              >
                <tab.icon className="w-4 h-4" />
                <span className="text-sm font-medium">{tab.label}</span>
              </button>
            ))}
          </div>

          {isLoading ? (
            <div className="glass-card rounded-2xl p-6 text-gray-300 flex items-center gap-3">
              <RefreshCw className="w-4 h-4 animate-spin" />
              Loading security data...
            </div>
          ) : null}

          {!isLoading && activeTab === "overview" && (
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4"
            >
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
                <p className="text-xs text-gray-500">
                  {encryptionFeature?.rotation_interval
                    ? `Rotation: ${encryptionFeature.rotation_interval}`
                    : "Rotation policy available"}
                </p>
              </div>

              <div className="glass-card rounded-2xl p-6">
                <div className="flex items-center gap-3 mb-4">
                  <div className="w-10 h-10 rounded-xl bg-blue-500/10 flex items-center justify-center">
                    <Key className="w-5 h-5 text-blue-400" />
                  </div>
                  <div>
                    <p className="text-sm text-gray-400">Active API Keys</p>
                    <p className="text-lg font-bold text-white">{apiKeys.length}</p>
                  </div>
                </div>
                <p className="text-xs text-gray-500">Generated through backend API</p>
              </div>

              <div className="glass-card rounded-2xl p-6">
                <div className="flex items-center gap-3 mb-4">
                  <div className="w-10 h-10 rounded-xl bg-purple-500/10 flex items-center justify-center">
                    <Smartphone className="w-5 h-5 text-purple-400" />
                  </div>
                  <div>
                    <p className="text-sm text-gray-400">2FA Status</p>
                    <p className="text-lg font-bold text-white">
                      {twoFactorEnabled ? "Enabled" : "Disabled"}
                    </p>
                  </div>
                </div>
                <p className="text-xs text-gray-500">
                  {twoFactorFeature?.methods?.join(", ") ?? "Methods exposed by backend"}
                </p>
              </div>

              <div className="glass-card rounded-2xl p-6">
                <div className="flex items-center gap-3 mb-4">
                  <div className="w-10 h-10 rounded-xl bg-amber-500/10 flex items-center justify-center">
                    <RefreshCw className="w-5 h-5 text-amber-400" />
                  </div>
                  <div>
                    <p className="text-sm text-gray-400">Module Status</p>
                    <p className="text-lg font-bold text-white">
                      {statusPayload?.status ?? "unknown"}
                    </p>
                  </div>
                </div>
                <p className="text-xs text-gray-500">Sourced from `/api/security/status`</p>
              </div>

              <div className="md:col-span-2 lg:col-span-4 glass-card rounded-2xl p-6">
                <h3 className="text-lg font-semibold text-white mb-4">Clearance Levels</h3>
                <div className="grid grid-cols-1 md:grid-cols-5 gap-3">
                  {clearanceLevels.map((level) => {
                    const style = LEVEL_STYLE_MAP[level.name] ?? LEVEL_STYLE_MAP.Public;
                    return (
                      <div key={level.name} className={`p-4 rounded-xl ${style.card}`}>
                        <div className="flex items-center justify-between mb-2">
                          <span className={`text-sm font-bold ${style.label}`}>{level.name}</span>
                          {level["2fa_required"] ? (
                            <Lock className="w-3 h-3 text-amber-400" />
                          ) : null}
                        </div>
                        <p className="text-xs text-gray-400">{level.description}</p>
                      </div>
                    );
                  })}
                </div>
              </div>
            </motion.div>
          )}

          {!isLoading && activeTab === "keys" && (
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              className="space-y-6"
            >
              <div className="glass-card rounded-2xl p-6">
                <h3 className="text-lg font-semibold text-white mb-4">Generate API Key</h3>
                <div className="flex flex-col md:flex-row gap-4">
                  <button
                    onClick={handleGenerateKey}
                    disabled={isGeneratingKey}
                    className="px-6 py-2 bg-blue-600 hover:bg-blue-500 disabled:bg-gray-700 text-white font-medium rounded-lg transition-colors flex items-center justify-center gap-2"
                  >
                    {isGeneratingKey ? (
                      <>
                        <RefreshCw className="w-4 h-4 animate-spin" />
                        Generating...
                      </>
                    ) : (
                      "Generate Key"
                    )}
                  </button>
                </div>

                {generatedKey && (
                  <div className="mt-4 p-4 bg-amber-500/10 border border-amber-500/30 rounded-xl">
                    <div className="flex items-center gap-2 mb-2">
                      <AlertTriangle className="w-5 h-5 text-amber-400" />
                      <span className="text-amber-400 font-medium">
                        Copy this key now. It is shown only once.
                      </span>
                    </div>
                    <div className="flex items-center gap-2">
                      <code className="flex-1 p-3 bg-gray-900 rounded-lg text-green-400 font-mono text-sm">
                        {showKey ? generatedKey : "ios_" + "*".repeat(32)}
                      </code>
                      <button
                        onClick={() => setShowKey((current) => !current)}
                        className="p-2 hover:bg-gray-800 rounded-lg text-gray-400"
                      >
                        {showKey ? <EyeOff className="w-5 h-5" /> : <Eye className="w-5 h-5" />}
                      </button>
                      <button
                        onClick={() => copyToClipboard(generatedKey)}
                        className="p-2 hover:bg-gray-800 rounded-lg text-gray-400"
                      >
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
                {apiKeys.length === 0 ? (
                  <div className="p-6 text-sm text-gray-400">No keys generated yet.</div>
                ) : (
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
                      {apiKeys.map((row) => (
                        <tr key={row.id} className="border-t border-gray-800">
                          <td className="px-6 py-4 text-white">{row.name}</td>
                          <td className="px-6 py-4">
                            <span className="px-2 py-1 text-xs rounded-full bg-blue-500/20 text-blue-400">
                              {row.clearance}
                            </span>
                          </td>
                          <td className="px-6 py-4 text-gray-400">{row.created}</td>
                          <td className="px-6 py-4 text-gray-400">{row.lastUsed}</td>
                          <td className="px-6 py-4">
                            <button
                              onClick={() => removeKey(row.id)}
                              className="p-2 hover:bg-rose-500/10 rounded-lg text-rose-400"
                            >
                              <Trash2 className="w-4 h-4" />
                            </button>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
              </div>
            </motion.div>
          )}

          {!isLoading && activeTab === "2fa" && (
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              className="glass-card rounded-2xl p-6"
            >
              <div className="flex items-center justify-between mb-6">
                <div>
                  <h3 className="text-lg font-semibold text-white">Two-Factor Authentication</h3>
                  <p className="text-gray-400">Add an extra layer of account protection</p>
                </div>
                <button
                  onClick={() => setTwoFactorEnabled((current) => !current)}
                  className={`px-4 py-2 rounded-lg font-medium transition-colors ${
                    twoFactorEnabled
                      ? "bg-rose-500/20 text-rose-400 hover:bg-rose-500/30"
                      : "bg-emerald-500/20 text-emerald-400 hover:bg-emerald-500/30"
                  }`}
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
                      <p className="text-sm text-gray-400">Runtime toggle for secure operations</p>
                    </div>
                  </div>
                </div>
              ) : (
                <div className="text-center py-12">
                  <div className="w-16 h-16 rounded-full bg-gray-800 flex items-center justify-center mx-auto mb-4">
                    <Smartphone className="w-8 h-8 text-gray-400" />
                  </div>
                  <p className="text-gray-400 mb-4">Enable 2FA for stronger account security</p>
                  <p className="text-sm text-gray-500">
                    {twoFactorFeature?.methods?.join(", ") ?? "TOTP, HOTP, WebAuthn, SMS, Email"}
                  </p>
                </div>
              )}
            </motion.div>
          )}

          {!isLoading && activeTab === "audit" && (
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              className="glass-card rounded-2xl overflow-hidden"
            >
              <div className="p-6 border-b border-gray-800 flex items-center justify-between">
                <div>
                  <h3 className="text-lg font-semibold text-white">Security Audit Log</h3>
                  <p className="text-gray-400 text-sm">
                    Backend-derived security activity snapshot
                  </p>
                </div>
                <div className="flex gap-2">
                  <span className="px-3 py-1 text-xs rounded-full bg-emerald-500/20 text-emerald-400">
                    {infoEvents} Info
                  </span>
                  <span className="px-3 py-1 text-xs rounded-full bg-amber-500/20 text-amber-400">
                    {warningEvents} Warning
                  </span>
                </div>
              </div>

              {auditEvents.length === 0 ? (
                <div className="p-6 text-sm text-gray-400">No audit events available.</div>
              ) : (
                <table className="w-full">
                  <thead>
                    <tr className="text-left text-xs text-gray-500 uppercase">
                      <th className="px-6 py-4">Event</th>
                      <th className="px-6 py-4">Details</th>
                      <th className="px-6 py-4">Severity</th>
                      <th className="px-6 py-4">Timestamp</th>
                    </tr>
                  </thead>
                  <tbody>
                    {auditEvents.map((event) => (
                      <tr key={event.id} className="border-t border-gray-800">
                        <td className="px-6 py-4 text-white">{event.event}</td>
                        <td className="px-6 py-4 text-gray-400">{event.details}</td>
                        <td className="px-6 py-4">
                          <span
                            className={`px-2 py-1 text-xs rounded-full ${
                              event.severity === "info"
                                ? "bg-emerald-500/20 text-emerald-400"
                                : "bg-amber-500/20 text-amber-400"
                            }`}
                          >
                            {event.severity}
                          </span>
                        </td>
                        <td className="px-6 py-4 text-gray-400">{event.timestamp}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </motion.div>
          )}
        </div>
      </main>
    </div>
  );
}
