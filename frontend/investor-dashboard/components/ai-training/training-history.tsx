"use client";

import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  History,
  ChevronDown,
  ChevronUp,
  Calendar,
  Target,
  TrendingUp,
  Clock,
  CheckCircle2,
  XCircle,
  AlertCircle,
} from "lucide-react";
import type { TrainingSession } from "./ai-training-mode";

interface TrainingHistoryProps {
  sessions: TrainingSession[];
}

export function TrainingHistory({ sessions }: TrainingHistoryProps) {
  const [expandedId, setExpandedId] = useState<string | null>(null);

  if (sessions.length === 0) {
    return (
      <div className="glass-card rounded-xl p-8 text-center">
        <History className="w-12 h-12 text-gray-600 mx-auto mb-4" />
        <p className="text-gray-400">No training history yet.</p>
      </div>
    );
  }

  // Sort by date (newest first)
  const sortedSessions = [...sessions].sort((a, b) => {
    const dateA = a.startTime?.getTime() || 0;
    const dateB = b.startTime?.getTime() || 0;
    return dateB - dateA;
  });

  const getStatusIcon = (status: string) => {
    switch (status) {
      case "completed":
        return <CheckCircle2 className="w-4 h-4 text-emerald-400" />;
      case "error":
        return <XCircle className="w-4 h-4 text-rose-400" />;
      default:
        return <AlertCircle className="w-4 h-4 text-amber-400" />;
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case "completed":
        return "text-emerald-400 bg-emerald-500/10 border-emerald-500/30";
      case "error":
        return "text-rose-400 bg-rose-500/10 border-rose-500/30";
      default:
        return "text-amber-400 bg-amber-500/10 border-amber-500/30";
    }
  };

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-xl bg-gray-700/50 flex items-center justify-center">
            <History className="w-5 h-5 text-gray-400" />
          </div>
          <div>
            <h3 className="font-semibold text-white">Training History</h3>
            <p className="text-sm text-gray-500">Past training sessions</p>
          </div>
        </div>
        <span className="px-3 py-1 bg-gray-800 text-gray-400 text-sm rounded-full">
          {sessions.length} sessions
        </span>
      </div>

      {/* Session List */}
      <div className="space-y-2">
        {sortedSessions.map((session) => {
          const isExpanded = expandedId === session.id;
          const achievedTarget = session.bestConfidence >= session.config.targetConfidence;

          return (
            <motion.div
              key={session.id}
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              className={`rounded-xl border transition-colors ${getStatusColor(session.status)}`}
            >
              {/* Header Row */}
              <button
                onClick={() => setExpandedId(isExpanded ? null : session.id)}
                className="w-full p-4 flex items-center justify-between"
              >
                <div className="flex items-center gap-3">
                  {getStatusIcon(session.status)}
                  <div className="text-left">
                    <p className="font-medium">{session.name}</p>
                    <div className="flex items-center gap-2 text-xs opacity-70">
                      <Calendar className="w-3 h-3" />
                      {session.startTime?.toLocaleString()}
                    </div>
                  </div>
                </div>

                <div className="flex items-center gap-4">
                  <div className="text-right">
                    <p className="font-bold">{session.bestConfidence.toFixed(2)}%</p>
                    <p className="text-xs opacity-70">
                      {achievedTarget ? "Target achieved" : "Below target"}
                    </p>
                  </div>
                  {isExpanded ? (
                    <ChevronUp className="w-5 h-5" />
                  ) : (
                    <ChevronDown className="w-5 h-5" />
                  )}
                </div>
              </button>

              {/* Expanded Details */}
              <AnimatePresence>
                {isExpanded && (
                  <motion.div
                    initial={{ height: 0, opacity: 0 }}
                    animate={{ height: "auto", opacity: 1 }}
                    exit={{ height: 0, opacity: 0 }}
                    className="border-t border-current border-opacity-20"
                  >
                    <div className="p-4 space-y-3">
                      {/* Metrics Grid */}
                      <div className="grid grid-cols-3 gap-3">
                        <DetailItem
                          icon={Target}
                          label="Target Confidence"
                          value={`${session.config.targetConfidence}%`}
                        />
                        <DetailItem
                          icon={TrendingUp}
                          label="Best Epoch"
                          value={session.bestEpoch.toString()}
                        />
                        <DetailItem
                          icon={Clock}
                          label="Duration"
                          value={
                            session.startTime && session.endTime
                              ? formatDuration(session.startTime, session.endTime)
                              : "N/A"
                          }
                        />
                      </div>

                      {/* Configuration */}
                      <div className="pt-3 border-t border-current border-opacity-20">
                        <p className="text-xs font-medium mb-2 opacity-70">Configuration</p>
                        <div className="grid grid-cols-2 gap-2 text-xs">
                          <div className="flex justify-between">
                            <span className="opacity-50">Model Type</span>
                            <span className="font-medium">{session.config.modelType}</span>
                          </div>
                          <div className="flex justify-between">
                            <span className="opacity-50">Max Epochs</span>
                            <span className="font-medium">{session.config.maxEpochs}</span>
                          </div>
                          <div className="flex justify-between">
                            <span className="opacity-50">Dataset Size</span>
                            <span className="font-medium">
                              {session.config.datasetSize.toLocaleString()}
                            </span>
                          </div>
                          <div className="flex justify-between">
                            <span className="opacity-50">Learning Rate</span>
                            <span className="font-medium">{session.config.learningRate}</span>
                          </div>
                        </div>
                      </div>

                      {/* Error Message */}
                      {session.error && (
                        <div className="p-3 rounded-lg bg-rose-500/20 text-rose-300 text-sm">
                          <p className="font-medium">Error:</p>
                          <p>{session.error}</p>
                        </div>
                      )}
                    </div>
                  </motion.div>
                )}
              </AnimatePresence>
            </motion.div>
          );
        })}
      </div>
    </div>
  );
}

// Detail Item Component
function DetailItem({
  icon: Icon,
  label,
  value,
}: {
  icon: React.ElementType;
  label: string;
  value: string;
}) {
  return (
    <div className="flex items-center gap-2">
      <Icon className="w-4 h-4 opacity-50" />
      <div>
        <p className="text-xs opacity-50">{label}</p>
        <p className="font-medium">{value}</p>
      </div>
    </div>
  );
}

// Helper
function formatDuration(start: Date, end: Date): string {
  const diff = Math.floor((end.getTime() - start.getTime()) / 1000);
  const hours = Math.floor(diff / 3600);
  const mins = Math.floor((diff % 3600) / 60);
  const secs = diff % 60;

  const parts = [];
  if (hours > 0) parts.push(`${hours}h`);
  if (mins > 0) parts.push(`${mins}m`);
  if (secs > 0 || parts.length === 0) parts.push(`${secs}s`);

  return parts.join(" ");
}
