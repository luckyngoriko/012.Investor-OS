"use client";

import { useState } from "react";
import { motion } from "framer-motion";
import {
  Brain,
  Sparkles,
  Target,
  TrendingUp,
  Activity,
  GitCompare,
  History,
  Settings,
  Zap,
} from "lucide-react";
import { BackButton } from "@/components/back-button";
import {
  AITrainingMode,
  TrainingConfigurator,
  TrainingMonitor,
  MetricsDashboard,
  ModelComparison,
  TrainingHistory,
  type TrainingConfig,
  type TrainingSession,
  DEFAULT_TRAINING_CONFIG,
} from "@/components/ai-training";

// ============================================
// AI TRAIN PAGE
// ============================================

export default function AITrainPage() {
  const [activeTab, setActiveTab] = useState<"train" | "monitor" | "compare" | "history">("train");
  const [config, setConfig] = useState<TrainingConfig>(DEFAULT_TRAINING_CONFIG);
  const [sessions, setSessions] = useState<TrainingSession[]>([]);
  const [currentStatus, setCurrentStatus] = useState<"idle" | "running" | "paused" | "completed" | "error">("idle");
  const [currentMetrics, setCurrentMetrics] = useState<any[]>([]);
  const [currentEpoch, setCurrentEpoch] = useState(0);

  // Handle session update from training
  const handleSessionUpdate = (session: TrainingSession) => {
    setSessions((prev) => [session, ...prev]);
    setCurrentStatus(session.status);
  };

  // Handle status change from training component
  const handleStatusChange = (status: typeof currentStatus) => {
    setCurrentStatus(status);
  };

  // Handle metrics update
  const handleMetricsUpdate = (metrics: any[]) => {
    setCurrentMetrics(metrics);
  };

  // Handle epoch update
  const handleEpochUpdate = (epoch: number) => {
    setCurrentEpoch(epoch);
  };

  // Delete session
  const handleDeleteSession = (id: string) => {
    setSessions((prev) => prev.filter((s) => s.id !== id));
  };

  // Export session
  const handleExportSession = (session: TrainingSession) => {
    const dataStr = JSON.stringify(session, null, 2);
    const blob = new Blob([dataStr], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `training-session-${session.id}.json`;
    link.click();
  };

  // Select model
  const handleSelectModel = (session: TrainingSession) => {
    alert(`Selected model: ${session.name}\nConfidence: ${session.bestConfidence.toFixed(2)}%`);
  };

  const tabs = [
    { id: "train" as const, label: "Train", icon: Brain, color: "blue" },
    { id: "monitor" as const, label: "Monitor", icon: Activity, color: "emerald" },
    { id: "compare" as const, label: "Compare", icon: GitCompare, color: "purple" },
    { id: "history" as const, label: "History", icon: History, color: "gray" },
  ];

  const isRunning = currentStatus === "running";

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0f1c] via-[#111827] to-[#0a0f1c]">
      {/* Header */}
      <header className="border-b border-gray-800/50 bg-gray-900/50 backdrop-blur-sm sticky top-0 z-30">
        <div className="max-w-7xl mx-auto px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <BackButton />
              
              <div className="flex items-center gap-3">
                <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-violet-600 to-fuchsia-500 flex items-center justify-center shadow-lg shadow-violet-500/20">
                  <Brain className="w-6 h-6 text-white" />
                </div>
                <div>
                  <h1 className="text-2xl font-bold bg-gradient-to-r from-white to-gray-300 bg-clip-text text-transparent">
                    AI Train
                  </h1>
                  <p className="text-sm text-gray-500">Train models to target confidence</p>
                </div>
              </div>
            </div>

            {/* Status Badge */}
            <div className={`px-4 py-2 rounded-lg border ${getStatusStyles(currentStatus)}`}>
              <div className="flex items-center gap-2">
                <Zap className="w-4 h-4" />
                <span className="font-medium capitalize">{currentStatus}</span>
              </div>
            </div>
          </div>
        </div>
      </header>

      <main className="max-w-7xl mx-auto px-6 py-6">
        {/* Target Info Card */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="mb-6 p-6 rounded-2xl bg-gradient-to-r from-violet-500/10 via-fuchsia-500/10 to-blue-500/10 border border-violet-500/20"
        >
          <div className="flex flex-wrap items-center gap-8">
            <div className="flex items-center gap-3">
              <div className="w-12 h-12 rounded-xl bg-violet-500/20 flex items-center justify-center">
                <Target className="w-6 h-6 text-violet-400" />
              </div>
              <div>
                <p className="text-sm text-gray-400">Target Confidence</p>
                <p className="text-3xl font-bold text-white">{config.targetConfidence}%</p>
              </div>
            </div>

            <div className="w-px h-16 bg-gray-700 hidden md:block" />

            <div className="flex items-center gap-3">
              <div className="w-12 h-12 rounded-xl bg-emerald-500/20 flex items-center justify-center">
                <TrendingUp className="w-6 h-6 text-emerald-400" />
              </div>
              <div>
                <p className="text-sm text-gray-400">Best Achieved</p>
                <p className="text-3xl font-bold text-white">
                  {sessions.length > 0
                    ? `${Math.max(...sessions.map((s) => s.bestConfidence)).toFixed(2)}%`
                    : "--"}
                </p>
              </div>
            </div>

            <div className="w-px h-16 bg-gray-700 hidden md:block" />

            <div className="flex items-center gap-3">
              <div className="w-12 h-12 rounded-xl bg-blue-500/20 flex items-center justify-center">
                <Sparkles className="w-6 h-6 text-blue-400" />
              </div>
              <div>
                <p className="text-sm text-gray-400">Training Sessions</p>
                <p className="text-3xl font-bold text-white">{sessions.length}</p>
              </div>
            </div>

            <div className="w-px h-16 bg-gray-700 hidden md:block" />

            <div className="flex items-center gap-3">
              <div className="w-12 h-12 rounded-xl bg-amber-500/20 flex items-center justify-center">
                <Settings className="w-6 h-6 text-amber-400" />
              </div>
              <div>
                <p className="text-sm text-gray-400">Model Type</p>
                <p className="text-3xl font-bold text-white capitalize">{config.modelType}</p>
              </div>
            </div>
          </div>
        </motion.div>

        {/* Navigation Tabs */}
        <div className="flex gap-2 mb-6 overflow-x-auto pb-2">
          {tabs.map((tab) => {
            const Icon = tab.icon;
            const isActive = activeTab === tab.id;

            return (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={`flex items-center gap-2 px-6 py-3 rounded-xl font-medium transition-all whitespace-nowrap
                  ${isActive
                    ? `bg-${tab.color}-600 text-white shadow-lg shadow-${tab.color}-500/20`
                    : "bg-gray-800/50 text-gray-400 hover:text-white hover:bg-gray-800"
                  }
                `}
              >
                <Icon className="w-5 h-5" />
                {tab.label}
              </button>
            );
          })}
        </div>

        {/* Content */}
        <motion.div
          key={activeTab}
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.2 }}
        >
          {activeTab === "train" && (
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
              {/* Training Controls */}
              <div className="space-y-6">
                <AITrainingMode
                  initialConfig={config}
                  onSessionUpdate={handleSessionUpdate}
                />
              </div>

              {/* Configuration */}
              <div>
                <TrainingConfigurator
                  config={config}
                  onConfigChange={setConfig}
                  isRunning={isRunning}
                />
              </div>
            </div>
          )}

          {activeTab === "monitor" && (
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
              <div className="lg:col-span-1">
                <TrainingMonitor
                  status={currentStatus}
                  metrics={currentMetrics}
                  currentEpoch={currentEpoch}
                  maxEpochs={config.maxEpochs}
                />
              </div>
              <div className="lg:col-span-2">
                <MetricsDashboard metrics={currentMetrics} />
              </div>
            </div>
          )}

          {activeTab === "compare" && (
            <ModelComparison
              sessions={sessions}
              onSelectModel={handleSelectModel}
              onDeleteSession={handleDeleteSession}
              onExportSession={handleExportSession}
            />
          )}

          {activeTab === "history" && (
            <TrainingHistory sessions={sessions} />
          )}
        </motion.div>
      </main>
    </div>
  );
}

// Status styles helper
function getStatusStyles(status: string) {
  switch (status) {
    case "running":
      return "bg-emerald-500/20 text-emerald-400 border-emerald-500/30";
    case "paused":
      return "bg-amber-500/20 text-amber-400 border-amber-500/30";
    case "completed":
      return "bg-blue-500/20 text-blue-400 border-blue-500/30";
    case "error":
      return "bg-rose-500/20 text-rose-400 border-rose-500/30";
    default:
      return "bg-gray-500/20 text-gray-400 border-gray-500/30";
  }
}
