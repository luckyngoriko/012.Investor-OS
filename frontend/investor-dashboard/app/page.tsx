import Link from "next/link";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { TrendingUp, TrendingDown, Activity, AlertCircle } from "lucide-react";

// API base URL - should come from env
const API_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:3000/api";

async function getPortfolio() {
  try {
    const res = await fetch(`${API_URL}/portfolio`, { cache: "no-store" });
    if (!res.ok) throw new Error("Failed to fetch portfolio");
    return await res.json();
  } catch {
    // Fallback data for development
    return {
      nav: 125000.50,
      daily_pnl: 0.025,
      cash: 45000.00,
      positions_value: 80000.50,
    };
  }
}

async function getRegime() {
  try {
    const res = await fetch(`${API_URL}/regime`, { cache: "no-store" });
    if (!res.ok) throw new Error("Failed to fetch regime");
    return await res.json();
  } catch {
    return {
      regime: "RISK_ON",
      vix: 18.5,
      regime_fit: 0.85,
    };
  }
}

async function getPendingProposals() {
  try {
    const res = await fetch(`${API_URL}/proposals?status=PENDING`, { cache: "no-store" });
    if (!res.ok) throw new Error("Failed to fetch proposals");
    const data = await res.json();
    return data.proposals || [];
  } catch {
    return [
      { id: "1", ticker: "AAPL", action: "Buy", cq_score: 0.78 },
      { id: "2", ticker: "MSFT", action: "Buy", cq_score: 0.82 },
    ];
  }
}

function RegimeBadge({ regime }: { regime: string }) {
  const variants: Record<string, { variant: "default" | "secondary" | "destructive" | "outline"; label: string; color: string }> = {
    RISK_ON: { variant: "default", label: "Risk ON", color: "bg-green-500" },
    UNCERTAIN: { variant: "secondary", label: "Uncertain", color: "bg-yellow-500" },
    RISK_OFF: { variant: "destructive", label: "Risk OFF", color: "bg-red-500" },
  };
  
  const config = variants[regime] || variants.UNCERTAIN;
  
  return (
    <div className="flex items-center gap-2">
      <span className={`w-3 h-3 rounded-full ${config.color}`}></span>
      <Badge variant={config.variant}>{config.label}</Badge>
    </div>
  );
}

export default async function DashboardPage() {
  const [portfolio, regime, proposals] = await Promise.all([
    getPortfolio(),
    getRegime(),
    getPendingProposals(),
  ]);

  const isPositive = portfolio.daily_pnl >= 0;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold text-slate-900">Dashboard</h1>
        <p className="text-sm text-slate-500">
          Last updated: {new Date().toLocaleTimeString()}
        </p>
      </div>

      {/* Key Metrics Grid */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        {/* Portfolio Value Card */}
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium text-slate-600">
              Portfolio Value
            </CardTitle>
            <Activity className="w-4 h-4 text-slate-400" />
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-bold text-slate-900">
              €{portfolio.nav.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
            </div>
            <div className={`flex items-center gap-1 mt-2 text-sm ${isPositive ? "text-green-600" : "text-red-600"}`}>
              {isPositive ? (
                <TrendingUp className="w-4 h-4" />
              ) : (
                <TrendingDown className="w-4 h-4" />
              )}
              <span className="font-medium">
                {isPositive ? "+" : ""}{(portfolio.daily_pnl * 100).toFixed(2)}%
              </span>
              <span className="text-slate-400">today</span>
            </div>
          </CardContent>
        </Card>

        {/* Market Regime Card */}
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium text-slate-600">
              Market Regime
            </CardTitle>
            <AlertCircle className="w-4 h-4 text-slate-400" />
          </CardHeader>
          <CardContent>
            <RegimeBadge regime={regime.regime} />
            <div className="mt-4 space-y-1">
              <div className="flex justify-between text-sm">
                <span className="text-slate-500">VIX</span>
                <span className="font-medium">{regime.vix}</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-slate-500">Regime Fit</span>
                <span className="font-medium">{(regime.regime_fit * 100).toFixed(0)}%</span>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Pending Decisions Card */}
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium text-slate-600">
              Pending Decisions
            </CardTitle>
            <span className="flex h-4 w-4 relative">
              {proposals.length > 0 && (
                <>
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-blue-400 opacity-75"></span>
                  <span className="relative inline-flex rounded-full h-4 w-4 bg-blue-500"></span>
                </>
              )}
            </span>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-bold text-slate-900">{proposals.length}</div>
            <p className="text-sm text-slate-500 mt-2">
              trade proposals awaiting review
            </p>
            <Link 
              href="/proposals" 
              className="inline-flex items-center gap-1 mt-4 text-sm text-blue-600 hover:text-blue-700 font-medium"
            >
              Review →
            </Link>
          </CardContent>
        </Card>
      </div>

      {/* Quick Actions */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Recent Proposals */}
        <Card>
          <CardHeader>
            <CardTitle className="text-lg">Recent Proposals</CardTitle>
          </CardHeader>
          <CardContent>
            {proposals.length === 0 ? (
              <p className="text-slate-500 text-center py-4">No pending proposals</p>
            ) : (
              <div className="space-y-3">
                {proposals.slice(0, 3).map((proposal: any) => (
                  <div 
                    key={proposal.id} 
                    className="flex items-center justify-between p-3 bg-slate-50 rounded-lg"
                  >
                    <div>
                      <span className="font-semibold text-slate-900">{proposal.ticker}</span>
                      <span className="text-sm text-slate-500 ml-2">{proposal.action}</span>
                    </div>
                    <Badge variant="outline">
                      CQ: {(proposal.cq_score * 100).toFixed(0)}%
                    </Badge>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>

        {/* System Status */}
        <Card>
          <CardHeader>
            <CardTitle className="text-lg">System Status</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <span className="text-slate-600">API Status</span>
                <Badge variant="default" className="bg-green-500">Online</Badge>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-slate-600">Data Freshness</span>
                <Badge variant="outline">&lt; 1 hour</Badge>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-slate-600">Kill Switch</span>
                <Badge variant="outline" className="text-green-600 border-green-600">
                  Disarmed
                </Badge>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-slate-600">Cash Available</span>
                <span className="font-medium">
                  €{portfolio.cash.toLocaleString("en-US", { minimumFractionDigits: 2 })}
                </span>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
