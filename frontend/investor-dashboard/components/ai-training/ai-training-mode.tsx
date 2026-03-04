"use client";

import { useState, useEffect, useCallback } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { addNotification } from "@/components/notification-center";
import {
  Brain,
  Play,
  Pause,
  Square,
  RotateCcw,
  Target,
  TrendingUp,
  AlertCircle,
  CheckCircle2,
  Clock,
  Zap,
  Shield,
  Save,
  Download,
} from "lucide-react";

// ============================================
// TYPES & CONFIGURATION
// ============================================

export type TrainingMode =
  | "idle"
  | "running"
  | "paused"
  | "completed"
  | "error";

export interface TrainingConfig {
  targetConfidence: number; // 0-100%
  maxEpochs: number; // Максимум епохи
  earlyStoppingPatience: number; // Epochs без подобрение преди спиране
  minDelta: number; // Минимално подобрение за отчитане
  learningRate: number; // Learning rate
  batchSize: number; // Batch size
  validationSplit: number; // % за validation
  datasetSize: number; // Общ брой примери
  modelType: "xgboost" | "lstm" | "transformer" | "ensemble";
  checkpointInterval: number; // Запазване на checkpoint на всеки N епохи
  autoSave: boolean; // Автоматично запазване на най-добрия модел
}

export interface TrainingMetrics {
  epoch: number;
  trainLoss: number;
  valLoss: number;
  trainAccuracy: number;
  valAccuracy: number;
  confidence: number; // Текущ confidence score
  learningRate: number;
  timePerEpoch: number; // Секунди
  estimatedTimeRemaining: number; // Секунди
}

export interface TrainingSession {
  id: string;
  name: string;
  config: TrainingConfig;
  metrics: TrainingMetrics[];
  bestEpoch: number;
  bestConfidence: number;
  status: TrainingMode;
  startTime?: Date;
  endTime?: Date;
  error?: string;
}

export const DEFAULT_TRAINING_CONFIG: TrainingConfig = {
  targetConfidence: 85, // Цел: 85% confidence
  maxEpochs: 1000,
  earlyStoppingPatience: 50,
  minDelta: 0.001,
  learningRate: 0.001,
  batchSize: 256,
  validationSplit: 0.2,
  datasetSize: 100000,
  modelType: "ensemble",
  checkpointInterval: 10,
  autoSave: true,
};

// ============================================
// TRAINING STATUS INDICATOR
// ============================================

interface TrainingStatusProps {
  status: TrainingMode;
  currentConfidence: number;
  targetConfidence: number;
}

export function TrainingStatus({
  status,
  currentConfidence,
  targetConfidence,
}: TrainingStatusProps) {
  const progress = Math.min((currentConfidence / targetConfidence) * 100, 100);

  const statusConfig = {
    idle: {
      color: "text-gray-400",
      bgColor: "bg-gray-500/10",
      borderColor: "border-gray-500/30",
      icon: Brain,
      label: "Ready to Train",
      message: "Configure and start training",
    },
    running: {
      color: "text-blue-400",
      bgColor: "bg-blue-500/10",
      borderColor: "border-blue-500/30",
      icon: Zap,
      label: "Training in Progress",
      message: `Confidence: ${currentConfidence.toFixed(2)}% / ${targetConfidence}%`,
    },
    paused: {
      color: "text-amber-400",
      bgColor: "bg-amber-500/10",
      borderColor: "border-amber-500/30",
      icon: Pause,
      label: "Training Paused",
      message: "Resume when ready",
    },
    completed: {
      color: "text-emerald-400",
      bgColor: "bg-emerald-500/10",
      borderColor: "border-emerald-500/30",
      icon: CheckCircle2,
      label: "Training Completed",
      message: `Target confidence ${targetConfidence}% achieved!`,
    },
    error: {
      color: "text-rose-400",
      bgColor: "bg-rose-500/10",
      borderColor: "border-rose-500/30",
      icon: AlertCircle,
      label: "Training Error",
      message: "Check logs for details",
    },
  };

  const config = statusConfig[status];
  const Icon = config.icon;

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      className={`relative p-6 rounded-2xl ${config.bgColor} ${config.borderColor} border`}
    >
      {/* Progress Bar Background */}
      <div className="absolute inset-x-0 bottom-0 h-1 bg-gray-800">
        <motion.div
          className={`h-full ${status === "completed" ? "bg-emerald-500" : "bg-blue-500"}`}
          initial={{ width: 0 }}
          animate={{ width: `${progress}%` }}
          transition={{ duration: 0.5 }}
        />
      </div>

      <div className="flex items-start gap-4">
        <motion.div
          animate={status === "running" ? { rotate: 360 } : {}}
          transition={
            status === "running"
              ? { duration: 2, repeat: Infinity, ease: "linear" }
              : {}
          }
          className={`w-14 h-14 rounded-xl ${config.bgColor} flex items-center justify-center`}
        >
          <Icon className={`w-7 h-7 ${config.color}`} />
        </motion.div>

        <div className="flex-1">
          <h3 className={`text-xl font-bold ${config.color}`}>
            {config.label}
          </h3>
          <p className="text-gray-400 mt-1">{config.message}</p>

          {/* Progress Details */}
          <div className="flex items-center gap-6 mt-4">
            <div>
              <span className="text-xs text-gray-500 uppercase">Progress</span>
              <p className={`text-2xl font-bold ${config.color}`}>
                {progress.toFixed(1)}%
              </p>
            </div>
            <div>
              <span className="text-xs text-gray-500 uppercase">Current</span>
              <p className="text-2xl font-bold text-white">
                {currentConfidence.toFixed(2)}%
              </p>
            </div>
            <div>
              <span className="text-xs text-gray-500 uppercase">Target</span>
              <p className="text-2xl font-bold text-white">
                {targetConfidence}%
              </p>
            </div>
          </div>
        </div>
      </div>
    </motion.div>
  );
}

// ============================================
// CONTROL PANEL
// ============================================

interface ControlPanelProps {
  status: TrainingMode;
  onStart: () => void;
  onPause: () => void;
  onStop: () => void;
  onReset: () => void;
  onSave: () => void;
  onExport: () => void;
  canSave: boolean;
}

export function ControlPanel({
  status,
  onStart,
  onPause,
  onStop,
  onReset,
  onSave,
  onExport,
  canSave,
}: ControlPanelProps) {
  const isRunning = status === "running";
  const isPaused = status === "paused";
  const isIdle = status === "idle";
  const isCompleted = status === "completed";

  return (
    <div className="flex flex-wrap items-center gap-3">
      {/* Main Controls */}
      {(isIdle || isPaused || isCompleted) && (
        <motion.button
          whileHover={{ scale: 1.02 }}
          whileTap={{ scale: 0.98 }}
          onClick={onStart}
          className="flex items-center gap-2 px-6 py-3 bg-blue-600 hover:bg-blue-500 text-white font-semibold rounded-xl transition-colors"
        >
          <Play className="w-5 h-5" />
          {isPaused ? "Resume" : "Start Training"}
        </motion.button>
      )}

      {isRunning && (
        <motion.button
          whileHover={{ scale: 1.02 }}
          whileTap={{ scale: 0.98 }}
          onClick={onPause}
          className="flex items-center gap-2 px-6 py-3 bg-amber-600 hover:bg-amber-500 text-white font-semibold rounded-xl transition-colors"
        >
          <Pause className="w-5 h-5" />
          Pause
        </motion.button>
      )}

      {(isRunning || isPaused) && (
        <motion.button
          whileHover={{ scale: 1.02 }}
          whileTap={{ scale: 0.98 }}
          onClick={onStop}
          className="flex items-center gap-2 px-6 py-3 bg-rose-600 hover:bg-rose-500 text-white font-semibold rounded-xl transition-colors"
        >
          <Square className="w-5 h-5" />
          Stop
        </motion.button>
      )}

      {/* Secondary Controls */}
      <div className="flex items-center gap-2 ml-auto">
        <motion.button
          whileHover={{ scale: 1.02 }}
          whileTap={{ scale: 0.98 }}
          onClick={onReset}
          disabled={isRunning}
          className="flex items-center gap-2 px-4 py-3 bg-gray-700 hover:bg-gray-600 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-xl transition-colors"
        >
          <RotateCcw className="w-4 h-4" />
          Reset
        </motion.button>

        <motion.button
          whileHover={{ scale: 1.02 }}
          whileTap={{ scale: 0.98 }}
          onClick={onSave}
          disabled={!canSave || isRunning}
          className="flex items-center gap-2 px-4 py-3 bg-emerald-600/20 hover:bg-emerald-600/30 disabled:opacity-50 disabled:cursor-not-allowed text-emerald-400 border border-emerald-500/30 rounded-xl transition-colors"
        >
          <Save className="w-4 h-4" />
          Save Model
        </motion.button>

        <motion.button
          whileHover={{ scale: 1.02 }}
          whileTap={{ scale: 0.98 }}
          onClick={onExport}
          disabled={!canSave}
          className="flex items-center gap-2 px-4 py-3 bg-gray-700 hover:bg-gray-600 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-xl transition-colors"
        >
          <Download className="w-4 h-4" />
          Export
        </motion.button>
      </div>
    </div>
  );
}

// ============================================
// MAIN AI TRAINING MODE COMPONENT
// ============================================

interface AITrainingModeProps {
  initialConfig?: TrainingConfig;
  onSessionUpdate?: (session: TrainingSession) => void;
}

export function AITrainingMode({
  initialConfig,
  onSessionUpdate,
}: AITrainingModeProps) {
  const [config, setConfig] = useState<TrainingConfig>(
    initialConfig || DEFAULT_TRAINING_CONFIG,
  );
  const [status, setStatus] = useState<TrainingMode>("idle");
  const [metrics, setMetrics] = useState<TrainingMetrics[]>([]);
  const [currentEpoch, setCurrentEpoch] = useState(0);
  const [currentConfidence, setCurrentConfidence] = useState(0);
  const [bestConfidence, setBestConfidence] = useState(0);
  const [patienceCounter, setPatienceCounter] = useState(0);
  const [startTime, setStartTime] = useState<Date | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Simulate training step
  const trainingStep = useCallback(() => {
    setCurrentEpoch((prev) => {
      const newEpoch = prev + 1;

      // Simulate metrics improvement with some randomness
      setMetrics((prevMetrics) => {
        const lastMetric = prevMetrics[prevMetrics.length - 1];
        const baseConfidence = lastMetric?.confidence || 50;

        // Gradual improvement with diminishing returns
        const improvement = Math.max(
          0,
          (config.targetConfidence - baseConfidence) * 0.05 * Math.random(),
        );
        const noise = (Math.random() - 0.5) * 2;
        const newConfidence = Math.min(
          baseConfidence + improvement + noise,
          99.9,
        );

        const newMetric: TrainingMetrics = {
          epoch: newEpoch,
          trainLoss: Math.max(0.1, (lastMetric?.trainLoss || 1.0) * 0.98),
          valLoss: Math.max(0.15, (lastMetric?.valLoss || 1.2) * 0.98),
          trainAccuracy: Math.min(
            99,
            (lastMetric?.trainAccuracy || 60) + improvement * 0.8,
          ),
          valAccuracy: Math.min(
            98,
            (lastMetric?.valAccuracy || 55) + improvement * 0.7,
          ),
          confidence: newConfidence,
          learningRate: config.learningRate * Math.pow(0.95, newEpoch / 100),
          timePerEpoch: 5 + Math.random() * 3,
          estimatedTimeRemaining: (config.maxEpochs - newEpoch) * 6,
        };

        // Check for improvement
        if (newConfidence > bestConfidence + config.minDelta) {
          setBestConfidence(newConfidence);
          setPatienceCounter(0);
        } else {
          setPatienceCounter((p) => p + 1);
        }

        setCurrentConfidence(newConfidence);

        // Check completion criteria
        if (newConfidence >= config.targetConfidence) {
          setStatus("completed");
          if (onSessionUpdate) {
            onSessionUpdate({
              id: Date.now().toString(),
              name: `Training Session ${new Date().toLocaleString()}`,
              config,
              metrics: [...prevMetrics, newMetric],
              bestEpoch: newEpoch,
              bestConfidence: newConfidence,
              status: "completed",
              startTime: startTime || undefined,
              endTime: new Date(),
            });
          }
        }

        // Check early stopping
        if (patienceCounter >= config.earlyStoppingPatience) {
          setStatus("completed");
          setError(
            `Early stopping triggered after ${newEpoch} epochs (no improvement for ${config.earlyStoppingPatience} epochs)`,
          );
        }

        // Check max epochs
        if (newEpoch >= config.maxEpochs) {
          setStatus("completed");
          if (newConfidence < config.targetConfidence) {
            setError(
              `Max epochs (${config.maxEpochs}) reached. Target confidence not achieved.`,
            );
          }
        }

        return [...prevMetrics, newMetric];
      });

      return newEpoch;
    });
  }, [config, bestConfidence, patienceCounter, startTime, onSessionUpdate]);

  // Training loop effect
  useEffect(() => {
    let interval: NodeJS.Timeout;

    if (status === "running") {
      interval = setInterval(trainingStep, 100); // Fast simulation: 1 epoch per 100ms
    }

    return () => clearInterval(interval);
  }, [status, trainingStep]);

  const handleStart = () => {
    if (status === "idle") {
      setStartTime(new Date());
      setMetrics([]);
      setCurrentEpoch(0);
      setCurrentConfidence(0);
      setBestConfidence(0);
      setPatienceCounter(0);
      setError(null);
    }
    setStatus("running");
  };

  const handlePause = () => setStatus("paused");

  const handleStop = () => {
    setStatus("completed");
    if (onSessionUpdate) {
      onSessionUpdate({
        id: Date.now().toString(),
        name: `Training Session ${new Date().toLocaleString()}`,
        config,
        metrics,
        bestEpoch: currentEpoch,
        bestConfidence: currentConfidence,
        status: "completed",
        startTime: startTime || undefined,
        endTime: new Date(),
      });
    }
  };

  const handleReset = () => {
    setStatus("idle");
    setMetrics([]);
    setCurrentEpoch(0);
    setCurrentConfidence(0);
    setBestConfidence(0);
    setPatienceCounter(0);
    setStartTime(null);
    setError(null);
  };

  const handleSave = () => {
    console.log("Saving model...", { bestConfidence, currentEpoch, config });
    addNotification({
      type: "success",
      title: "Model Saved",
      message: `Best confidence: ${bestConfidence.toFixed(2)}%`,
    });
  };

  const handleExport = () => {
    const dataStr = JSON.stringify(
      { config, metrics, bestConfidence },
      null,
      2,
    );
    const blob = new Blob([dataStr], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `model-training-${Date.now()}.json`;
    link.click();
  };

  return (
    <div className="space-y-6">
      {/* Status Header */}
      <TrainingStatus
        status={status}
        currentConfidence={currentConfidence}
        targetConfidence={config.targetConfidence}
      />

      {/* Error Display */}
      {error && (
        <motion.div
          initial={{ opacity: 0, height: 0 }}
          animate={{ opacity: 1, height: "auto" }}
          className="p-4 rounded-xl bg-rose-500/10 border border-rose-500/30 text-rose-400"
        >
          <div className="flex items-center gap-2">
            <AlertCircle className="w-5 h-5" />
            <span>{error}</span>
          </div>
        </motion.div>
      )}

      {/* Control Panel */}
      <ControlPanel
        status={status}
        onStart={handleStart}
        onPause={handlePause}
        onStop={handleStop}
        onReset={handleReset}
        onSave={handleSave}
        onExport={handleExport}
        canSave={bestConfidence > 0}
      />

      {/* Training Info Cards */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <InfoCard
          icon={Target}
          label="Target Confidence"
          value={`${config.targetConfidence}%`}
          color="blue"
        />
        <InfoCard
          icon={TrendingUp}
          label="Best Achieved"
          value={`${bestConfidence.toFixed(2)}%`}
          color={
            bestConfidence >= config.targetConfidence ? "emerald" : "amber"
          }
        />
        <InfoCard
          icon={Clock}
          label="Epochs"
          value={`${currentEpoch} / ${config.maxEpochs}`}
          color="purple"
        />
        <InfoCard
          icon={Shield}
          label="Early Stop Patience"
          value={`${patienceCounter} / ${config.earlyStoppingPatience}`}
          color={
            patienceCounter > config.earlyStoppingPatience * 0.7
              ? "rose"
              : "gray"
          }
        />
      </div>
    </div>
  );
}

// Info Card Helper
function InfoCard({
  icon: Icon,
  label,
  value,
  color,
}: {
  icon: React.ElementType;
  label: string;
  value: string;
  color: "blue" | "emerald" | "amber" | "purple" | "rose" | "gray";
}) {
  const colorClasses = {
    blue: "bg-blue-500/10 text-blue-400 border-blue-500/30",
    emerald: "bg-emerald-500/10 text-emerald-400 border-emerald-500/30",
    amber: "bg-amber-500/10 text-amber-400 border-amber-500/30",
    purple: "bg-purple-500/10 text-purple-400 border-purple-500/30",
    rose: "bg-rose-500/10 text-rose-400 border-rose-500/30",
    gray: "bg-gray-500/10 text-gray-400 border-gray-500/30",
  };

  return (
    <div className={`p-4 rounded-xl border ${colorClasses[color]}`}>
      <Icon className="w-5 h-5 mb-2 opacity-70" />
      <p className="text-xs opacity-70 mb-1">{label}</p>
      <p className="text-xl font-bold">{value}</p>
    </div>
  );
}
