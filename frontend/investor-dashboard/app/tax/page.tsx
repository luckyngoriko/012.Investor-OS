"use client";

import { useEffect, useMemo, useState } from "react";
import { motion } from "framer-motion";
import {
  AlertTriangle,
  Calculator,
  CheckCircle2,
  DollarSign,
  Download,
  FileText,
  RefreshCw,
  TrendingDown,
} from "lucide-react";
import Sidebar from "@/components/sidebar";
import {
  type TaxCalculationResponse,
  type TaxStatusResponse,
  fetchTaxCalculation,
  fetchTaxStatus,
} from "@/lib/domain-api";

type TaxTab = "overview" | "harvesting" | "washsale" | "reports";

function formatCurrency(value: number): string {
  return `$${value.toLocaleString(undefined, { maximumFractionDigits: 2 })}`;
}

export default function TaxPage() {
  const [activeTab, setActiveTab] = useState<TaxTab>("overview");
  const [jurisdiction, setJurisdiction] = useState("US");
  const [statusPayload, setStatusPayload] = useState<TaxStatusResponse | null>(null);
  const [calculationPayload, setCalculationPayload] = useState<TaxCalculationResponse | null>(
    null,
  );
  const [isLoading, setIsLoading] = useState(true);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;

    const loadTaxData = async () => {
      setIsLoading(true);
      setErrorMessage(null);
      try {
        const [status, calculation] = await Promise.all([
          fetchTaxStatus(),
          fetchTaxCalculation(),
        ]);

        if (!mounted) return;
        setStatusPayload(status);
        setCalculationPayload(calculation);
      } catch (error) {
        if (!mounted) return;
        setErrorMessage(error instanceof Error ? error.message : "Failed to load tax data");
      } finally {
        if (mounted) {
          setIsLoading(false);
        }
      }
    };

    void loadTaxData();

    return () => {
      mounted = false;
    };
  }, []);

  const calculations = calculationPayload?.calculations;
  const estimatedTax = calculations?.estimated_tax;
  const shortTermGains = calculations?.short_term_gains ?? 0;
  const longTermGains = calculations?.long_term_gains ?? 0;
  const shortTermTax = estimatedTax?.short_term ?? 0;
  const longTermTax = estimatedTax?.long_term ?? 0;
  const totalTax = estimatedTax?.total ?? 0;

  const potentialSavings = useMemo(() => {
    if (!calculationPayload) return 0;
    return calculationPayload.optimization_opportunities.reduce((sum, opportunity) => {
      const source = opportunity.tax_savings ?? opportunity.potential_savings ?? "$0";
      const normalized = Number.parseFloat(source.replace(/[^0-9.-]/g, ""));
      return sum + (Number.isFinite(normalized) ? normalized : 0);
    }, 0);
  }, [calculationPayload]);

  const harvestingFeature = statusPayload?.features.find((feature) =>
    feature.name.includes("Tax Loss Harvesting"),
  );
  const washSaleFeature = statusPayload?.features.find((feature) =>
    feature.name.includes("Wash Sale"),
  );
  const reportingFeature = statusPayload?.features.find((feature) =>
    feature.name.includes("Tax Reporting"),
  );
  const reportFormats = reportingFeature?.formats ?? [];

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
                <p className="text-gray-400 text-sm">
                  Backend-driven tax calculation, harvesting, and reporting visibility
                </p>
              </div>
            </div>
          </motion.div>

          {errorMessage && (
            <div className="rounded-xl border border-rose-500/40 bg-rose-500/10 p-4 text-sm text-rose-300">
              {errorMessage}
            </div>
          )}

          <div className="flex gap-2 p-1 bg-gray-800/30 rounded-xl w-fit">
            {[
              { id: "overview", label: "Overview", icon: DollarSign },
              { id: "harvesting", label: "Loss Harvesting", icon: TrendingDown },
              { id: "washsale", label: "Wash Sale", icon: AlertTriangle },
              { id: "reports", label: "Reports", icon: FileText },
            ].map((tab) => (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id as TaxTab)}
                className={`flex items-center gap-2 px-4 py-2 rounded-lg transition-all ${
                  activeTab === tab.id
                    ? "bg-blue-600 text-white"
                    : "text-gray-400 hover:text-white hover:bg-gray-700/50"
                }`}
              >
                <tab.icon className="w-4 h-4" />
                <span className="text-sm font-medium">{tab.label}</span>
              </button>
            ))}
          </div>

          {isLoading ? (
            <div className="glass-card rounded-2xl p-6 text-gray-300 flex items-center gap-3">
              <RefreshCw className="w-4 h-4 animate-spin" />
              Loading tax data...
            </div>
          ) : null}

          {!isLoading && (
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
              <div className="lg:col-span-2 space-y-6">
                {activeTab === "overview" && (
                  <motion.div
                    initial={{ opacity: 0, y: 20 }}
                    animate={{ opacity: 1, y: 0 }}
                    className="space-y-6"
                  >
                    <div className="glass-card rounded-2xl p-6">
                      <div className="flex items-center justify-between mb-4">
                        <h3 className="text-lg font-semibold text-white">
                          Tax Summary {calculationPayload?.tax_year ?? new Date().getFullYear()}
                        </h3>
                        <select
                          value={jurisdiction}
                          onChange={(event) => setJurisdiction(event.target.value)}
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
                          <p className="text-xl font-bold text-emerald-400">
                            {formatCurrency(shortTermGains)}
                          </p>
                          <p className="text-xs text-gray-500">
                            Tax rate: {calculations?.short_term_rate ?? "n/a"}
                          </p>
                        </div>
                        <div className="p-4 bg-gray-800/30 rounded-xl">
                          <p className="text-sm text-gray-400">Long-Term Gains</p>
                          <p className="text-xl font-bold text-emerald-400">
                            {formatCurrency(longTermGains)}
                          </p>
                          <p className="text-xs text-gray-500">
                            Tax rate: {calculations?.long_term_rate ?? "n/a"}
                          </p>
                        </div>
                        <div className="p-4 bg-gray-800/30 rounded-xl">
                          <p className="text-sm text-gray-400">Short-Term Tax</p>
                          <p className="text-xl font-bold text-rose-400">
                            {formatCurrency(shortTermTax)}
                          </p>
                        </div>
                        <div className="p-4 bg-gray-800/30 rounded-xl">
                          <p className="text-sm text-gray-400">Long-Term Tax</p>
                          <p className="text-xl font-bold text-rose-400">
                            {formatCurrency(longTermTax)}
                          </p>
                        </div>
                      </div>

                      <div className="p-4 bg-blue-500/10 border border-blue-500/30 rounded-xl">
                        <div className="flex items-center justify-between">
                          <div>
                            <p className="text-white font-medium">Estimated Tax Liability</p>
                            <p className="text-2xl font-bold text-blue-400">
                              {formatCurrency(totalTax)}
                            </p>
                          </div>
                          <div className="text-right">
                            <p className="text-sm text-gray-400">Potential Savings</p>
                            <p className="text-xl font-bold text-emerald-400">
                              {formatCurrency(potentialSavings)}
                            </p>
                          </div>
                        </div>
                      </div>
                    </div>
                  </motion.div>
                )}

                {activeTab === "harvesting" && (
                  <motion.div
                    initial={{ opacity: 0, y: 20 }}
                    animate={{ opacity: 1, y: 0 }}
                    className="glass-card rounded-2xl p-6"
                  >
                    <h3 className="text-lg font-semibold text-white mb-4">
                      Tax Loss Harvesting Opportunities
                    </h3>
                    <p className="text-gray-400 text-sm mb-4">
                      {harvestingFeature?.description ??
                        "Automatic identification of loss-harvesting opportunities"}
                    </p>

                    {calculationPayload?.optimization_opportunities.length ? (
                      calculationPayload.optimization_opportunities.map((opportunity) => (
                        <div
                          key={opportunity.action}
                          className="p-4 bg-gray-800/30 rounded-xl mb-3"
                        >
                          <div className="flex items-center justify-between mb-2">
                            <p className="text-white font-medium">{opportunity.action}</p>
                            <span className="text-emerald-400 font-bold">
                              {opportunity.tax_savings ?? opportunity.potential_savings ?? "n/a"}
                            </span>
                          </div>
                          <div className="text-sm text-gray-400">
                            {opportunity.replacement ?? opportunity.reason ?? "No extra details"}
                          </div>
                        </div>
                      ))
                    ) : (
                      <div className="p-4 bg-gray-800/30 rounded-xl text-sm text-gray-400">
                        No harvesting opportunities returned by backend.
                      </div>
                    )}
                  </motion.div>
                )}

                {activeTab === "washsale" && (
                  <motion.div
                    initial={{ opacity: 0, y: 20 }}
                    animate={{ opacity: 1, y: 0 }}
                    className="glass-card rounded-2xl p-6"
                  >
                    <div className="flex items-center gap-2 mb-4">
                      <AlertTriangle className="w-5 h-5 text-amber-400" />
                      <h3 className="text-lg font-semibold text-white">Wash Sale Monitor</h3>
                    </div>
                    <p className="text-gray-400 text-sm mb-4">
                      {washSaleFeature?.description ??
                        "Tracking 30-day window to avoid wash-sale violations"}
                    </p>

                    <div className="p-4 bg-amber-500/10 border border-amber-500/30 rounded-xl mb-3">
                      <p className="text-white font-medium mb-1">Current status</p>
                      <p className="text-sm text-gray-300">
                        {calculationPayload?.harvesting_status ?? "No status from backend"}
                      </p>
                    </div>

                    <div className="p-4 bg-gray-800/30 rounded-xl">
                      <p className="text-sm text-gray-300">
                        Replacement securities guidance:
                      </p>
                      <p className="text-sm text-blue-300 mt-2">
                        {washSaleFeature?.replacement_securities ??
                          "Use correlated alternatives to avoid wash-sale conflicts."}
                      </p>
                    </div>
                  </motion.div>
                )}

                {activeTab === "reports" && (
                  <motion.div
                    initial={{ opacity: 0, y: 20 }}
                    animate={{ opacity: 1, y: 0 }}
                    className="glass-card rounded-2xl p-6"
                  >
                    <h3 className="text-lg font-semibold text-white mb-4">Tax Reports</h3>
                    <div className="space-y-3">
                      {reportFormats.length ? (
                        reportFormats.map((format) => (
                          <div
                            key={format}
                            className="flex items-center justify-between p-4 bg-gray-800/30 rounded-xl"
                          >
                            <div>
                              <p className="text-white font-medium">Tax Report Export</p>
                              <p className="text-sm text-gray-400">Format: {format}</p>
                            </div>
                            <button className="flex items-center gap-2 px-4 py-2 bg-blue-600/20 text-blue-400 rounded-lg hover:bg-blue-600/30 transition-colors">
                              <Download className="w-4 h-4" />
                              Download
                            </button>
                          </div>
                        ))
                      ) : (
                        <div className="p-4 bg-gray-800/30 rounded-xl text-sm text-gray-400">
                          No report formats available from backend.
                        </div>
                      )}
                    </div>
                  </motion.div>
                )}
              </div>

              <div className="space-y-6">
                <motion.div
                  initial={{ opacity: 0, x: 20 }}
                  animate={{ opacity: 1, x: 0 }}
                  className="glass-card rounded-2xl p-6"
                >
                  <h3 className="text-lg font-semibold text-white mb-4">Tax Settings</h3>
                  <div className="space-y-4">
                    <div>
                      <p className="text-sm text-gray-400 mb-2">Backend Jurisdiction</p>
                      <p className="text-white">{statusPayload?.jurisdiction ?? "n/a"}</p>
                    </div>
                    <div>
                      <p className="text-sm text-gray-400 mb-2">Selected Jurisdiction</p>
                      <p className="text-white">{jurisdiction}</p>
                    </div>
                    <div>
                      <p className="text-sm text-gray-400 mb-2">Min Loss Threshold</p>
                      <p className="text-white">
                        {harvestingFeature?.min_loss_threshold ?? "n/a"}
                      </p>
                    </div>
                    <div>
                      <p className="text-sm text-gray-400 mb-2">Max Harvests/Month</p>
                      <p className="text-white">
                        {harvestingFeature?.max_harvests_per_month ?? "n/a"}
                      </p>
                    </div>
                    <div className="flex items-center gap-2 p-3 bg-emerald-500/10 rounded-lg">
                      <CheckCircle2 className="w-4 h-4 text-emerald-400" />
                      <span className="text-sm text-emerald-300">
                        Tax engine connected to backend endpoints
                      </span>
                    </div>
                  </div>
                </motion.div>
              </div>
            </div>
          )}
        </div>
      </main>
    </div>
  );
}
