"use client";

import { useState, useEffect } from "react";
import { motion } from "framer-motion";
import {
  Trophy,
  TrendingUp,
  TrendingDown,
  Target,
  BarChart3,
  Loader2,
  AlertTriangle,
  Users,
  Hash,
  Percent,
} from "lucide-react";

// -- Types ------------------------------------------------------------------

interface LeaderboardEntry {
  rank: number;
  strategy_name: string;
  creator_anonymous: string;
  total_return: number;
  sharpe: number;
  drawdown: number;
  win_rate: number;
  trades: number;
}

type Timeframe = "7d" | "30d" | "all";

// -- Page Component ---------------------------------------------------------

export default function LeaderboardPage() {
  const [timeframe, setTimeframe] = useState<Timeframe>("30d");
  const [entries, setEntries] = useState<LeaderboardEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchLeaderboard(timeframe);
  }, [timeframe]);

  const fetchLeaderboard = async (tf: Timeframe) => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(`/api/leaderboard?timeframe=${tf}`);
      const data = await res.json();
      if (data.success) {
        setEntries(data.data);
      } else {
        setError(data.error?.message || "Failed to load leaderboard");
      }
    } catch {
      setError("Failed to connect to leaderboard API");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen text-slate-100">
      <div className="p-6 space-y-6">
        {/* Header */}
        <motion.div
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
        >
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-xl bg-amber-500/10 border border-amber-500/20">
                <Trophy className="w-7 h-7 text-amber-400" />
              </div>
              <div>
                <h1 className="text-2xl font-bold text-white">
                  Performance Leaderboard
                </h1>
                <p className="text-gray-400 text-sm">
                  Top strategies ranked by risk-adjusted returns
                </p>
              </div>
            </div>
          </div>
        </motion.div>

        {/* Timeframe Buttons */}
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.1 }}
          className="flex gap-2"
        >
          {(["7d", "30d", "all"] as Timeframe[]).map((tf) => (
            <button
              key={tf}
              onClick={() => setTimeframe(tf)}
              className={`px-4 py-2 rounded-xl font-medium text-sm transition-colors duration-200 cursor-pointer ${
                timeframe === tf
                  ? "bg-amber-600 text-white"
                  : "bg-gray-800/50 text-slate-400 hover:bg-gray-700/50 border border-gray-700/50"
              }`}
            >
              {tf === "7d" ? "7 Days" : tf === "30d" ? "30 Days" : "All Time"}
            </button>
          ))}
        </motion.div>

        {/* Error */}
        {error && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            className="bg-red-500/10 border border-red-500/30 rounded-xl p-4 flex items-center gap-3"
          >
            <AlertTriangle className="w-5 h-5 text-red-400 shrink-0" />
            <span className="text-red-300 text-sm">{error}</span>
          </motion.div>
        )}

        {/* Loading */}
        {loading && (
          <div className="flex justify-center py-16">
            <Loader2 className="w-8 h-8 text-amber-400 animate-spin" />
          </div>
        )}

        {/* Empty State */}
        {!loading && !error && entries.length === 0 && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            className="bg-gray-800/50 border border-gray-700/50 rounded-2xl p-12 text-center"
          >
            <Users className="w-12 h-12 text-slate-600 mx-auto mb-4" />
            <h3 className="text-lg font-semibold text-slate-400 mb-2">
              No Strategies Yet
            </h3>
            <p className="text-sm text-slate-500">
              Start trading with a strategy to appear on the leaderboard.
            </p>
          </motion.div>
        )}

        {/* Leaderboard Table */}
        {!loading && entries.length > 0 && (
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.15 }}
            className="bg-gray-800/50 border border-gray-700/50 rounded-2xl overflow-hidden"
          >
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead>
                  <tr className="border-b border-gray-700/50">
                    <th className="text-left px-6 py-4 text-xs font-semibold text-slate-400 uppercase tracking-wider">
                      Rank
                    </th>
                    <th className="text-left px-6 py-4 text-xs font-semibold text-slate-400 uppercase tracking-wider">
                      Strategy
                    </th>
                    <th className="text-left px-6 py-4 text-xs font-semibold text-slate-400 uppercase tracking-wider">
                      Creator
                    </th>
                    <th className="text-right px-6 py-4 text-xs font-semibold text-slate-400 uppercase tracking-wider">
                      Return
                    </th>
                    <th className="text-right px-6 py-4 text-xs font-semibold text-slate-400 uppercase tracking-wider">
                      Sharpe
                    </th>
                    <th className="text-right px-6 py-4 text-xs font-semibold text-slate-400 uppercase tracking-wider">
                      Drawdown
                    </th>
                    <th className="text-right px-6 py-4 text-xs font-semibold text-slate-400 uppercase tracking-wider">
                      Win Rate
                    </th>
                    <th className="text-right px-6 py-4 text-xs font-semibold text-slate-400 uppercase tracking-wider">
                      Trades
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {entries.map((entry, idx) => (
                    <motion.tr
                      key={`${entry.rank}-${entry.strategy_name}`}
                      initial={{ opacity: 0, x: -10 }}
                      animate={{ opacity: 1, x: 0 }}
                      transition={{ delay: 0.05 * idx }}
                      className="border-b border-gray-700/20 hover:bg-gray-700/20 transition-colors"
                    >
                      <td className="px-6 py-4">
                        <RankBadge rank={entry.rank} />
                      </td>
                      <td className="px-6 py-4">
                        <div className="flex items-center gap-2">
                          <BarChart3 className="w-4 h-4 text-cyan-400" />
                          <span className="font-medium text-white">
                            {entry.strategy_name}
                          </span>
                        </div>
                      </td>
                      <td className="px-6 py-4 text-slate-400 text-sm">
                        {entry.creator_anonymous}
                      </td>
                      <td className="px-6 py-4 text-right">
                        <span
                          className={`font-semibold ${
                            entry.total_return >= 0
                              ? "text-emerald-400"
                              : "text-red-400"
                          }`}
                        >
                          {entry.total_return >= 0 ? "+" : ""}
                          {entry.total_return.toFixed(2)}%
                        </span>
                      </td>
                      <td className="px-6 py-4 text-right">
                        <span
                          className={`font-medium ${
                            entry.sharpe >= 1
                              ? "text-emerald-400"
                              : entry.sharpe >= 0
                                ? "text-amber-400"
                                : "text-red-400"
                          }`}
                        >
                          {entry.sharpe.toFixed(2)}
                        </span>
                      </td>
                      <td className="px-6 py-4 text-right">
                        <span
                          className={`font-medium ${
                            entry.drawdown <= 10
                              ? "text-emerald-400"
                              : entry.drawdown <= 25
                                ? "text-amber-400"
                                : "text-red-400"
                          }`}
                        >
                          -{entry.drawdown.toFixed(2)}%
                        </span>
                      </td>
                      <td className="px-6 py-4 text-right">
                        <span
                          className={`font-medium ${
                            entry.win_rate >= 50
                              ? "text-emerald-400"
                              : "text-amber-400"
                          }`}
                        >
                          {entry.win_rate.toFixed(1)}%
                        </span>
                      </td>
                      <td className="px-6 py-4 text-right text-slate-300 font-medium">
                        {entry.trades}
                      </td>
                    </motion.tr>
                  ))}
                </tbody>
              </table>
            </div>
          </motion.div>
        )}
      </div>
    </div>
  );
}

// -- Sub-components ---------------------------------------------------------

function RankBadge({ rank }: { rank: number }) {
  if (rank === 1) {
    return (
      <div className="flex items-center gap-1.5">
        <div className="w-7 h-7 rounded-full bg-amber-500/20 border border-amber-500/40 flex items-center justify-center">
          <Trophy className="w-4 h-4 text-amber-400" />
        </div>
        <span className="font-bold text-amber-400">#1</span>
      </div>
    );
  }
  if (rank === 2) {
    return (
      <div className="flex items-center gap-1.5">
        <div className="w-7 h-7 rounded-full bg-slate-400/20 border border-slate-400/40 flex items-center justify-center">
          <Trophy className="w-4 h-4 text-slate-300" />
        </div>
        <span className="font-bold text-slate-300">#2</span>
      </div>
    );
  }
  if (rank === 3) {
    return (
      <div className="flex items-center gap-1.5">
        <div className="w-7 h-7 rounded-full bg-orange-600/20 border border-orange-600/40 flex items-center justify-center">
          <Trophy className="w-4 h-4 text-orange-400" />
        </div>
        <span className="font-bold text-orange-400">#3</span>
      </div>
    );
  }
  return (
    <div className="flex items-center gap-1.5">
      <div className="w-7 h-7 rounded-full bg-gray-700/50 border border-gray-600/40 flex items-center justify-center">
        <Hash className="w-3.5 h-3.5 text-slate-500" />
      </div>
      <span className="font-medium text-slate-400">#{rank}</span>
    </div>
  );
}
