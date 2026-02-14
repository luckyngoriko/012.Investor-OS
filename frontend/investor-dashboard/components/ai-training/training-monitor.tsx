"use client";

import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import {
  Activity,
  Clock,
  TrendingUp,
  Zap,
  AlertCircle,
  CheckCircle2,
  Pause,
} from "lucide-react";
import type { TrainingMetrics, TrainingMode } from "./ai-training-mode";

interface TrainingMonitorProps {
  status: TrainingMode;
  metrics: TrainingMetrics[];
  currentEpoch: number;
  maxEpochs: number;
  startTime?: Date | null;
}

export function TrainingMonitor({
  status,
  metrics,
  currentEpoch,
  maxEpochs,
  startTime,
}: TrainingMonitorProps) {
  const [elapsedTime, setElapsedTime] = useState(0);
  const isRunning = status === "running";

  // Update elapsed time
  useEffect(() => {
    if (!startTime || !isRunning) return;

    const interval = setInterval(() => {
      setElapsedTime(Math.floor((Date.now() - startTime.getTime()) / 1000));
    }, 1000);

    return () => clearInterval(interval);
  }, [startTime, isRunning]);

  const formatTime = (seconds: number) => {
    const hrs = Math.floor(seconds / 3600);
    const mins = Math.floor((seconds % 3600) / 60);
    const secs = seconds % 60;
    return `${hrs.toString().padStart(2, "0")}:${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
  };

  const currentMetric = metrics[metrics.length - 1];
  const previousMetric = metrics[metrics.length - 2];

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center gap-3">
        <div className="w-10 h-10 rounded-xl bg-emerald-500/10 flex items-center justify-center">
          <Activity className="w-5 h-5 text-emerald-400" />
        </div>
        <div>
          <h3 className="font-semibold text-white">Live Monitor</h3>
          <p className="text-sm text-gray-500">Real-time training progress</p>
        </div>
      </div>

      {/* Status Bar */}
      <div className="glass-card rounded-xl p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <StatusIndicator status={status} />
            
            <div className="flex items-center gap-2 text-sm text-gray-400">
              <Clock className="w-4 h-4" />
              <span>{formatTime(elapsedTime)}</span>
            </div>
          </div>

          <div className="text-right">
            <span className="text-sm text-gray-500">Epoch</span>
            <p className="text-xl font-bold text-white">
              {currentEpoch} <span className="text-gray-500">/ {maxEpochs}</span>
            </p>
          </div>
        </div>

        {/* Progress Bar */}
        <div className="mt-4">
          <div className="h-2 bg-gray-700 rounded-full overflow-hidden">
            <motion.div
              className="h-full bg-gradient-to-r from-blue-500 to-emerald-500"
              initial={{ width: 0 }}
              animate={{ width: `${(currentEpoch / maxEpochs) * 100}%` }}
              transition={{ duration: 0.5 }}
            />
          </div>
        </div>
      </div>

      {/* Current Metrics */}
      {currentMetric && (
        <div className="grid grid-cols-2 gap-3">
          <MetricCard
            label="Confidence"
            value={`${currentMetric.confidence.toFixed(2)}%`}
            change={previousMetric ? currentMetric.confidence - previousMetric.confidence : 0}
            icon={Zap}
            color="amber"
          />
          <MetricCard
            label="Validation Accuracy"
            value={`${currentMetric.valAccuracy.toFixed(2)}%`}
            change={previousMetric ? currentMetric.valAccuracy - previousMetric.valAccuracy : 0}
            icon={CheckCircle2}
            color="emerald"
          />
          <MetricCard
            label="Training Loss"
            value={currentMetric.trainLoss.toFixed(4)}
            change={previousMetric ? currentMetric.trainLoss - previousMetric.trainLoss : 0}
            icon={TrendingUp}
            color="blue"
            inverse
          />
          <MetricCard
            label="Validation Loss"
            value={currentMetric.valLoss.toFixed(4)}
            change={previousMetric ? currentMetric.valLoss - previousMetric.valLoss : 0}
            icon={Activity}
            color="purple"
            inverse
          />
        </div>
      )}

      {/* Mini Chart */}
      {metrics.length > 1 && (
        <div className="glass-card rounded-xl p-4">
          <h4 className="text-sm font-medium text-gray-300 mb-3">Confidence History</h4>
          <MiniChart data={metrics} />
        </div>
      )}
    </div>
  );
}

// Status Indicator
function StatusIndicator({ status }: { status: TrainingMode }) {
  const config = {
    idle: { color: "bg-gray-500", text: "Idle", icon: null },
    running: { color: "bg-emerald-500", text: "Running", icon: Activity },
    paused: { color: "bg-amber-500", text: "Paused", icon: Pause },
    completed: { color: "bg-blue-500", text: "Completed", icon: CheckCircle2 },
    error: { color: "bg-rose-500", text: "Error", icon: AlertCircle },
  };

  const { color, text, icon: Icon } = config[status];

  return (
    <div className="flex items-center gap-2">
      <motion.div
        animate={status === "running" ? { scale: [1, 1.2, 1] } : {}}
        transition={{ duration: 1, repeat: status === "running" ? Infinity : 0 }}
        className={`w-3 h-3 rounded-full ${color}`}
      />
      <span className="text-sm font-medium text-white">{text}</span>
      {Icon && <Icon className="w-4 h-4 text-gray-400" />}
    </div>
  );
}

// Metric Card
function MetricCard({
  label,
  value,
  change,
  icon: Icon,
  color,
  inverse = false,
}: {
  label: string;
  value: string;
  change: number;
  icon: React.ElementType;
  color: "blue" | "emerald" | "amber" | "purple" | "rose";
  inverse?: boolean;
}) {
  const colorClasses = {
    blue: "bg-blue-500/10 text-blue-400 border-blue-500/30",
    emerald: "bg-emerald-500/10 text-emerald-400 border-emerald-500/30",
    amber: "bg-amber-500/10 text-amber-400 border-amber-500/30",
    purple: "bg-purple-500/10 text-purple-400 border-purple-500/30",
    rose: "bg-rose-500/10 text-rose-400 border-rose-500/30",
  };

  const isPositive = inverse ? change < 0 : change > 0;
  const changeColor = isPositive ? "text-emerald-400" : change < 0 ? "text-rose-400" : "text-gray-400";

  return (
    <div className={`p-3 rounded-xl border ${colorClasses[color]}`}>
      <div className="flex items-center gap-2 mb-1">
        <Icon className="w-4 h-4 opacity-70" />
        <span className="text-xs opacity-70">{label}</span>
      </div>
      <div className="flex items-baseline gap-2">
        <span className="text-xl font-bold">{value}</span>
        {change !== 0 && (
          <span className={`text-xs ${changeColor}`}>
            {change > 0 ? "+" : ""}{change.toFixed(2)}
          </span>
        )}
      </div>
    </div>
  );
}

// Mini Chart
function MiniChart({ data }: { data: TrainingMetrics[] }) {
  const width = 300;
  const height = 80;
  const padding = 5;

  const minConf = Math.min(...data.map((d) => d.confidence), 50);
  const maxConf = Math.max(...data.map((d) => d.confidence), 100);
  const range = maxConf - minConf || 1;

  const points = data.map((d, i) => {
    const x = padding + (i / (data.length - 1 || 1)) * (width - padding * 2);
    const y = height - padding - ((d.confidence - minConf) / range) * (height - padding * 2);
    return `${x},${y}`;
  }).join(" ");

  return (
    <svg viewBox={`0 0 ${width} ${height}`} className="w-full h-20">
      {/* Grid lines */}
      <line x1="0" y1={height / 2} x2={width} y2={height / 2} stroke="rgba(255,255,255,0.1)" strokeDasharray="4" />
      
      {/* Line */}
      <polyline
        points={points}
        fill="none"
        stroke="#10b981"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      
      {/* Area under line */}
      <polygon
        points={`${padding},${height - padding} ${points} ${width - padding},${height - padding}`}
        fill="url(#gradient)"
        opacity="0.3"
      />
      
      {/* Gradient definition */}
      <defs>
        <linearGradient id="gradient" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#10b981" />
          <stop offset="100%" stopColor="transparent" />
        </linearGradient>
      </defs>
      
      {/* Current point */}
      {data.length > 0 && (
        <circle
          cx={width - padding}
          cy={height - padding - ((data[data.length - 1].confidence - minConf) / range) * (height - padding * 2)}
          r="4"
          fill="#10b981"
        />
      )}
    </svg>
  );
}
