"use client";

import { useState, useEffect } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { Loader2, BookOpen, CheckCircle, XCircle, HelpCircle } from "lucide-react";
import { BackButton } from "@/components/back-button";

const API_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:3000/api";

interface JournalEntry {
  id: string;
  ticker: string;
  decision_type: "BUY" | "SELL" | "HOLD";
  outcome: "WIN" | "LOSS" | "PENDING";
  entry_price: number;
  exit_price?: number;
  pnl?: number;
  notes: string;
  created_at: string;
  closed_at?: string;
}

function OutcomeBadge({ outcome }: { outcome: string }) {
  const variants: Record<string, { color: string; icon: React.ReactNode }> = {
    WIN: { 
      color: "bg-green-100 text-green-800 border-green-300", 
      icon: <CheckCircle className="w-3 h-3" />
    },
    LOSS: { 
      color: "bg-red-100 text-red-800 border-red-300", 
      icon: <XCircle className="w-3 h-3" />
    },
    PENDING: { 
      color: "bg-slate-100 text-slate-800 border-slate-300", 
      icon: <HelpCircle className="w-3 h-3" />
    },
  };
  
  const config = variants[outcome] || variants.PENDING;
  
  return (
    <Badge variant="outline" className={`${config.color} flex items-center gap-1 w-fit`}>
      {config.icon}
      {outcome}
    </Badge>
  );
}

function DecisionBadge({ type }: { type: string }) {
  const colors: Record<string, string> = {
    BUY: "bg-green-100 text-green-800",
    SELL: "bg-red-100 text-red-800",
    HOLD: "bg-slate-100 text-slate-800",
  };
  
  return (
    <Badge className={colors[type] || colors.HOLD}>
      {type}
    </Badge>
  );
}

export default function JournalPage() {
  const [entries, setEntries] = useState<JournalEntry[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [selectedEntry, setSelectedEntry] = useState<JournalEntry | null>(null);

  useEffect(() => {
    fetchEntries();
  }, []);

  const fetchEntries = async () => {
    try {
      const res = await fetch(`${API_URL}/journal`);
      if (!res.ok) throw new Error("Failed to fetch");
      const data = await res.json();
      setEntries(data.data || []);
    } catch {
      // Fallback data for development
      setEntries([
        {
          id: "1",
          ticker: "NVDA",
          decision_type: "BUY",
          outcome: "WIN",
          entry_price: 520.00,
          exit_price: 675.00,
          pnl: 4650.00,
          notes: "Strong earnings momentum, AI tailwinds",
          created_at: "2026-01-05T10:30:00Z",
          closed_at: "2026-01-28T14:20:00Z",
        },
        {
          id: "2",
          ticker: "MSFT",
          decision_type: "BUY",
          outcome: "WIN",
          entry_price: 380.00,
          exit_price: 415.50,
          pnl: 1775.00,
          notes: "Cloud growth accelerating, good valuation",
          created_at: "2026-01-10T09:15:00Z",
          closed_at: "2026-02-01T11:45:00Z",
        },
        {
          id: "3",
          ticker: "GOOGL",
          decision_type: "BUY",
          outcome: "LOSS",
          entry_price: 142.00,
          exit_price: 138.75,
          pnl: -243.75,
          notes: "Regulatory concerns, reduced ad spend",
          created_at: "2026-01-20T13:00:00Z",
          closed_at: "2026-01-25T10:30:00Z",
        },
        {
          id: "4",
          ticker: "AAPL",
          decision_type: "BUY",
          outcome: "PENDING",
          entry_price: 185.50,
          notes: "Strong QVM metrics, positive insider activity",
          created_at: "2026-01-15T11:00:00Z",
        },
        {
          id: "5",
          ticker: "AMZN",
          decision_type: "SELL",
          outcome: "WIN",
          entry_price: 175.00,
          exit_price: 152.30,
          pnl: 1136.25,
          notes: "Margin compression, reduced guidance",
          created_at: "2026-01-12T14:30:00Z",
          closed_at: "2026-01-25T09:00:00Z",
        },
      ]);
    } finally {
      setIsLoading(false);
    }
  };

  const totalTrades = entries.length;
  const winningTrades = entries.filter(e => e.outcome === "WIN").length;
  const losingTrades = entries.filter(e => e.outcome === "LOSS").length;
  const pendingTrades = entries.filter(e => e.outcome === "PENDING").length;
  
  const totalPnl = entries
    .filter(e => e.pnl !== undefined)
    .reduce((sum, e) => sum + (e.pnl || 0), 0);
  
  const winRate = (winningTrades + losingTrades) > 0 
    ? (winningTrades / (winningTrades + losingTrades)) * 100 
    : 0;

  const avgWin = entries
    .filter(e => e.outcome === "WIN" && e.pnl)
    .reduce((sum, e) => sum + (e.pnl || 0), 0) / winningTrades || 0;
    
  const avgLoss = entries
    .filter(e => e.outcome === "LOSS" && e.pnl)
    .reduce((sum, e) => sum + (e.pnl || 0), 0) / losingTrades || 0;

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="w-8 h-8 animate-spin text-blue-600" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <BackButton />
          <h1 className="text-3xl font-bold text-slate-900">Decision Journal</h1>
        </div>
        <Button variant="outline" onClick={fetchEntries}>
          Refresh
        </Button>
      </div>

      {/* Stats Overview */}
      <div className="grid grid-cols-1 md:grid-cols-5 gap-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-slate-600">
              Total P&L
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className={`text-2xl font-bold ${totalPnl >= 0 ? "text-green-600" : "text-red-600"}`}>
              {totalPnl >= 0 ? "+" : ""}
              €{totalPnl.toLocaleString("en-US", { minimumFractionDigits: 2 })}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-slate-600">
              Win Rate
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-slate-900">
              {winRate.toFixed(1)}%
            </div>
            <p className="text-xs text-slate-500 mt-1">
              {winningTrades}W / {losingTrades}L
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-slate-600">
              Avg Win
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-600">
              +€{avgWin.toFixed(2)}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-slate-600">
              Avg Loss
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-red-600">
              €{avgLoss.toFixed(2)}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-slate-600">
              Open Trades
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-slate-900">
              {pendingTrades}
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Journal Entries Table */}
      <Card>
        <CardHeader>
          <CardTitle className="text-lg flex items-center gap-2">
            <BookOpen className="w-5 h-5" />
            Trade History
          </CardTitle>
        </CardHeader>
        <CardContent>
          {entries.length === 0 ? (
            <div className="text-center py-12">
              <p className="text-slate-500">No journal entries yet</p>
              <p className="text-sm text-slate-400 mt-1">
                Entries will be created when you confirm or reject proposals
              </p>
            </div>
          ) : (
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Date</TableHead>
                    <TableHead>Ticker</TableHead>
                    <TableHead>Decision</TableHead>
                    <TableHead className="text-right">Entry</TableHead>
                    <TableHead className="text-right">Exit</TableHead>
                    <TableHead>Outcome</TableHead>
                    <TableHead className="text-right">P&L</TableHead>
                    <TableHead>Notes</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {entries.map((entry) => (
                    <TableRow key={entry.id}>
                      <TableCell className="text-slate-600">
                        {new Date(entry.created_at).toLocaleDateString()}
                      </TableCell>
                      <TableCell className="font-medium">{entry.ticker}</TableCell>
                      <TableCell>
                        <DecisionBadge type={entry.decision_type} />
                      </TableCell>
                      <TableCell className="text-right">
                        €{entry.entry_price.toFixed(2)}
                      </TableCell>
                      <TableCell className="text-right">
                        {entry.exit_price ? `€${entry.exit_price.toFixed(2)}` : "-"}
                      </TableCell>
                      <TableCell>
                        <OutcomeBadge outcome={entry.outcome} />
                      </TableCell>
                      <TableCell className={`text-right font-medium ${
                        entry.pnl && entry.pnl >= 0 ? "text-green-600" : 
                        entry.pnl && entry.pnl < 0 ? "text-red-600" : ""
                      }`}>
                        {entry.pnl !== undefined ? (
                          <>
                            {entry.pnl >= 0 ? "+" : ""}
                            €{entry.pnl.toFixed(2)}
                          </>
                        ) : (
                          "-"
                        )}
                      </TableCell>
                      <TableCell>
                        <Dialog>
                          <DialogTrigger asChild>
                            <Button 
                              variant="ghost" 
                              size="sm"
                              onClick={() => setSelectedEntry(entry)}
                            >
                              View
                            </Button>
                          </DialogTrigger>
                          <DialogContent className="max-w-lg">
                            <DialogHeader>
                              <DialogTitle className="flex items-center gap-2">
                                {entry.ticker}
                                <DecisionBadge type={entry.decision_type} />
                              </DialogTitle>
                              <DialogDescription>
                                Trade details and analysis
                              </DialogDescription>
                            </DialogHeader>
                            <div className="space-y-4 py-4">
                              <div className="grid grid-cols-2 gap-4">
                                <div>
                                  <Label className="text-slate-500">Entry Price</Label>
                                  <p className="font-medium">€{entry.entry_price.toFixed(2)}</p>
                                </div>
                                <div>
                                  <Label className="text-slate-500">Exit Price</Label>
                                  <p className="font-medium">
                                    {entry.exit_price ? `€${entry.exit_price.toFixed(2)}` : "Open"}
                                  </p>
                                </div>
                                <div>
                                  <Label className="text-slate-500">Outcome</Label>
                                  <div className="mt-1">
                                    <OutcomeBadge outcome={entry.outcome} />
                                  </div>
                                </div>
                                <div>
                                  <Label className="text-slate-500">P&L</Label>
                                  <p className={`font-medium ${
                                    entry.pnl && entry.pnl >= 0 ? "text-green-600" : 
                                    entry.pnl && entry.pnl < 0 ? "text-red-600" : ""
                                  }`}>
                                    {entry.pnl !== undefined ? (
                                      <>
                                        {entry.pnl >= 0 ? "+" : ""}
                                        €{entry.pnl.toFixed(2)}
                                      </>
                                    ) : (
                                      "Pending"
                                    )}
                                  </p>
                                </div>
                              </div>
                              <div>
                                <Label className="text-slate-500">Notes</Label>
                                <p className="mt-1 text-sm bg-slate-50 p-3 rounded-lg">
                                  {entry.notes || "No notes recorded"}
                                </p>
                              </div>
                              <div className="text-xs text-slate-400">
                                <p>Opened: {new Date(entry.created_at).toLocaleString()}</p>
                                {entry.closed_at && (
                                  <p>Closed: {new Date(entry.closed_at).toLocaleString()}</p>
                                )}
                              </div>
                            </div>
                          </DialogContent>
                        </Dialog>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
