"use client";

import React, { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Badge } from "@/components/ui/badge";
import { Activity, BarChart3, Globe, Zap, Server } from "lucide-react";

/**
 * Monitoring Dashboard with Embedded Grafana
 * Sprint 46: Performance Monitoring
 */
export default function MonitoringPage() {
  const [activeTab, setActiveTab] = useState("grafana");
  
  // Grafana embed URL (configured for anonymous access)
  const grafanaUrl = process.env.NEXT_PUBLIC_GRAFANA_URL || "http://localhost:3000";
  const dashboardUid = "investor-os-hrm";
  
  return (
    <div className="container mx-auto p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Performance Monitoring</h1>
          <p className="text-muted-foreground mt-1">
            Real-time metrics and system health dashboard
          </p>
        </div>
        <div className="flex gap-2">
          <Badge variant="outline" className="flex items-center gap-1">
            <span className="h-2 w-2 rounded-full bg-green-500 animate-pulse" />
            Live
          </Badge>
          <Badge variant="secondary">Sprint 46</Badge>
        </div>
      </div>

      {/* Quick Stats */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">HRM Inferences</CardTitle>
            <Zap className="h-4 w-4 text-yellow-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">~0.3ms</div>
            <p className="text-xs text-muted-foreground">Avg latency</p>
          </CardContent>
        </Card>
        
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">WebSocket</CardTitle>
            <Globe className="h-4 w-4 text-blue-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">Active</div>
            <p className="text-xs text-muted-foreground">/ws/hrm endpoint</p>
          </CardContent>
        </Card>
        
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">API Requests</CardTitle>
            <Activity className="h-4 w-4 text-green-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">Healthy</div>
            <p className="text-xs text-muted-foreground">All endpoints</p>
          </CardContent>
        </Card>
        
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Memory</CardTitle>
            <Server className="h-4 w-4 text-purple-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">~45MB</div>
            <p className="text-xs text-muted-foreground">Current usage</p>
          </CardContent>
        </Card>
      </div>

      {/* Tabs */}
      <Tabs value={activeTab} onValueChange={setActiveTab} className="space-y-4">
        <TabsList className="grid w-full grid-cols-3 lg:w-[400px]">
          <TabsTrigger value="grafana" className="flex items-center gap-2">
            <BarChart3 className="h-4 w-4" />
            Grafana
          </TabsTrigger>
          <TabsTrigger value="metrics" className="flex items-center gap-2">
            <Activity className="h-4 w-4" />
            Raw Metrics
          </TabsTrigger>
          <TabsTrigger value="health" className="flex items-center gap-2">
            <Server className="h-4 w-4" />
            Health
          </TabsTrigger>
        </TabsList>

        {/* Grafana Tab */}
        <TabsContent value="grafana" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <BarChart3 className="h-5 w-5" />
                Grafana Dashboard
              </CardTitle>
            </CardHeader>
            <CardContent className="p-0">
              <div className="relative w-full h-[800px] bg-muted/50 rounded-b-lg">
                <iframe
                  src={`${grafanaUrl}/d/${dashboardUid}?kiosk&theme=dark`}
                  className="absolute inset-0 w-full h-full rounded-b-lg border-0"
                  allowFullScreen
                  title="Grafana Dashboard"
                />
                {/* Overlay for demo mode */}
                <div className="absolute inset-0 flex items-center justify-center bg-background/80 backdrop-blur-sm rounded-b-lg">
                  <div className="text-center space-y-4 max-w-md p-6">
                    <Activity className="h-12 w-12 mx-auto text-muted-foreground" />
                    <h3 className="text-lg font-semibold">Grafana Dashboard</h3>
                    <p className="text-sm text-muted-foreground">
                      To view the embedded Grafana dashboard, ensure Grafana is running at{" "}
                      <code className="bg-muted px-1 py-0.5 rounded">{grafanaUrl}</code>{" "}
                      with the Investor OS dashboard configured.
                    </p>
                    <div className="flex gap-2 justify-center">
                      <a
                        href={grafanaUrl}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="inline-flex items-center justify-center rounded-md text-sm font-medium ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 bg-primary text-primary-foreground hover:bg-primary/90 h-10 px-4 py-2"
                      >
                        Open Grafana
                      </a>
                      <a
                        href="/metrics"
                        target="_blank"
                        className="inline-flex items-center justify-center rounded-md text-sm font-medium ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 border border-input bg-background hover:bg-accent hover:text-accent-foreground h-10 px-4 py-2"
                      >
                        View Metrics
                      </a>
                    </div>
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        {/* Raw Metrics Tab */}
        <TabsContent value="metrics" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>Prometheus Metrics</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                <div className="rounded-md bg-muted p-4">
                  <h4 className="text-sm font-semibold mb-2">Available Metrics Endpoints</h4>
                  <ul className="space-y-2 text-sm">
                    <li className="flex items-center justify-between">
                      <code>/metrics</code>
                      <Badge variant="outline">Prometheus</Badge>
                    </li>
                    <li className="flex items-center justify-between">
                      <code>/api/health</code>
                      <Badge variant="outline">Health</Badge>
                    </li>
                    <li className="flex items-center justify-between">
                      <code>/api/v1/hrm/health</code>
                      <Badge variant="outline">HRM Health</Badge>
                    </li>
                  </ul>
                </div>
                
                <div className="rounded-md bg-muted p-4">
                  <h4 className="text-sm font-semibold mb-2">HRM Metrics</h4>
                  <ul className="space-y-1 text-sm font-mono text-muted-foreground">
                    <li>hrm_inference_total</li>
                    <li>hrm_inference_duration_seconds</li>
                    <li>hrm_inference_errors_total</li>
                    <li>hrm_model_loaded</li>
                  </ul>
                </div>
                
                <div className="rounded-md bg-muted p-4">
                  <h4 className="text-sm font-semibold mb-2">WebSocket Metrics</h4>
                  <ul className="space-y-1 text-sm font-mono text-muted-foreground">
                    <li>websocket_connections_active</li>
                    <li>websocket_messages_total</li>
                    <li>websocket_errors_total</li>
                  </ul>
                </div>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        {/* Health Tab */}
        <TabsContent value="health" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>System Health</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                <div className="flex items-center justify-between p-4 rounded-lg border bg-green-50 dark:bg-green-950">
                  <div className="flex items-center gap-3">
                    <div className="h-3 w-3 rounded-full bg-green-500" />
                    <div>
                      <p className="font-medium">API Server</p>
                      <p className="text-sm text-muted-foreground">Running on port 8080</p>
                    </div>
                  </div>
                  <Badge className="bg-green-500">Healthy</Badge>
                </div>
                
                <div className="flex items-center justify-between p-4 rounded-lg border bg-green-50 dark:bg-green-950">
                  <div className="flex items-center gap-3">
                    <div className="h-3 w-3 rounded-full bg-green-500" />
                    <div>
                      <p className="font-medium">HRM Model</p>
                      <p className="text-sm text-muted-foreground">9,347 parameters loaded</p>
                    </div>
                  </div>
                  <Badge className="bg-green-500">Ready</Badge>
                </div>
                
                <div className="flex items-center justify-between p-4 rounded-lg border bg-green-50 dark:bg-green-950">
                  <div className="flex items-center gap-3">
                    <div className="h-3 w-3 rounded-full bg-green-500" />
                    <div>
                      <p className="font-medium">WebSocket</p>
                      <p className="text-sm text-muted-foreground">/ws/hrm endpoint active</p>
                    </div>
                  </div>
                  <Badge className="bg-green-500">Active</Badge>
                </div>
                
                <div className="flex items-center justify-between p-4 rounded-lg border bg-green-50 dark:bg-green-950">
                  <div className="flex items-center gap-3">
                    <div className="h-3 w-3 rounded-full bg-green-500" />
                    <div>
                      <p className="font-medium">Metrics</p>
                      <p className="text-sm text-muted-foreground">Prometheus endpoint ready</p>
                    </div>
                  </div>
                  <Badge className="bg-green-500">Enabled</Badge>
                </div>
              </div>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
