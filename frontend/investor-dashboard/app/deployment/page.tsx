"use client";

import { useState } from "react";
import { motion } from "framer-motion";
import { 
  Server, GitBranch, CheckCircle2, XCircle, Clock,
  RefreshCw, ArrowRight, Shield, Globe, Box
} from "lucide-react";
import Sidebar from "@/components/sidebar";

const environments = [
  { name: "Development", status: "healthy", replicas: 1, version: "1.0.0-dev", branch: "develop", lastDeploy: "10 min ago" },
  { name: "Staging", status: "healthy", replicas: 2, version: "1.0.0-rc1", branch: "main", lastDeploy: "2 hours ago" },
  { name: "Production", status: "healthy", replicas: 5, version: "1.0.0", branch: "v1.0.0", lastDeploy: "1 day ago" },
];

const deploymentSteps = [
  { name: "Test Suite", status: "passed", duration: "4m 32s" },
  { name: "Security Audit", status: "passed", duration: "2m 15s" },
  { name: "Build & Push", status: "passed", duration: "8m 45s" },
  { name: "Deploy to Dev", status: "passed", duration: "1m 20s" },
  { name: "Deploy to Staging", status: "in_progress", duration: "--" },
  { name: "Deploy to Production", status: "pending", duration: "--" },
];

const k8sResources = [
  { type: "Deployment", name: "investor-api", replicas: "5/5", status: "healthy" },
  { type: "Service", name: "investor-api-svc", clusterIP: "10.0.1.42", status: "healthy" },
  { type: "Ingress", name: "investor-api-ingress", host: "api.investor-os.io", status: "healthy" },
  { type: "HPA", name: "investor-api-hpa", range: "5-20", status: "healthy" },
];

export default function DeploymentPage() {
  const [activeTab, setActiveTab] = useState<"overview" | "pipeline" | "kubernetes">("overview");

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0f1c] via-[#111827] to-[#0a0f1c] flex">
      <Sidebar />
      <main className="flex-1 min-h-screen p-6 lg:p-8">
        <div className="max-w-7xl mx-auto space-y-6">
          <motion.div initial={{ opacity: 0, y: -20 }} animate={{ opacity: 1, y: 0 }}>
            <div className="flex items-center gap-3 mb-2">
              <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-cyan-500/20 to-cyan-600/10 flex items-center justify-center">
                <Server className="w-5 h-5 text-cyan-400" />
              </div>
              <div>
                <h1 className="text-2xl font-bold text-white">Deployment & CI/CD</h1>
                <p className="text-gray-400 text-sm">Sprint 35: GitHub Actions, Kubernetes, Canary deployment</p>
              </div>
            </div>
          </motion.div>

          <div className="flex gap-2 p-1 bg-gray-800/30 rounded-xl w-fit">
            {[
              { id: "overview", label: "Overview", icon: Globe },
              { id: "pipeline", label: "CI/CD Pipeline", icon: GitBranch },
              { id: "kubernetes", label: "Kubernetes", icon: Box },
            ].map((tab) => (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id as any)}
                className={`flex items-center gap-2 px-4 py-2 rounded-lg transition-all
                  ${activeTab === tab.id ? "bg-blue-600 text-white" : "text-gray-400 hover:text-white hover:bg-gray-700/50"}`}
              >
                <tab.icon className="w-4 h-4" />
                <span className="text-sm font-medium">{tab.label}</span>
              </button>
            ))}
          </div>

          {activeTab === "overview" && (
            <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="space-y-6">
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                {environments.map((env) => (
                  <div key={env.name} className="glass-card rounded-2xl p-6">
                    <div className="flex items-center justify-between mb-4">
                      <h3 className="text-lg font-semibold text-white">{env.name}</h3>
                      <div className={`w-2 h-2 rounded-full ${env.status === "healthy" ? "bg-emerald-500" : "bg-rose-500"}`} />
                    </div>
                    <div className="space-y-2 text-sm">
                      <div className="flex justify-between">
                        <span className="text-gray-400">Version:</span>
                        <span className="text-white">{env.version}</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-gray-400">Replicas:</span>
                        <span className="text-white">{env.replicas}</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-gray-400">Branch:</span>
                        <span className="text-blue-400">{env.branch}</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-gray-400">Last Deploy:</span>
                        <span className="text-white">{env.lastDeploy}</span>
                      </div>
                    </div>
                  </div>
                ))}
              </div>

              <div className="glass-card rounded-2xl p-6">
                <h3 className="text-lg font-semibold text-white mb-4">Deployment Features</h3>
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
                  <div className="p-4 bg-gray-800/30 rounded-xl">
                    <Shield className="w-6 h-6 text-emerald-400 mb-2" />
                    <p className="text-white font-medium">Security Scanning</p>
                    <p className="text-sm text-gray-400">cargo-audit, Trivy, Hadolint</p>
                  </div>
                  <div className="p-4 bg-gray-800/30 rounded-xl">
                    <Box className="w-6 h-6 text-blue-400 mb-2" />
                    <p className="text-white font-medium">Multi-Arch Builds</p>
                    <p className="text-sm text-gray-400">linux/amd64, linux/arm64</p>
                  </div>
                  <div className="p-4 bg-gray-800/30 rounded-xl">
                    <RefreshCw className="w-6 h-6 text-amber-400 mb-2" />
                    <p className="text-white font-medium">Canary Deployment</p>
                    <p className="text-sm text-gray-400">10% → 100% traffic split</p>
                  </div>
                  <div className="p-4 bg-gray-800/30 rounded-xl">
                    <Globe className="w-6 h-6 text-purple-400 mb-2" />
                    <p className="text-white font-medium">Auto Rollback</p>
                    <p className="text-sm text-gray-400">On error threshold</p>
                  </div>
                </div>
              </div>
            </motion.div>
          )}

          {activeTab === "pipeline" && (
            <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="glass-card rounded-2xl p-6">
              <h3 className="text-lg font-semibold text-white mb-4">CI/CD Pipeline</h3>
              <div className="space-y-4">
                {deploymentSteps.map((step, index) => (
                  <div key={step.name} className="flex items-center gap-4">
                    <div className="w-8 h-8 rounded-full flex items-center justify-center
                      ${step.status === 'passed' ? 'bg-emerald-500/20 text-emerald-400' : ''}
                      ${step.status === 'in_progress' ? 'bg-amber-500/20 text-amber-400 animate-pulse' : ''}
                      ${step.status === 'pending' ? 'bg-gray-700 text-gray-500' : ''}
                    ">
                      {step.status === "passed" ? <CheckCircle2 className="w-5 h-5" /> :
                       step.status === "in_progress" ? <RefreshCw className="w-5 h-5 animate-spin" /> :
                       <Clock className="w-5 h-5" />}
                    </div>
                    <div className="flex-1">
                      <div className="flex items-center justify-between">
                        <p className="text-white font-medium">{step.name}</p>
                        <span className="text-sm text-gray-400">{step.duration}</span>
                      </div>
                    </div>
                    {index < deploymentSteps.length - 1 && (
                      <ArrowRight className="w-4 h-4 text-gray-600" />
                    )}
                  </div>
                ))}
              </div>
            </motion.div>
          )}

          {activeTab === "kubernetes" && (
            <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="glass-card rounded-2xl overflow-hidden">
              <div className="p-6 border-b border-gray-800">
                <h3 className="text-lg font-semibold text-white">Kubernetes Resources</h3>
              </div>
              <table className="w-full">
                <thead>
                  <tr className="text-left text-xs text-gray-500 uppercase">
                    <th className="px-6 py-4">Type</th>
                    <th className="px-6 py-4">Name</th>
                    <th className="px-6 py-4">Details</th>
                    <th className="px-6 py-4">Status</th>
                  </tr>
                </thead>
                <tbody>
                  {k8sResources.map((resource) => (
                    <tr key={resource.name} className="border-t border-gray-800">
                      <td className="px-6 py-4 text-gray-400">{resource.type}</td>
                      <td className="px-6 py-4 text-white">{resource.name}</td>
                      <td className="px-6 py-4 text-gray-400">
                        {resource.replicas || resource.clusterIP || resource.host || resource.range}
                      </td>
                      <td className="px-6 py-4">
                        <span className="px-2 py-1 text-xs rounded-full bg-emerald-500/20 text-emerald-400">
                          {resource.status}
                        </span>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </motion.div>
          )}
        </div>
      </main>
    </div>
  );
}
