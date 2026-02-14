"use client";

import { useMemo } from "react";
import { motion } from "framer-motion";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
  AreaChart,
  Area,
} from "recharts";
import {
  TrendingUp,
  TrendingDown,
  Activity,
  Target,
  BarChart3,
  PieChart,
  Clock,
} from "lucide-react";
import type { TrainingMetrics } from "./ai-training-mode";

interface MetricsDashboardProps {
  metrics: TrainingMetrics[];
}

export function MetricsDashboard({ metrics }: MetricsDashboardProps) {
  const stats = useMemo(() => {
    if (metrics.length === 0) return null;

    const confidences = metrics.map((m) => m.confidence);
    const trainLosses = metrics.map((m) => m.trainLoss);
    const valLosses = metrics.map((m) => m.valLoss);
    const valAccuracies = metrics.map((m) => m.valAccuracy);

    return {
      bestConfidence: Math.max(...confidences),
      worstConfidence: Math.min(...confidences),
      avgConfidence: confidences.reduce((a, b) => a + b, 0) / confidences.length,
      finalTrainLoss: trainLosses[trainLosses.length - 1],
      finalValLoss: valLosses[valLosses.length - 1],
      bestValAccuracy: Math.max(...valAccuracies),
      overfitting: trainLosses[trainLosses.length - 1] < valLosses[valLosses.length - 1] - 0.1,
      totalEpochs: metrics.length,
      avgTimePerEpoch: metrics.reduce((a, m) => a + m.timePerEpoch, 0) / metrics.length,
    };
  }, [metrics]);

  if (metrics.length === 0) {
    return (
      <div className="glass-card rounded-xl p-8 text-center">
        <Activity className="w-12 h-12 text-gray-600 mx-auto mb-4" />
        <p className="text-gray-400">No training data yet. Start training to see metrics.</p>
      </div>
    );
  }

  const chartData = metrics.map((m) => ({
    epoch: m.epoch,
    confidence: parseFloat(m.confidence.toFixed(2)),
    trainLoss: parseFloat(m.trainLoss.toFixed(4)),
    valLoss: parseFloat(m.valLoss.toFixed(4)),
    trainAccuracy: parseFloat(m.trainAccuracy.toFixed(2)),
    valAccuracy: parseFloat(m.valAccuracy.toFixed(2)),
  }));

  return (
    <div className="space-y-6">
      {/* Stats Overview */}
      {stats && (
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <StatCard
            label="Best Confidence"
            value={`${stats.bestConfidence.toFixed(2)}%`}
            icon={Target}
            trend="up"
            color="emerald"
          />
          <StatCard
            label="Best Val Accuracy"
            value={`${stats.bestValAccuracy.toFixed(2)}%`}
            icon={TrendingUp}
            trend="up"
            color="blue"
          />
          <StatCard
            label="Final Train Loss"
            value={stats.finalTrainLoss.toFixed(4)}
            icon={TrendingDown}
            trend="down"
            color="amber"
          />
          <StatCard
            label="Avg Time/Epoch"
            value={`${stats.avgTimePerEpoch.toFixed(1)}s`}
            icon={Clock}
            trend="neutral"
            color="purple"
          />
        </div>
      )}

      {/* Charts Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Confidence Chart */}
        <ChartCard title="Confidence Progress" icon={Target}>
          <ResponsiveContainer width="100%" height={250}>
            <AreaChart data={chartData}>
              <defs>
                <linearGradient id="confidenceGradient" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#10b981" stopOpacity={0.3} />
                  <stop offset="95%" stopColor="#10b981" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
              <XAxis dataKey="epoch" stroke="#9ca3af" fontSize={12} />
              <YAxis domain={[50, 100]} stroke="#9ca3af" fontSize={12} />
              <Tooltip
                contentStyle={{
                  backgroundColor: "#1f2937",
                  border: "1px solid #374151",
                  borderRadius: "8px",
                }}
              />
              <Area
                type="monotone"
                dataKey="confidence"
                stroke="#10b981"
                fillOpacity={1}
                fill="url(#confidenceGradient)"
                strokeWidth={2}
              />
            </AreaChart>
          </ResponsiveContainer>
        </ChartCard>

        {/* Loss Chart */}
        <ChartCard title="Loss Curves" icon={Activity}>
          <ResponsiveContainer width="100%" height={250}>
            <LineChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
              <XAxis dataKey="epoch" stroke="#9ca3af" fontSize={12} />
              <YAxis stroke="#9ca3af" fontSize={12} />
              <Tooltip
                contentStyle={{
                  backgroundColor: "#1f2937",
                  border: "1px solid #374151",
                  borderRadius: "8px",
                }}
              />
              <Legend />
              <Line
                type="monotone"
                dataKey="trainLoss"
                name="Training Loss"
                stroke="#3b82f6"
                strokeWidth={2}
                dot={false}
              />
              <Line
                type="monotone"
                dataKey="valLoss"
                name="Validation Loss"
                stroke="#f59e0b"
                strokeWidth={2}
                dot={false}
              />
            </LineChart>
          </ResponsiveContainer>
        </ChartCard>

        {/* Accuracy Chart */}
        <ChartCard title="Accuracy Comparison" icon={BarChart3}>
          <ResponsiveContainer width="100%" height={250}>
            <LineChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
              <XAxis dataKey="epoch" stroke="#9ca3af" fontSize={12} />
              <YAxis domain={[0, 100]} stroke="#9ca3af" fontSize={12} />
              <Tooltip
                contentStyle={{
                  backgroundColor: "#1f2937",
                  border: "1px solid #374151",
                  borderRadius: "8px",
                }}
              />
              <Legend />
              <Line
                type="monotone"
                dataKey="trainAccuracy"
                name="Train Accuracy"
                stroke="#8b5cf6"
                strokeWidth={2}
                dot={false}
              />
              <Line
                type="monotone"
                dataKey="valAccuracy"
                name="Val Accuracy"
                stroke="#10b981"
                strokeWidth={2}
                dot={false}
              />
            </LineChart>
          </ResponsiveContainer>
        </ChartCard>

        {/* Learning Rate */}
        <ChartCard title="Learning Rate Schedule" icon={PieChart}>
          <ResponsiveContainer width="100%" height={250}>
            <AreaChart data={chartData.map((d, i) => ({ ...d, lr: metrics[i]?.learningRate || 0 }))}>
              <defs>
                <linearGradient id="lrGradient" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#ec4899" stopOpacity={0.3} />
                  <stop offset="95%" stopColor="#ec4899" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
              <XAxis dataKey="epoch" stroke="#9ca3af" fontSize={12} />
              <YAxis stroke="#9ca3af" fontSize={12} />
              <Tooltip
                contentStyle={{
                  backgroundColor: "#1f2937",
                  border: "1px solid #374151",
                  borderRadius: "8px",
                }}
              />
              <Area
                type="monotone"
                dataKey="lr"
                name="Learning Rate"
                stroke="#ec4899"
                fillOpacity={1}
                fill="url(#lrGradient)"
                strokeWidth={2}
              />
            </AreaChart>
          </ResponsiveContainer>
        </ChartCard>
      </div>

      {/* Overfitting Warning */}
      {stats?.overfitting && (
        <motion.div
          initial={{ opacity: 0, height: 0 }}
          animate={{ opacity: 1, height: "auto"}}
          className="p-4 rounded-xl bg-amber-500/10 border border-amber-500/30"
        >
          <div className="flex items-start gap-3">
            <TrendingDown className="w-5 h-5 text-amber-400 flex-shrink-0 mt-0.5" />
            <div>
              <h4 className="font-medium text-amber-400">Potential Overfitting Detected</h4>
              <p className="text-sm text-amber-200/80 mt-1">
                Training loss is significantly lower than validation loss. Consider:
              </p>
              <ul className="text-sm text-amber-200/60 mt-2 list-disc list-inside">
                <li>Reducing model complexity</li>
                <li>Adding regularization (dropout, L2)</li>
                <li>Increasing training data</li>
                <li>Enabling early stopping with lower patience</li>
              </ul>
            </div>
          </div>
        </motion.div>
      )}
    </div>
  );
}

// Stat Card Component
function StatCard({
  label,
  value,
  icon: Icon,
  trend,
  color,
}: {
  label: string;
  value: string;
  icon: React.ElementType;
  trend: "up" | "down" | "neutral";
  color: "emerald" | "blue" | "amber" | "purple";
}) {
  const colorClasses = {
    emerald: "bg-emerald-500/10 text-emerald-400 border-emerald-500/30",
    blue: "bg-blue-500/10 text-blue-400 border-blue-500/30",
    amber: "bg-amber-500/10 text-amber-400 border-amber-500/30",
    purple: "bg-purple-500/10 text-purple-400 border-purple-500/30",
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      className={`p-4 rounded-xl border ${colorClasses[color]}`}
    >
      <div className="flex items-center gap-2 mb-2">
        <Icon className="w-4 h-4" />
        <span className="text-xs opacity-70">{label}</span>
      </div>
      <p className="text-2xl font-bold">{value}</p>
    </motion.div>
  );
}

// Chart Card Component
function ChartCard({
  title,
  icon: Icon,
  children,
}: {
  title: string;
  icon: React.ElementType;
  children: React.ReactNode;
}) {
  return (
    <div className="glass-card rounded-xl p-4">
      <div className="flex items-center gap-2 mb-4">
        <Icon className="w-4 h-4 text-gray-400" />
        <h4 className="font-medium text-white">{title}</h4>
      </div>
      {children}
    </div>
  );
}
