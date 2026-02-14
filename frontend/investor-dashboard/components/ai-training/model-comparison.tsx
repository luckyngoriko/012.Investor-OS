"use client";

import { motion } from "framer-motion";
import {
  GitCompare,
  Trophy,
  Clock,
  Target,
  TrendingUp,
  CheckCircle2,
  Download,
  Trash2,
} from "lucide-react";
import type { TrainingSession } from "./ai-training-mode";

interface ModelComparisonProps {
  sessions: TrainingSession[];
  onSelectModel?: (session: TrainingSession) => void;
  onDeleteSession?: (id: string) => void;
  onExportSession?: (session: TrainingSession) => void;
}

export function ModelComparison({
  sessions,
  onSelectModel,
  onDeleteSession,
  onExportSession,
}: ModelComparisonProps) {
  if (sessions.length === 0) {
    return (
      <div className="glass-card rounded-xl p-8 text-center">
        <GitCompare className="w-12 h-12 text-gray-600 mx-auto mb-4" />
        <p className="text-gray-400">No training sessions yet.</p>
        <p className="text-sm text-gray-500 mt-1">Complete training runs to compare models.</p>
      </div>
    );
  }

  // Sort by best confidence
  const sortedSessions = [...sessions].sort((a, b) => b.bestConfidence - a.bestConfidence);
  const bestSession = sortedSessions[0];

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-xl bg-purple-500/10 flex items-center justify-center">
            <GitCompare className="w-5 h-5 text-purple-400" />
          </div>
          <div>
            <h3 className="font-semibold text-white">Model Comparison</h3>
            <p className="text-sm text-gray-500">{sessions.length} training sessions</p>
          </div>
        </div>
      </div>

      {/* Best Model Highlight */}
      {bestSession && (
        <motion.div
          initial={{ opacity: 0, scale: 0.98 }}
          animate={{ opacity: 1, scale: 1 }}
          className="p-5 rounded-2xl bg-gradient-to-r from-emerald-500/10 to-blue-500/10 border border-emerald-500/30"
        >
          <div className="flex items-start justify-between">
            <div className="flex items-center gap-3">
              <div className="w-12 h-12 rounded-xl bg-emerald-500/20 flex items-center justify-center">
                <Trophy className="w-6 h-6 text-emerald-400" />
              </div>
              <div>
                <div className="flex items-center gap-2">
                  <span className="px-2 py-0.5 bg-emerald-500 text-white text-xs font-bold rounded">
                    BEST
                  </span>
                  <h4 className="font-semibold text-white">{bestSession.name}</h4>
                </div>
                <p className="text-sm text-gray-400 mt-1">
                  {bestSession.config.modelType.toUpperCase()} • {bestSession.bestEpoch} epochs
                </p>
              </div>
            </div>

            <div className="text-right">
              <p className="text-3xl font-bold text-emerald-400">
                {bestSession.bestConfidence.toFixed(2)}%
              </p>
              <p className="text-sm text-gray-500">Best Confidence</p>
            </div>
          </div>

          <div className="grid grid-cols-3 gap-4 mt-4 pt-4 border-t border-emerald-500/20">
            <div>
              <p className="text-xs text-gray-500">Training Duration</p>
              <p className="text-white font-medium">
                {bestSession.startTime && bestSession.endTime
                  ? formatDuration(bestSession.startTime, bestSession.endTime)
                  : "N/A"}
              </p>
            </div>
            <div>
              <p className="text-xs text-gray-500">Dataset Size</p>
              <p className="text-white font-medium">
                {bestSession.config.datasetSize.toLocaleString()}
              </p>
            </div>
            <div>
              <p className="text-xs text-gray-500">Target</p>
              <p className="text-white font-medium">{bestSession.config.targetConfidence}%</p>
            </div>
          </div>

          <div className="flex gap-2 mt-4">
            {onSelectModel && (
              <button
                onClick={() => onSelectModel(bestSession)}
                className="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 text-white text-sm font-medium rounded-lg transition-colors"
              >
                <CheckCircle2 className="w-4 h-4" />
                Use This Model
              </button>
            )}
            {onExportSession && (
              <button
                onClick={() => onExportSession(bestSession)}
                className="flex items-center gap-2 px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white text-sm font-medium rounded-lg transition-colors"
              >
                <Download className="w-4 h-4" />
                Export
              </button>
            )}
          </div>
        </motion.div>
      )}

      {/* Comparison Table */}
      <div className="glass-card rounded-xl overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead>
              <tr className="bg-gray-800/50 text-left">
                <th className="px-4 py-3 text-xs font-medium text-gray-400">Model</th>
                <th className="px-4 py-3 text-xs font-medium text-gray-400">Type</th>
                <th className="px-4 py-3 text-xs font-medium text-gray-400 text-center">
                  <div className="flex items-center justify-center gap-1">
                    <Target className="w-3 h-3" />
                    Confidence
                  </div>
                </th>
                <th className="px-4 py-3 text-xs font-medium text-gray-400 text-center">
                  <div className="flex items-center justify-center gap-1">
                    <TrendingUp className="w-3 h-3" />
                    Epochs
                  </div>
                </th>
                <th className="px-4 py-3 text-xs font-medium text-gray-400 text-center">
                  <div className="flex items-center justify-center gap-1">
                    <Clock className="w-3 h-3" />
                    Duration
                  </div>
                </th>
                <th className="px-4 py-3 text-xs font-medium text-gray-400 text-right">Actions</th>
              </tr>
            </thead>
            <tbody>
              {sortedSessions.map((session, idx) => (
                <tr
                  key={session.id}
                  className={`border-t border-gray-800 hover:bg-gray-800/30 transition-colors
                    ${session.id === bestSession?.id ? "bg-emerald-500/5" : ""}
                  `}
                >
                  <td className="px-4 py-3">
                    <div className="flex items-center gap-2">
                      {session.id === bestSession?.id && (
                        <Trophy className="w-4 h-4 text-emerald-400" />
                      )}
                      <div>
                        <p className="text-sm font-medium text-white">{session.name}</p>
                        <p className="text-xs text-gray-500">
                          {session.startTime?.toLocaleDateString()}
                        </p>
                      </div>
                    </div>
                  </td>
                  <td className="px-4 py-3">
                    <span className="px-2 py-1 bg-gray-700 text-gray-300 text-xs rounded">
                      {session.config.modelType}
                    </span>
                  </td>
                  <td className="px-4 py-3 text-center">
                    <span
                      className={`text-sm font-bold ${
                        session.bestConfidence >= session.config.targetConfidence
                          ? "text-emerald-400"
                          : "text-amber-400"
                      }`}
                    >
                      {session.bestConfidence.toFixed(2)}%
                    </span>
                  </td>
                  <td className="px-4 py-3 text-center text-sm text-gray-300">
                    {session.bestEpoch}
                  </td>
                  <td className="px-4 py-3 text-center text-sm text-gray-300">
                    {session.startTime && session.endTime
                      ? formatDuration(session.startTime, session.endTime)
                      : "N/A"}
                  </td>
                  <td className="px-4 py-3">
                    <div className="flex items-center justify-end gap-1">
                      {onSelectModel && (
                        <button
                          onClick={() => onSelectModel(session)}
                          className="p-2 text-gray-400 hover:text-emerald-400 hover:bg-emerald-500/10 rounded-lg transition-colors"
                          title="Use this model"
                        >
                          <CheckCircle2 className="w-4 h-4" />
                        </button>
                      )}
                      {onExportSession && (
                        <button
                          onClick={() => onExportSession(session)}
                          className="p-2 text-gray-400 hover:text-blue-400 hover:bg-blue-500/10 rounded-lg transition-colors"
                          title="Export"
                        >
                          <Download className="w-4 h-4" />
                        </button>
                      )}
                      {onDeleteSession && (
                        <button
                          onClick={() => onDeleteSession(session.id)}
                          className="p-2 text-gray-400 hover:text-rose-400 hover:bg-rose-500/10 rounded-lg transition-colors"
                          title="Delete"
                        >
                          <Trash2 className="w-4 h-4" />
                        </button>
                      )}
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}

// Helper function
function formatDuration(start: Date, end: Date): string {
  const diff = Math.floor((end.getTime() - start.getTime()) / 1000);
  const hours = Math.floor(diff / 3600);
  const mins = Math.floor((diff % 3600) / 60);
  const secs = diff % 60;

  if (hours > 0) return `${hours}h ${mins}m`;
  if (mins > 0) return `${mins}m ${secs}s`;
  return `${secs}s`;
}
