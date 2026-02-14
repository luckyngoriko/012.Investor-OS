"use client";

import { useState } from "react";
import { motion } from "framer-motion";
import {
  Target,
  Settings,
  Database,
  Cpu,
  Save,
  RotateCcw,
  Sliders,
  AlertTriangle,
  CheckCircle2,
} from "lucide-react";
import type { TrainingConfig } from "./ai-training-mode";

interface TrainingConfiguratorProps {
  config: TrainingConfig;
  onConfigChange: (config: TrainingConfig) => void;
  isRunning: boolean;
}

export function TrainingConfigurator({
  config,
  onConfigChange,
  isRunning,
}: TrainingConfiguratorProps) {
  const [activeTab, setActiveTab] = useState<"objectives" | "model" | "data">("objectives");
  const [hasChanges, setHasChanges] = useState(false);

  const updateConfig = (updates: Partial<TrainingConfig>) => {
    onConfigChange({ ...config, ...updates });
    setHasChanges(true);
  };

  const handleSave = () => {
    setHasChanges(false);
    // Config is already saved via onConfigChange
  };

  const handleReset = () => {
    onConfigChange({
      targetConfidence: 85,
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
    });
    setHasChanges(false);
  };

  const tabs = [
    { id: "objectives" as const, label: "Objectives", icon: Target },
    { id: "model" as const, label: "Model", icon: Cpu },
    { id: "data" as const, label: "Dataset", icon: Database },
  ];

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-xl bg-blue-500/10 flex items-center justify-center">
            <Settings className="w-5 h-5 text-blue-400" />
          </div>
          <div>
            <h3 className="font-semibold text-white">Training Configuration</h3>
            <p className="text-sm text-gray-500">Set targets and parameters</p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <motion.button
            whileHover={{ scale: 1.02 }}
            whileTap={{ scale: 0.98 }}
            onClick={handleReset}
            disabled={isRunning}
            className="flex items-center gap-2 px-3 py-2 text-gray-400 hover:text-white disabled:opacity-50 transition-colors"
          >
            <RotateCcw className="w-4 h-4" />
            Reset
          </motion.button>
          
          {hasChanges && (
            <motion.button
              initial={{ opacity: 0, scale: 0.9 }}
              animate={{ opacity: 1, scale: 1 }}
              whileHover={{ scale: 1.02 }}
              whileTap={{ scale: 0.98 }}
              onClick={handleSave}
              className="flex items-center gap-2 px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded-lg transition-colors"
            >
              <Save className="w-4 h-4" />
              Save
            </motion.button>
          )}
        </div>
      </div>

      {/* Warning if running */}
      {isRunning && (
        <div className="p-3 rounded-lg bg-amber-500/10 border border-amber-500/30 text-amber-400 text-sm flex items-center gap-2">
          <AlertTriangle className="w-4 h-4" />
          Training is running. Stop to modify configuration.
        </div>
      )}

      {/* Tabs */}
      <div className="flex gap-2">
        {tabs.map((tab) => {
          const Icon = tab.icon;
          const isActive = activeTab === tab.id;
          
          return (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all
                ${isActive 
                  ? "bg-blue-600 text-white" 
                  : "bg-gray-800/50 text-gray-400 hover:text-white hover:bg-gray-800"
                }
              `}
            >
              <Icon className="w-4 h-4" />
              {tab.label}
            </button>
          );
        })}
      </div>

      {/* Tab Content */}
      <div className="glass-card rounded-xl p-6">
        {activeTab === "objectives" && (
          <ObjectivesTab config={config} onUpdate={updateConfig} disabled={isRunning} />
        )}
        {activeTab === "model" && (
          <ModelTab config={config} onUpdate={updateConfig} disabled={isRunning} />
        )}
        {activeTab === "data" && (
          <DataTab config={config} onUpdate={updateConfig} disabled={isRunning} />
        )}
      </div>
    </div>
  );
}

// ============================================
// OBJECTIVES TAB
// ============================================

function ObjectivesTab({
  config,
  onUpdate,
  disabled,
}: {
  config: TrainingConfig;
  onUpdate: (updates: Partial<TrainingConfig>) => void;
  disabled: boolean;
}) {
  return (
    <div className="space-y-6">
      {/* Target Confidence */}
      <div>
        <div className="flex items-center justify-between mb-2">
          <label className="text-sm font-medium text-gray-300 flex items-center gap-2">
            <Target className="w-4 h-4 text-blue-400" />
            Target Confidence
          </label>
          <span className="text-2xl font-bold text-blue-400">{config.targetConfidence}%</span>
        </div>
        <input
          type="range"
          min="50"
          max="99"
          step="0.5"
          value={config.targetConfidence}
          onChange={(e) => onUpdate({ targetConfidence: parseFloat(e.target.value) })}
          disabled={disabled}
          className="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-blue-500 disabled:opacity-50"
        />
        <p className="text-xs text-gray-500 mt-1">
          Training will stop automatically when this confidence level is reached
        </p>
      </div>

      {/* Max Epochs */}
      <div>
        <div className="flex items-center justify-between mb-2">
          <label className="text-sm font-medium text-gray-300">Maximum Epochs</label>
          <span className="text-lg font-bold text-white">{config.maxEpochs}</span>
        </div>
        <input
          type="range"
          min="100"
          max="10000"
          step="100"
          value={config.maxEpochs}
          onChange={(e) => onUpdate({ maxEpochs: parseInt(e.target.value) })}
          disabled={disabled}
          className="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-blue-500 disabled:opacity-50"
        />
      </div>

      {/* Early Stopping */}
      <div className="p-4 rounded-xl bg-gray-800/50 border border-gray-700">
        <h4 className="font-medium text-white mb-4 flex items-center gap-2">
          <Sliders className="w-4 h-4" />
          Early Stopping Criteria
        </h4>
        
        <div className="space-y-4">
          <div>
            <div className="flex items-center justify-between mb-2">
              <label className="text-sm text-gray-400">Patience (epochs)</label>
              <span className="text-white font-medium">{config.earlyStoppingPatience}</span>
            </div>
            <input
              type="range"
              min="10"
              max="200"
              step="10"
              value={config.earlyStoppingPatience}
              onChange={(e) => onUpdate({ earlyStoppingPatience: parseInt(e.target.value) })}
              disabled={disabled}
              className="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-amber-500 disabled:opacity-50"
            />
            <p className="text-xs text-gray-500 mt-1">
              Stop if no improvement for N consecutive epochs
            </p>
          </div>

          <div>
            <div className="flex items-center justify-between mb-2">
              <label className="text-sm text-gray-400">Minimum Improvement (δ)</label>
              <span className="text-white font-medium">{config.minDelta.toFixed(4)}</span>
            </div>
            <input
              type="range"
              min="0.0001"
              max="0.01"
              step="0.0001"
              value={config.minDelta}
              onChange={(e) => onUpdate({ minDelta: parseFloat(e.target.value) })}
              disabled={disabled}
              className="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-amber-500 disabled:opacity-50"
            />
          </div>
        </div>
      </div>

      {/* Summary */}
      <div className="flex items-center gap-3 p-4 rounded-xl bg-blue-500/10 border border-blue-500/30">
        <CheckCircle2 className="w-5 h-5 text-blue-400 flex-shrink-0" />
        <p className="text-sm text-blue-200">
          Training will complete when confidence reaches <strong>{config.targetConfidence}%</strong> or 
          after <strong>{config.maxEpochs}</strong> epochs, whichever comes first.
        </p>
      </div>
    </div>
  );
}

// ============================================
// MODEL TAB
// ============================================

function ModelTab({
  config,
  onUpdate,
  disabled,
}: {
  config: TrainingConfig;
  onUpdate: (updates: Partial<TrainingConfig>) => void;
  disabled: boolean;
}) {
  const modelTypes = [
    { id: "xgboost", name: "XGBoost", description: "Gradient boosting, fast training" },
    { id: "lstm", name: "LSTM", description: "Sequential patterns, time series" },
    { id: "transformer", name: "Transformer", description: "Attention-based, complex patterns" },
    { id: "ensemble", name: "Ensemble", description: "Combined models, best accuracy" },
  ] as const;

  return (
    <div className="space-y-6">
      {/* Model Type */}
      <div>
        <label className="text-sm font-medium text-gray-300 mb-3 block">Model Architecture</label>
        <div className="grid grid-cols-2 gap-3">
          {modelTypes.map((model) => (
            <button
              key={model.id}
              onClick={() => onUpdate({ modelType: model.id })}
              disabled={disabled}
              className={`p-4 rounded-xl border text-left transition-all disabled:opacity-50
                ${config.modelType === model.id
                  ? "bg-blue-500/20 border-blue-500/50 text-white"
                  : "bg-gray-800/50 border-gray-700 text-gray-400 hover:border-gray-600"
                }
              `}
            >
              <div className="font-medium mb-1">{model.name}</div>
              <div className="text-xs opacity-70">{model.description}</div>
            </button>
          ))}
        </div>
      </div>

      {/* Learning Rate */}
      <div>
        <div className="flex items-center justify-between mb-2">
          <label className="text-sm font-medium text-gray-300">Learning Rate</label>
          <span className="text-lg font-bold text-white">{config.learningRate.toFixed(4)}</span>
        </div>
        <input
          type="range"
          min="0.00001"
          max="0.01"
          step="0.00001"
          value={config.learningRate}
          onChange={(e) => onUpdate({ learningRate: parseFloat(e.target.value) })}
          disabled={disabled}
          className="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-purple-500 disabled:opacity-50"
        />
      </div>

      {/* Batch Size */}
      <div>
        <div className="flex items-center justify-between mb-2">
          <label className="text-sm font-medium text-gray-300">Batch Size</label>
          <span className="text-lg font-bold text-white">{config.batchSize}</span>
        </div>
        <input
          type="range"
          min="32"
          max="1024"
          step="32"
          value={config.batchSize}
          onChange={(e) => onUpdate({ batchSize: parseInt(e.target.value) })}
          disabled={disabled}
          className="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-purple-500 disabled:opacity-50"
        />
      </div>

      {/* Checkpoint Settings */}
      <div className="p-4 rounded-xl bg-gray-800/50 border border-gray-700">
        <h4 className="font-medium text-white mb-4">Checkpoint & Saving</h4>
        
        <div className="space-y-4">
          <div>
            <div className="flex items-center justify-between mb-2">
              <label className="text-sm text-gray-400">Checkpoint Interval (epochs)</label>
              <span className="text-white font-medium">{config.checkpointInterval}</span>
            </div>
            <input
              type="range"
              min="1"
              max="100"
              step="1"
              value={config.checkpointInterval}
              onChange={(e) => onUpdate({ checkpointInterval: parseInt(e.target.value) })}
              disabled={disabled}
              className="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-emerald-500 disabled:opacity-50"
            />
          </div>

          <label className="flex items-center gap-3 cursor-pointer">
            <input
              type="checkbox"
              checked={config.autoSave}
              onChange={(e) => onUpdate({ autoSave: e.target.checked })}
              disabled={disabled}
              className="w-4 h-4 rounded border-gray-600 bg-gray-700 text-blue-600 focus:ring-blue-500 disabled:opacity-50"
            />
            <span className="text-sm text-gray-300">Auto-save best model</span>
          </label>
        </div>
      </div>
    </div>
  );
}

// ============================================
// DATA TAB
// ============================================

function DataTab({
  config,
  onUpdate,
  disabled,
}: {
  config: TrainingConfig;
  onUpdate: (updates: Partial<TrainingConfig>) => void;
  disabled: boolean;
}) {
  const trainSize = Math.floor(config.datasetSize * (1 - config.validationSplit));
  const valSize = Math.floor(config.datasetSize * config.validationSplit);

  return (
    <div className="space-y-6">
      {/* Dataset Size */}
      <div>
        <div className="flex items-center justify-between mb-2">
          <label className="text-sm font-medium text-gray-300">Dataset Size</label>
          <span className="text-lg font-bold text-white">{config.datasetSize.toLocaleString()} samples</span>
        </div>
        <input
          type="range"
          min="10000"
          max="1000000"
          step="10000"
          value={config.datasetSize}
          onChange={(e) => onUpdate({ datasetSize: parseInt(e.target.value) })}
          disabled={disabled}
          className="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-emerald-500 disabled:opacity-50"
        />
      </div>

      {/* Validation Split */}
      <div>
        <div className="flex items-center justify-between mb-2">
          <label className="text-sm font-medium text-gray-300">Validation Split</label>
          <span className="text-lg font-bold text-white">{(config.validationSplit * 100).toFixed(0)}%</span>
        </div>
        <input
          type="range"
          min="0.1"
          max="0.4"
          step="0.05"
          value={config.validationSplit}
          onChange={(e) => onUpdate({ validationSplit: parseFloat(e.target.value) })}
          disabled={disabled}
          className="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-emerald-500 disabled:opacity-50"
        />
      </div>

      {/* Data Split Visualization */}
      <div className="p-4 rounded-xl bg-gray-800/50 border border-gray-700">
        <h4 className="font-medium text-white mb-3">Data Distribution</h4>
        
        <div className="space-y-3">
          <div>
            <div className="flex items-center justify-between text-sm mb-1">
              <span className="text-gray-400">Training Set</span>
              <span className="text-white font-medium">{trainSize.toLocaleString()} samples</span>
            </div>
            <div className="h-4 bg-gray-700 rounded-full overflow-hidden">
              <div
                className="h-full bg-blue-500 rounded-full"
                style={{ width: `${(1 - config.validationSplit) * 100}%` }}
              />
            </div>
          </div>
          
          <div>
            <div className="flex items-center justify-between text-sm mb-1">
              <span className="text-gray-400">Validation Set</span>
              <span className="text-white font-medium">{valSize.toLocaleString()} samples</span>
            </div>
            <div className="h-4 bg-gray-700 rounded-full overflow-hidden">
              <div
                className="h-full bg-emerald-500 rounded-full"
                style={{ width: `${config.validationSplit * 100}%` }}
              />
            </div>
          </div>
        </div>
      </div>

      {/* Feature Info */}
      <div className="p-4 rounded-xl bg-gray-800/50 border border-gray-700">
        <h4 className="font-medium text-white mb-2">Features</h4>
        <ul className="text-sm text-gray-400 space-y-1">
          <li>• Technical indicators (RSI, MACD, ATR, Breakout)</li>
          <li>• Insider sentiment & cluster signals</li>
          <li>• News & social sentiment scores</li>
          <li>• Market regime indicators (VIX, breadth)</li>
          <li>• Quality & value composite scores</li>
        </ul>
      </div>
    </div>
  );
}
