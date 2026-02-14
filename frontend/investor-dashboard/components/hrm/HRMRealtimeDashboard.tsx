"use client";

import React, { useEffect, useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { 
  Activity, 
  BarChart3, 
  Zap, 
  History,
  Settings,
  Play,
  Pause
} from "lucide-react";
import { useHRMWebSocket } from "@/hooks/useHRMWebSocket";
import { ConnectionStatus } from "./ConnectionStatus";

interface HRMRealtimeDashboardProps {
  wsUrl?: string;
}

/**
 * HRM Realtime Dashboard
 * 
 * Main dashboard component with WebSocket integration for real-time updates.
 */
export function HRMRealtimeDashboard({ 
  wsUrl = process.env.NEXT_PUBLIC_WS_URL || "ws://localhost:8080/ws/hrm"
}: HRMRealtimeDashboardProps) {
  const [autoStream, setAutoStream] = useState(false);
  
  const {
    connectionStatus,
    lastResult,
    error,
    analyze,
    reconnect,
    history,
  } = useHRMWebSocket({
    url: wsUrl,
    debug: process.env.NODE_ENV === "development",
  });

  // Auto-stream effect
  useEffect(() => {
    if (!autoStream || connectionStatus !== "connected") return;

    analyze({
      pegy: 0.8,
      insider: 0.9,
      sentiment: 0.7,
      vix: 15,
      regime: 0,
      time: 0.5,
    });
  }, [autoStream, connectionStatus, analyze]);

  const handleAnalyze = (values: {
    pegy: number;
    insider: number;
    sentiment: number;
    vix: number;
    regime: number;
    time: number;
  }) => {
    analyze(values);
  };

  return (
    <div className="container mx-auto p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">HRM Real-Time Dashboard</h1>
          <p className="text-muted-foreground mt-1">
            Live conviction streaming via WebSocket
          </p>
        </div>
        <ConnectionStatus 
          status={connectionStatus} 
          error={error}
          onReconnect={reconnect}
        />
      </div>

      {/* Quick Stats */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Latest Conviction</CardTitle>
            <Zap className="h-4 w-4 text-yellow-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {lastResult ? `${(lastResult.conviction * 100).toFixed(1)}%` : "--"}
            </div>
            <p className="text-xs text-muted-foreground">
              {lastResult ? `Latency: ${lastResult.latency_ms.toFixed(1)}ms` : "Waiting..."}
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Confidence</CardTitle>
            <Activity className="h-4 w-4 text-blue-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {lastResult ? `${(lastResult.confidence * 100).toFixed(1)}%` : "--"}
            </div>
            <p className="text-xs text-muted-foreground">
              {lastResult?.confidence && lastResult.confidence > 0.7 ? "High" : "Low"}
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Regime</CardTitle>
            <BarChart3 className="h-4 w-4 text-purple-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {lastResult ? lastResult.regime : "--"}
            </div>
            <p className="text-xs text-muted-foreground">
              {lastResult?.should_trade ? (
                <Badge variant="default" className="bg-green-500">Trade</Badge>
              ) : (
                <Badge variant="secondary">Hold</Badge>
              )}
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">History</CardTitle>
            <History className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{history.length}</div>
            <p className="text-xs text-muted-foreground">Stored (max 100)</p>
          </CardContent>
        </Card>
      </div>

      {/* Controls */}
      <Card>
        <CardHeader>
          <CardTitle>Controls</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex gap-4">
            <Button
              variant={autoStream ? "destructive" : "default"}
              onClick={() => setAutoStream(!autoStream)}
              disabled={connectionStatus !== "connected"}
            >
              {autoStream ? (
                <><Pause className="h-4 w-4 mr-2" /> Stop</>
              ) : (
                <><Play className="h-4 w-4 mr-2" /> Auto-Stream</>
              )}
            </Button>
          </div>
          
          {lastResult && (
            <div className="rounded-lg bg-muted p-4">
              <h4 className="font-medium mb-2">Last Result</h4>
              <pre className="text-sm text-muted-foreground overflow-x-auto">
                {JSON.stringify(lastResult, null, 2)}
              </pre>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

export default HRMRealtimeDashboard;
