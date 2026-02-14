"use client";

import { useState } from "react";
import { motion } from "framer-motion";
import { 
  Calculator, TrendingDown, FileText, AlertTriangle,
  CheckCircle2, Download, RefreshCw, DollarSign
} from "lucide-react";
import Sidebar from "@/components/sidebar";

const mockTaxSummary = {
  shortTermGains: 15000,
  longTermGains: 25000,
  shortTermLosses: 3000,
  longTermLosses: 1500,
  washSaleAdjustments: 800,
};

const mockHarvestingOpportunities = [
  { symbol: "TSLA", shares: 25, loss: 1250, replacement: "QQQ", daysToAvoid: 25 },
  { symbol: "META", shares: 10, loss: 450, replacement: "VTI", daysToAvoid: 18 },
];

const mockWashSaleWarnings = [
  { symbol: "AAPL", bought: "2026-01-15", sold: "2026-02-05", adjustment: 320 },
];

export default function TaxPage() {
  const [activeTab, setActiveTab] = useState<"overview" | "harvesting" | "washsale" | "reports">("overview");
  const [jurisdiction, setJurisdiction] = useState("US");

  const netShortTerm = mockTaxSummary.shortTermGains - mockTaxSummary.shortTermLosses;
  const netLongTerm = mockTaxSummary.longTermGains - mockTaxSummary.longTermLosses;
  const shortTermTax = netShortTerm * 0.35;
  const longTermTax = netLongTerm * 0.15;
  const totalTax = shortTermTax + longTermTax;
  const potentialSavings = mockHarvestingOpportunities.reduce((sum, o) => sum + o.loss * 0.35, 0);

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0f1c] via-[#111827] to-[#0a0f1c] flex">
      <Sidebar />
      <main className="flex-1 min-h-screen p-6 lg:p-8">
        <div className="max-w-7xl mx-auto space-y-6">
          <motion.div initial={{ opacity: 0, y: -20 }} animate={{ opacity: 1, y: 0 }}>
            <div className="flex items-center gap-3 mb-2">
              <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-amber-500/20 to-amber-600/10 flex items-center justify-center">
                <Calculator className="w-5 h-5 text-amber-400" />
              </div>
              <div>
                <h1 className="text-2xl font-bold text-white">Tax & Compliance</h1>
                <p className="text-gray-400 text-sm">Sprint 30: Tax loss harvesting, wash sale monitoring, reporting</p>
              </div>
            </div>
          </motion.div>

          <div className="flex gap-2 p-1 bg-gray-800/30 rounded-xl w-fit">
            {[
              { id: "overview", label: "Overview", icon: DollarSign },
              { id: "harvesting", label: "Loss Harvesting", icon: TrendingDown },
              { id: "washsale", label: "Wash Sale", icon: AlertTriangle },
              { id: "reports", label: "Reports", icon: FileText },
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

          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
            <div className="lg:col-span-2 space-y-6">
              {activeTab === "overview" && (
                <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="space-y-6">
                  <div className="glass-card rounded-2xl p-6">
                    <div className="flex items-center justify-between mb-4">
                      <h3 className="text-lg font-semibold text-white">Tax Summary 2026</h3>
                      <select 
                        value={jurisdiction}
                        onChange={(e) => setJurisdiction(e.target.value)}
                        className="px-3 py-1 bg-gray-800 border border-gray-700 rounded-lg text-sm text-white"
                      >
                        <option value="US">United States</option>
                        <option value="UK">United Kingdom</option>
                        <option value="EU">European Union</option>
                        <option value="CA">Canada</option>
                      </select>
                    </div>

                    <div className="grid grid-cols-2 gap-4 mb-6">
                      <div className="p-4 bg-gray-800/30 rounded-xl">
                        <p className="text-sm text-gray-400">Short-Term Gains</p>
                        <p className="text-xl font-bold text-emerald-400">+${mockTaxSummary.shortTermGains.toLocaleString()}</p>
                        <p className="text-xs text-gray-500">Tax rate: 35%</p>
                      </div>
                      <div className="p-4 bg-gray-800/30 rounded-xl">
                        <p className="text-sm text-gray-400">Long-Term Gains</p>
                        <p className="text-xl font-bold text-emerald-400">+${mockTaxSummary.longTermGains.toLocaleString()}</p>
                        <p className="text-xs text-gray-500">Tax rate: 15%</p>
                      </div>
                      <div className="p-4 bg-gray-800/30 rounded-xl">
                        <p className="text-sm text-gray-400">Short-Term Losses</p>
                        <p className="text-xl font-bold text-rose-400">-${mockTaxSummary.shortTermLosses.toLocaleString()}</p>
                      </div>
                      <div className="p-4 bg-gray-800/30 rounded-xl">
                        <p className="text-sm text-gray-400">Long-Term Losses</p>
                        <p className="text-xl font-bold text-rose-400">-${mockTaxSummary.longTermLosses.toLocaleString()}</p>
                      </div>
                    </div>

                    <div className="p-4 bg-blue-500/10 border border-blue-500/30 rounded-xl">
                      <div className="flex items-center justify-between">
                        <div>
                          <p className="text-white font-medium">Estimated Tax Liability</p>
                          <p className="text-2xl font-bold text-blue-400">${totalTax.toLocaleString()}</p>
                        </div>
                        <div className="text-right">
                          <p className="text-sm text-gray-400">Potential Savings</p>
                          <p className="text-xl font-bold text-emerald-400">${potentialSavings.toFixed(0)}</p>
                        </div>
                      </div>
                    </div>
                  </div>
                </motion.div>
              )}

              {activeTab === "harvesting" && (
                <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="glass-card rounded-2xl p-6">
                  <h3 className="text-lg font-semibold text-white mb-4">Tax Loss Harvesting Opportunities</h3>
                  <p className="text-gray-400 text-sm mb-4">Automatic identification of positions with unrealized losses</p>
                  
                  {mockHarvestingOpportunities.map((opp) => (
                    <div key={opp.symbol} className="p-4 bg-gray-800/30 rounded-xl mb-3">
                      <div className="flex items-center justify-between mb-2">
                        <div className="flex items-center gap-3">
                          <span className="text-lg font-bold text-white">{opp.symbol}</span>
                          <span className="text-sm text-gray-400">{opp.shares} shares</span>
                        </div>
                        <span className="text-rose-400 font-bold">-${opp.loss}</span>
                      </div>
                      <div className="flex items-center justify-between text-sm">
                        <div className="text-gray-400">
                          Suggested replacement: <span className="text-blue-400">{opp.replacement}</span>
                        </div>
                        <div className="text-gray-500">
                          {opp.daysToAvoid} days until safe
                        </div>
                      </div>
                      <button className="mt-3 w-full py-2 bg-emerald-600/20 text-emerald-400 rounded-lg hover:bg-emerald-600/30 transition-colors">
                        Execute Harvest
                      </button>
                    </div>
                  ))}
                </motion.div>
              )}

              {activeTab === "washsale" && (
                <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="glass-card rounded-2xl p-6">
                  <div className="flex items-center gap-2 mb-4">
                    <AlertTriangle className="w-5 h-5 text-amber-400" />
                    <h3 className="text-lg font-semibold text-white">Wash Sale Monitor</h3>
                  </div>
                  <p className="text-gray-400 text-sm mb-4">Tracking 30-day window to avoid wash sale violations</p>
                  
                  {mockWashSaleWarnings.map((warning) => (
                    <div key={warning.symbol} className="p-4 bg-amber-500/10 border border-amber-500/30 rounded-xl mb-3">
                      <div className="flex items-center justify-between">
                        <div>
                          <p className="text-white font-medium">{warning.symbol}</p>
                          <p className="text-sm text-gray-400">Bought: {warning.bought} | Sold: {warning.sold}</p>
                        </div>
                        <div className="text-right">
                          <p className="text-amber-400 font-bold">${warning.adjustment}</p>
                          <p className="text-xs text-gray-500">Adjustment</p>
                        </div>
                      </div>
                    </div>
                  ))}

                  <div className="mt-4 p-4 bg-gray-800/30 rounded-xl">
                    <p className="text-sm text-gray-300">Replacement securities suggested to maintain exposure while avoiding wash sale:</p>
                    <div className="flex gap-2 mt-2">
                      <span className="px-2 py-1 text-xs bg-blue-500/20 text-blue-400 rounded">VOO</span>
                      <span className="px-2 py-1 text-xs bg-blue-500/20 text-blue-400 rounded">VTI</span>
                      <span className="px-2 py-1 text-xs bg-blue-500/20 text-blue-400 rounded">QQQ</span>
                    </div>
                  </div>
                </motion.div>
              )}

              {activeTab === "reports" && (
                <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="glass-card rounded-2xl p-6">
                  <h3 className="text-lg font-semibold text-white mb-4">Tax Reports</h3>
                  <div className="space-y-3">
                    {[
                      { name: "Schedule D - Capital Gains", format: "PDF", status: "ready" },
                      { name: "Form 8949 - Sales", format: "PDF", status: "ready" },
                      { name: "Cost Basis Report", format: "CSV", status: "ready" },
                      { name: "TurboTax Export (TXF)", format: "TXF", status: "ready" },
                    ].map((report) => (
                      <div key={report.name} className="flex items-center justify-between p-4 bg-gray-800/30 rounded-xl">
                        <div>
                          <p className="text-white font-medium">{report.name}</p>
                          <p className="text-sm text-gray-400">Format: {report.format}</p>
                        </div>
                        <button className="flex items-center gap-2 px-4 py-2 bg-blue-600/20 text-blue-400 rounded-lg hover:bg-blue-600/30 transition-colors">
                          <Download className="w-4 h-4" />
                          Download
                        </button>
                      </div>
                    ))}
                  </div>
                </motion.div>
              )}
            </div>

            <div className="space-y-6">
              <motion.div initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }} className="glass-card rounded-2xl p-6">
                <h3 className="text-lg font-semibold text-white mb-4">Tax Settings</h3>
                <div className="space-y-4">
                  <div>
                    <p className="text-sm text-gray-400 mb-2">Cost Basis Method</p>
                    <select className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded-lg text-white text-sm">
                      <option>FIFO (First In, First Out)</option>
                      <option>LIFO (Last In, First Out)</option>
                      <option>HIFO (Highest In, First Out)</option>
                      <option>Specific Identification</option>
                    </select>
                  </div>
                  <div>
                    <p className="text-sm text-gray-400 mb-2">Auto-Harvesting</p>
                    <div className="flex items-center justify-between p-3 bg-gray-800/30 rounded-lg">
                      <span className="text-sm text-white">Enabled</span>
                      <div className="w-10 h-5 bg-emerald-500 rounded-full relative">
                        <div className="w-4 h-4 bg-white rounded-full absolute right-0.5 top-0.5" />
                      </div>
                    </div>
                  </div>
                  <div>
                    <p className="text-sm text-gray-400 mb-2">Min Loss Threshold</p>
                    <p className="text-white">$100</p>
                  </div>
                  <div>
                    <p className="text-sm text-gray-400 mb-2">Max Harvests/Month</p>
                    <p className="text-white">10</p>
                  </div>
                </div>
              </motion.div>
            </div>
          </div>
        </div>
      </main>
    </div>
  );
}
