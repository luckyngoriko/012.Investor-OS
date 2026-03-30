"use client";

import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import {
  Brain,
  Clock,
  Cpu,
  Database,
  RefreshCw,
  Settings,
  Zap,
} from "lucide-react";

interface ModelEntry {
  id: string;
  model_name: string;
  model_version: string;
  model_type: string;
  tier: number;
  status: string;
  metrics: Record<string, number>;
  trained_at: string | null;
  created_at: string;
}

function statusBadge(status: string): string {
  switch (status) {
    case "active":
      return "bg-green-500/20 text-green-400 border-green-500/30";
    case "training":
      return "bg-yellow-500/20 text-yellow-400 border-yellow-500/30";
    case "failed":
      return "bg-red-500/20 text-red-400 border-red-500/30";
    case "retired":
      return "bg-gray-500/20 text-gray-400 border-gray-500/30";
    default:
      return "bg-gray-500/20 text-gray-400 border-gray-500/30";
  }
}

function tierIcon(tier: number) {
  if (tier === 1) return <Cpu className="w-4 h-4 text-blue-400" />;
  if (tier === 2) return <Zap className="w-4 h-4 text-purple-400" />;
  return <Database className="w-4 h-4 text-orange-400" />;
}

export default function ModelRegistryPage() {
  const [models, setModels] = useState<ModelEntry[]>([]);
  const [loading, setLoading] = useState(true);

  const fetchModels = async () => {
    setLoading(true);
    try {
      const res = await fetch("/api/models/registry");
      if (res.ok) {
        const data = await res.json();
        setModels(data.data || []);
      }
    } catch {
      // silently fail
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchModels();
  }, []);

  return (
    <div className="min-h-screen text-white">
      <div className="p-6">
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-3xl font-bold flex items-center gap-3">
              <Settings className="w-8 h-8 text-gray-400" />
              Model Registry
            </h1>
            <p className="text-gray-400 mt-1">
              Manage ML models across all tiers
            </p>
          </div>
          <button
            onClick={fetchModels}
            className="flex items-center gap-2 px-4 py-2 bg-gray-800 rounded-lg hover:bg-gray-700 transition"
          >
            <RefreshCw className={`w-4 h-4 ${loading ? "animate-spin" : ""}`} />
            Refresh
          </button>
        </div>

        {models.length === 0 ? (
          <div className="bg-gray-900 rounded-xl border border-gray-800 p-12 text-center">
            <Brain className="w-16 h-16 text-gray-700 mx-auto mb-4" />
            <h2 className="text-xl font-semibold text-gray-400 mb-2">
              No Models Registered
            </h2>
            <p className="text-gray-500">
              Train your first model with{" "}
              <code className="bg-gray-800 px-2 py-1 rounded">
                python scripts/ml/train_catboost.py
              </code>
            </p>
          </div>
        ) : (
          <div className="space-y-4">
            {models.map((m, idx) => (
              <motion.div
                key={m.id}
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: idx * 0.05 }}
                className="bg-gray-900 rounded-xl border border-gray-800 p-6"
              >
                <div className="flex items-center justify-between mb-4">
                  <div className="flex items-center gap-3">
                    {tierIcon(m.tier)}
                    <h3 className="text-lg font-semibold">{m.model_name}</h3>
                    <span className="text-gray-500 text-sm">
                      v{m.model_version}
                    </span>
                    <span
                      className={`text-xs px-2 py-1 rounded-full border ${statusBadge(m.status)}`}
                    >
                      {m.status}
                    </span>
                  </div>
                  <span className="text-gray-600 text-sm">{m.model_type}</span>
                </div>

                {/* Metrics */}
                {Object.keys(m.metrics).length > 0 && (
                  <div className="grid grid-cols-4 gap-4 mt-3">
                    {Object.entries(m.metrics).map(([key, val]) => (
                      <div key={key} className="bg-gray-800/50 rounded-lg p-3">
                        <span className="text-gray-400 text-xs block mb-1">
                          {key.replace(/_/g, " ")}
                        </span>
                        <span className="text-sm font-medium">
                          {typeof val === "number"
                            ? val.toFixed(4)
                            : String(val)}
                        </span>
                      </div>
                    ))}
                  </div>
                )}

                {/* Footer */}
                <div className="flex items-center gap-4 mt-4 text-xs text-gray-600">
                  {m.trained_at && (
                    <span className="flex items-center gap-1">
                      <Clock className="w-3 h-3" />
                      Trained: {new Date(m.trained_at).toLocaleDateString()}
                    </span>
                  )}
                  <span className="flex items-center gap-1">
                    <Clock className="w-3 h-3" />
                    Created: {new Date(m.created_at).toLocaleDateString()}
                  </span>
                </div>
              </motion.div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
