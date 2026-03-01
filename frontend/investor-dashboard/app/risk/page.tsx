"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { AlertTriangle, ShieldCheck } from "lucide-react";

export default function RiskPage() {
  return (
    <div className="container mx-auto p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Risk Management</h1>
          <p className="text-muted-foreground mt-1">
            Portfolio safeguards, limits, and automated control checks.
          </p>
        </div>
        <Badge variant="outline" className="flex items-center gap-2">
          <ShieldCheck className="h-4 w-4" />
          Active
        </Badge>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <AlertTriangle className="h-5 w-5" />
            Control Checks
          </CardTitle>
        </CardHeader>
        <CardContent>
          <ul className="space-y-2 text-sm text-muted-foreground">
            <li>Maximum position size limit validation</li>
            <li>Daily loss threshold and drawdown guardrails</li>
            <li>Pre-trade exposure and concentration checks</li>
          </ul>
        </CardContent>
      </Card>
    </div>
  );
}
