"use client";

import { motion } from "framer-motion";
import { 
  User, 
  UserCog, 
  Bot, 
  ArrowRight, 
  CheckCircle2, 
  XCircle,
  Brain,
  TrendingUp,
  Bell,
  AlertTriangle,
  Shield
} from "lucide-react";
import type { TradingMode } from "./trading-mode";

interface TradingFlowDiagramProps {
  mode: TradingMode;
  className?: string;
}

const FlowStep = ({ 
  icon: Icon, 
  title, 
  description, 
  color = "blue",
  active = false,
  decision = false 
}: { 
  icon: React.ElementType;
  title: string;
  description: string;
  color?: string;
  active?: boolean;
  decision?: boolean;
}) => {
  const colorClasses = {
    blue: "bg-blue-500/10 border-blue-500/30 text-blue-400",
    amber: "bg-amber-500/10 border-amber-500/30 text-amber-400",
    emerald: "bg-emerald-500/10 border-emerald-500/30 text-emerald-400",
    rose: "bg-rose-500/10 border-rose-500/30 text-rose-400",
    purple: "bg-purple-500/10 border-purple-500/30 text-purple-400",
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      className={`relative p-4 rounded-xl border ${colorClasses[color as keyof typeof colorClasses]} 
        ${active ? "ring-2 ring-offset-2 ring-offset-[#0a0f1c] ring-blue-500/50" : ""}
        ${decision ? "border-dashed" : ""}`}
    >
      <div className="flex items-start gap-3">
        <div className={`w-10 h-10 rounded-lg flex items-center justify-center bg-gray-800/50`}>
          <Icon className="w-5 h-5" />
        </div>
        <div className="flex-1">
          <h4 className="font-semibold text-white text-sm">{title}</h4>
          <p className="text-xs text-gray-400 mt-1">{description}</p>
        </div>
      </div>
    </motion.div>
  );
};

const FlowArrow = ({ label }: { label?: string }) => (
  <div className="flex flex-col items-center py-2">
    <ArrowRight className="w-5 h-5 text-gray-600 rotate-90" />
    {label && <span className="text-xs text-gray-500 mt-1">{label}</span>}
  </div>
);

const DecisionBranch = ({ 
  yesLabel = "Yes", 
  noLabel = "No",
  yesColor = "emerald",
  noColor = "rose"
}: { 
  yesLabel?: string;
  noLabel?: string;
  yesColor?: string;
  noColor?: string;
}) => (
  <div className="flex justify-center gap-8 py-2">
    <div className="flex flex-col items-center">
      <div className={`px-3 py-1 rounded-full text-xs font-medium bg-${yesColor}-500/20 text-${yesColor}-400`}>
        {yesLabel}
      </div>
      <ArrowRight className="w-4 h-4 text-gray-600 rotate-90 mt-1" />
    </div>
    <div className="flex flex-col items-center">
      <div className={`px-3 py-1 rounded-full text-xs font-medium bg-${noColor}-500/20 text-${noColor}-400`}>
        {noLabel}
      </div>
      <ArrowRight className="w-4 h-4 text-gray-600 rotate-90 mt-1" />
    </div>
  </div>
);

export function TradingFlowDiagram({ mode, className = "" }: TradingFlowDiagramProps) {
  return (
    <div className={`p-6 rounded-2xl glass-card ${className}`}>
      <h3 className="text-lg font-semibold text-white mb-6 flex items-center gap-2">
        <Brain className="w-5 h-5 text-blue-400" />
        How Trading Works in {mode === "manual" ? "Manual" : mode === "semi_auto" ? "Semi-Auto" : "Fully Auto"} Mode
      </h3>

      <div className="space-y-2">
        {/* Step 1: Market Analysis */}
        <FlowStep
          icon={Brain}
          title="1. AI Market Analysis"
          description="AI continuously analyzes market data, insider activity, sentiment, and technical indicators"
          color="blue"
          active
        />
        <FlowArrow />

        {/* Step 2: Proposal Generation */}
        <FlowStep
          icon={TrendingUp}
          title="2. Trade Proposal Generated"
          description="AI creates a proposal with CQ score (0-100%), rationale, and factor breakdown"
          color="purple"
        />
        <FlowArrow />

        {mode === "manual" && (
          <>
            {/* Manual Mode Flow */}
            <FlowStep
              icon={Bell}
              title="3. Notification Sent"
              description="You receive a notification about the new proposal"
              color="amber"
            />
            <FlowArrow />

            <FlowStep
              icon={User}
              title="4. You Review Proposal"
              description="You review the CQ score, rationale, and factors in the dashboard"
              color="blue"
            />
            <FlowArrow />

            <FlowStep
              icon={CheckCircle2}
              title="5. You Execute Trade"
              description="You manually place the trade through your broker interface"
              color="emerald"
            />
          </>
        )}

        {mode === "semi_auto" && (
          <>
            {/* Semi-Auto Mode Flow */}
            <FlowStep
              icon={Bell}
              title="3. Notification Sent"
              description="You receive push/email notification with proposal summary"
              color="amber"
            />
            <FlowArrow />

            <FlowStep
              icon={UserCog}
              title="4. You Review & Decide"
              description="You have 5 minutes to confirm or reject the proposal in the dashboard"
              color="amber"
              decision
            />
            
            <DecisionBranch yesLabel="Confirmed" noLabel="Rejected" />

            <div className="grid grid-cols-2 gap-4">
              <FlowStep
                icon={Bot}
                title="5a. AI Executes Trade"
                description="AI automatically executes the confirmed trade via API"
                color="emerald"
              />
              <FlowStep
                icon={XCircle}
                title="5b. Proposal Rejected"
                description="Proposal dismissed, reason logged for learning"
                color="rose"
              />
            </div>
          </>
        )}

        {mode === "fully_auto" && (
          <>
            {/* Fully Auto Mode Flow */}
            <FlowStep
              icon={Shield}
              title="3. Risk & CQ Check"
              description="System checks if CQ >= threshold AND trade is within risk limits"
              color="amber"
              decision
            />
            
            <DecisionBranch 
              yesLabel="Pass (CQ >= 80%)" 
              noLabel="Fail (CQ < 80%)"
              yesColor="emerald"
              noColor="amber"
            />

            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <FlowStep
                  icon={Bot}
                  title="4a. Auto-Execution"
                  description="AI immediately executes the trade via API"
                  color="emerald"
                />
                <FlowArrow />
                <FlowStep
                  icon={Bell}
                  title="5a. Execution Notification"
                  description="You receive notification of the executed trade"
                  color="blue"
                />
              </div>
              <div className="space-y-2">
                <FlowStep
                  icon={UserCog}
                  title="4b. Queued for Manual"
                  description="Proposal added to queue for your review"
                  color="amber"
                />
                <FlowArrow />
                <FlowStep
                  icon={User}
                  title="5b. You Decide"
                  description="You review and confirm/reject when ready"
                  color="blue"
                />
              </div>
            </div>
          </>
        )}

        {/* Final: Position Opened */}
        <FlowArrow />
        <FlowStep
          icon={TrendingUp}
          title="Position Opened"
          description="Trade completed, position tracked in portfolio"
          color="emerald"
        />
      </div>

      {/* Mode-Specific Notes */}
      <div className="mt-6 p-4 rounded-xl bg-gray-800/30 border border-gray-700/50">
        <h4 className="text-sm font-semibold text-white mb-2 flex items-center gap-2">
          <AlertTriangle className="w-4 h-4 text-amber-400" />
          Important Notes
        </h4>
        <ul className="text-sm text-gray-400 space-y-1">
          {mode === "manual" && (
            <>
              <li>• You have complete control over every trade decision</li>
              <li>• AI provides analysis but never executes</li>
              <li>• Best for learning how the AI thinks</li>
            </>
          )}
          {mode === "semi_auto" && (
            <>
              <li>• Default confirmation timeout: 5 minutes</li>
              <li>• Missed confirmations are treated as rejections</li>
              <li>• You can bulk-confirm similar proposals</li>
            </>
          )}
          {mode === "fully_auto" && (
            <>
              <li>• Trades only execute during market hours</li>
              <li>• All risk limits are enforced automatically</li>
              <li>• Kill switch available at all times</li>
            </>
          )}
        </ul>
      </div>
    </div>
  );
}

// ============================================
// COMPACT MODE COMPARISON TABLE
// ============================================

export function ModeComparisonTable() {
  const features = [
    { name: "AI Generates Proposals", manual: true, semi: true, auto: true },
    { name: "Human Review Required", manual: true, semi: true, auto: false },
    { name: "AI Auto-Executes", manual: false, semi: "After confirm", auto: true },
    { name: "Notification Type", manual: "Proposals", semi: "Proposals", auto: "Executions" },
    { name: "Response Time", manual: "Hours", semi: "Minutes", auto: "Seconds" },
    { name: "Control Level", manual: "Maximum", semi: "Balanced", auto: "Minimum" },
    { name: "Best For", manual: "Learning", semi: "Most Users", auto: "Experienced" },
  ];

  return (
    <div className="overflow-x-auto">
      <table className="w-full">
        <thead>
          <tr className="border-b border-gray-800">
            <th className="text-left py-3 px-4 text-sm font-medium text-gray-400">Feature</th>
            <th className="text-center py-3 px-4 text-sm font-medium text-blue-400">
              <div className="flex items-center justify-center gap-2">
                <User className="w-4 h-4" />
                Manual
              </div>
            </th>
            <th className="text-center py-3 px-4 text-sm font-medium text-amber-400">
              <div className="flex items-center justify-center gap-2">
                <UserCog className="w-4 h-4" />
                Semi-Auto
              </div>
            </th>
            <th className="text-center py-3 px-4 text-sm font-medium text-emerald-400">
              <div className="flex items-center justify-center gap-2">
                <Bot className="w-4 h-4" />
                Fully Auto
              </div>
            </th>
          </tr>
        </thead>
        <tbody>
          {features.map((feature, idx) => (
            <tr key={feature.name} className={idx % 2 === 0 ? "bg-gray-800/20" : ""}>
              <td className="py-3 px-4 text-sm text-gray-300">{feature.name}</td>
              <td className="text-center py-3 px-4">
                {typeof feature.manual === "boolean" ? (
                  feature.manual ? (
                    <CheckCircle2 className="w-5 h-5 text-emerald-400 mx-auto" />
                  ) : (
                    <XCircle className="w-5 h-5 text-rose-400 mx-auto" />
                  )
                ) : (
                  <span className="text-sm text-gray-400">{feature.manual}</span>
                )}
              </td>
              <td className="text-center py-3 px-4">
                {typeof feature.semi === "boolean" ? (
                  feature.semi ? (
                    <CheckCircle2 className="w-5 h-5 text-emerald-400 mx-auto" />
                  ) : (
                    <XCircle className="w-5 h-5 text-rose-400 mx-auto" />
                  )
                ) : (
                  <span className="text-sm text-gray-400">{feature.semi}</span>
                )}
              </td>
              <td className="text-center py-3 px-4">
                {typeof feature.auto === "boolean" ? (
                  feature.auto ? (
                    <CheckCircle2 className="w-5 h-5 text-emerald-400 mx-auto" />
                  ) : (
                    <XCircle className="w-5 h-5 text-rose-400 mx-auto" />
                  )
                ) : (
                  <span className="text-sm text-gray-400">{feature.auto}</span>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
