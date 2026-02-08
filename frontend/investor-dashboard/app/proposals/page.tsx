"use client";

import { useState, useEffect } from "react";
import { Card, CardContent, CardHeader, CardTitle, CardFooter } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { 
  Dialog, 
  DialogContent, 
  DialogDescription, 
  DialogHeader, 
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Textarea } from "@/components/ui/textarea";
import { Check, X, Loader2 } from "lucide-react";

const API_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:3000/api";

interface Proposal {
  id: string;
  ticker: string;
  action: string;
  proposed_size: number;
  cq_score: number;
  rationale?: string;
  status: "PENDING" | "CONFIRMED" | "REJECTED";
  created_at: string;
}

function ProposalCard({ 
  proposal, 
  onConfirm, 
  onReject,
  isProcessing 
}: { 
  proposal: Proposal; 
  onConfirm: (id: string) => void;
  onReject: (id: string, reason: string) => void;
  isProcessing: boolean;
}) {
  const [showRejectDialog, setShowRejectDialog] = useState(false);
  const [rejectReason, setRejectReason] = useState("");

  const handleReject = () => {
    onReject(proposal.id, rejectReason);
    setShowRejectDialog(false);
    setRejectReason("");
  };

  const getActionColor = (action: string) => {
    switch (action.toUpperCase()) {
      case "BUY": return "bg-green-100 text-green-800";
      case "SELL": return "bg-red-100 text-red-800";
      default: return "bg-slate-100 text-slate-800";
    }
  };

  return (
    <>
      <Card>
        <CardContent className="pt-6">
          <div className="flex justify-between items-start">
            <div>
              <div className="flex items-center gap-3">
                <h3 className="text-xl font-bold text-slate-900">{proposal.ticker}</h3>
                <Badge className={getActionColor(proposal.action)}>
                  {proposal.action}
                </Badge>
                <Badge variant="outline">
                  {(proposal.proposed_size * 100).toFixed(1)}% position
                </Badge>
              </div>
              <div className="mt-2 flex items-center gap-4 text-sm">
                <span className="text-slate-500">
                  CQ Score: <span className="font-medium text-slate-900">{(proposal.cq_score * 100).toFixed(0)}%</span>
                </span>
                <span className="text-slate-400">
                  {new Date(proposal.created_at).toLocaleDateString()}
                </span>
              </div>
            </div>
            
            {proposal.status === "PENDING" && (
              <div className="flex gap-2">
                <Button 
                  size="sm" 
                  onClick={() => onConfirm(proposal.id)}
                  disabled={isProcessing}
                  className="bg-green-600 hover:bg-green-700"
                >
                  {isProcessing ? (
                    <Loader2 className="w-4 h-4 animate-spin" />
                  ) : (
                    <Check className="w-4 h-4 mr-1" />
                  )}
                  Confirm
                </Button>
                <Button 
                  size="sm" 
                  variant="outline"
                  onClick={() => setShowRejectDialog(true)}
                  disabled={isProcessing}
                  className="border-red-300 text-red-600 hover:bg-red-50"
                >
                  <X className="w-4 h-4 mr-1" />
                  Reject
                </Button>
              </div>
            )}
            
            {proposal.status === "CONFIRMED" && (
              <Badge className="bg-green-100 text-green-800">Confirmed</Badge>
            )}
            
            {proposal.status === "REJECTED" && (
              <Badge variant="destructive">Rejected</Badge>
            )}
          </div>
          
          {proposal.rationale && (
            <div className="mt-4 p-3 bg-slate-50 rounded-lg text-sm text-slate-600">
              <span className="font-medium text-slate-900">Rationale:</span> {proposal.rationale}
            </div>
          )}
        </CardContent>
      </Card>

      <Dialog open={showRejectDialog} onOpenChange={setShowRejectDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Reject Proposal</DialogTitle>
            <DialogDescription>
              Are you sure you want to reject the {proposal.action} proposal for {proposal.ticker}?
            </DialogDescription>
          </DialogHeader>
          <div className="py-4">
            <Textarea
              placeholder="Reason for rejection (optional)..."
              value={rejectReason}
              onChange={(e) => setRejectReason(e.target.value)}
            />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowRejectDialog(false)}>
              Cancel
            </Button>
            <Button variant="destructive" onClick={handleReject}>
              Reject Proposal
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

export default function ProposalsPage() {
  const [proposals, setProposals] = useState<Proposal[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [processingId, setProcessingId] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState("pending");

  useEffect(() => {
    fetchProposals();
  }, []);

  const fetchProposals = async () => {
    try {
      const res = await fetch(`${API_URL}/proposals`);
      if (!res.ok) throw new Error("Failed to fetch");
      const data = await res.json();
      setProposals(data.data?.proposals || []);
    } catch {
      // Fallback data for development
      setProposals([
        {
          id: "1",
          ticker: "AAPL",
          action: "Buy",
          proposed_size: 0.05,
          cq_score: 0.78,
          rationale: "Strong QVM metrics, positive insider activity",
          status: "PENDING",
          created_at: new Date().toISOString(),
        },
        {
          id: "2",
          ticker: "MSFT",
          action: "Buy",
          proposed_size: 0.03,
          cq_score: 0.82,
          rationale: "Excellent value score, momentum building",
          status: "PENDING",
          created_at: new Date().toISOString(),
        },
        {
          id: "3",
          ticker: "TSLA",
          action: "Sell",
          proposed_size: 0.02,
          cq_score: 0.35,
          rationale: "Declining momentum, overvalued",
          status: "CONFIRMED",
          created_at: new Date(Date.now() - 86400000).toISOString(),
        },
      ]);
    } finally {
      setIsLoading(false);
    }
  };

  const handleConfirm = async (id: string) => {
    setProcessingId(id);
    try {
      const res = await fetch(`${API_URL}/proposals/${id}/confirm`, {
        method: "POST",
      });
      if (!res.ok) throw new Error("Failed to confirm");
      
      // Update local state
      setProposals(prev => prev.map(p => 
        p.id === id ? { ...p, status: "CONFIRMED" as const } : p
      ));
    } catch (error) {
      console.error("Error confirming proposal:", error);
      // For development, update state anyway
      setProposals(prev => prev.map(p => 
        p.id === id ? { ...p, status: "CONFIRMED" as const } : p
      ));
    } finally {
      setProcessingId(null);
    }
  };

  const handleReject = async (id: string, reason: string) => {
    setProcessingId(id);
    try {
      const res = await fetch(`${API_URL}/proposals/${id}/reject`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ reason }),
      });
      if (!res.ok) throw new Error("Failed to reject");
      
      setProposals(prev => prev.map(p => 
        p.id === id ? { ...p, status: "REJECTED" as const } : p
      ));
    } catch (error) {
      console.error("Error rejecting proposal:", error);
      setProposals(prev => prev.map(p => 
        p.id === id ? { ...p, status: "REJECTED" as const } : p
      ));
    } finally {
      setProcessingId(null);
    }
  };

  const pendingProposals = proposals.filter(p => p.status === "PENDING");
  const confirmedProposals = proposals.filter(p => p.status === "CONFIRMED");
  const rejectedProposals = proposals.filter(p => p.status === "REJECTED");

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
        <h1 className="text-3xl font-bold text-slate-900">Trade Proposals</h1>
        <div className="flex gap-2">
          <Badge variant="outline" className="px-3 py-1">
            {pendingProposals.length} Pending
          </Badge>
        </div>
      </div>

      <Tabs value={activeTab} onValueChange={setActiveTab}>
        <TabsList>
          <TabsTrigger value="pending">
            Pending ({pendingProposals.length})
          </TabsTrigger>
          <TabsTrigger value="confirmed">
            Confirmed ({confirmedProposals.length})
          </TabsTrigger>
          <TabsTrigger value="rejected">
            Rejected ({rejectedProposals.length})
          </TabsTrigger>
        </TabsList>

        <TabsContent value="pending" className="mt-6">
          {pendingProposals.length === 0 ? (
            <Card>
              <CardContent className="py-12 text-center">
                <p className="text-slate-500">No pending proposals</p>
                <p className="text-sm text-slate-400 mt-1">
                  New proposals will appear here when the decision engine generates them
                </p>
              </CardContent>
            </Card>
          ) : (
            <div className="space-y-4">
              {pendingProposals.map(proposal => (
                <ProposalCard
                  key={proposal.id}
                  proposal={proposal}
                  onConfirm={handleConfirm}
                  onReject={handleReject}
                  isProcessing={processingId === proposal.id}
                />
              ))}
            </div>
          )}
        </TabsContent>

        <TabsContent value="confirmed" className="mt-6">
          {confirmedProposals.length === 0 ? (
            <Card>
              <CardContent className="py-12 text-center">
                <p className="text-slate-500">No confirmed proposals</p>
              </CardContent>
            </Card>
          ) : (
            <div className="space-y-4">
              {confirmedProposals.map(proposal => (
                <ProposalCard
                  key={proposal.id}
                  proposal={proposal}
                  onConfirm={handleConfirm}
                  onReject={handleReject}
                  isProcessing={false}
                />
              ))}
            </div>
          )}
        </TabsContent>

        <TabsContent value="rejected" className="mt-6">
          {rejectedProposals.length === 0 ? (
            <Card>
              <CardContent className="py-12 text-center">
                <p className="text-slate-500">No rejected proposals</p>
              </CardContent>
            </Card>
          ) : (
            <div className="space-y-4">
              {rejectedProposals.map(proposal => (
                <ProposalCard
                  key={proposal.id}
                  proposal={proposal}
                  onConfirm={handleConfirm}
                  onReject={handleReject}
                  isProcessing={false}
                />
              ))}
            </div>
          )}
        </TabsContent>
      </Tabs>
    </div>
  );
}
