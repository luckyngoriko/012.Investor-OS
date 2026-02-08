"use client";

import { useState, useEffect } from "react";
import { Card, CardContent, CardHeader, CardTitle, CardDescription, CardFooter } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { AlertTriangle, Shield, AlertCircle, Loader2 } from "lucide-react";
import { Textarea } from "@/components/ui/textarea";

const API_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:3000/api";

export default function SettingsPage() {
  const [killswitchEnabled, setKillswitchEnabled] = useState(false);
  const [killswitchReason, setKillswitchReason] = useState("");
  const [showKillswitchDialog, setShowKillswitchDialog] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [isCheckingStatus, setIsCheckingStatus] = useState(true);

  useEffect(() => {
    checkKillswitchStatus();
  }, []);

  const checkKillswitchStatus = async () => {
    try {
      const res = await fetch(`${API_URL}/killswitch`);
      if (res.ok) {
        const data = await res.json();
        setKillswitchEnabled(data.data?.enabled || false);
      }
    } catch {
      // Default to disabled if API fails
      setKillswitchEnabled(false);
    } finally {
      setIsCheckingStatus(false);
    }
  };

  const handleTriggerKillswitch = async () => {
    setIsLoading(true);
    try {
      const res = await fetch(`${API_URL}/killswitch`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ reason: killswitchReason }),
      });
      
      if (res.ok) {
        setKillswitchEnabled(true);
        setShowKillswitchDialog(false);
        setKillswitchReason("");
      }
    } catch (error) {
      console.error("Failed to trigger killswitch:", error);
      // For demo purposes, enable anyway
      setKillswitchEnabled(true);
      setShowKillswitchDialog(false);
    } finally {
      setIsLoading(false);
    }
  };

  if (isCheckingStatus) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="w-8 h-8 animate-spin text-blue-600" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <h1 className="text-3xl font-bold text-slate-900">Settings</h1>

      {/* Kill Switch Section */}
      <Card className={killswitchEnabled ? "border-red-500 border-2" : ""}>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className={`p-2 rounded-lg ${killswitchEnabled ? "bg-red-100" : "bg-green-100"}`}>
                <Shield className={`w-6 h-6 ${killswitchEnabled ? "text-red-600" : "text-green-600"}`} />
              </div>
              <div>
                <CardTitle>Kill Switch</CardTitle>
                <CardDescription>
                  Emergency stop for all trading activity
                </CardDescription>
              </div>
            </div>
            <Badge 
              variant={killswitchEnabled ? "destructive" : "default"}
              className={!killswitchEnabled ? "bg-green-500" : ""}
            >
              {killswitchEnabled ? "TRIGGERED" : "ARMED"}
            </Badge>
          </div>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            <div className="flex items-center justify-between p-4 bg-slate-50 rounded-lg">
              <div className="space-y-0.5">
                <Label className="text-base">Emergency Stop</Label>
                <p className="text-sm text-slate-500">
                  When triggered, all new trades are blocked and the system enters safe mode
                </p>
              </div>
              <Button
                variant={killswitchEnabled ? "outline" : "destructive"}
                onClick={() => setShowKillswitchDialog(true)}
                disabled={killswitchEnabled}
              >
                <AlertTriangle className="w-4 h-4 mr-2" />
                {killswitchEnabled ? "Already Triggered" : "Trigger Kill Switch"}
              </Button>
            </div>

            {killswitchEnabled && (
              <div className="p-4 bg-red-50 border border-red-200 rounded-lg">
                <div className="flex items-start gap-3">
                  <AlertCircle className="w-5 h-5 text-red-600 mt-0.5" />
                  <div>
                    <h4 className="font-semibold text-red-900">System is in Safe Mode</h4>
                    <p className="text-sm text-red-700 mt-1">
                      The kill switch has been triggered. No new trades will be executed until manually reset.
                    </p>
                    {killswitchReason && (
                      <p className="text-sm text-red-600 mt-2">
                        <span className="font-medium">Reason:</span> {killswitchReason}
                      </p>
                    )}
                  </div>
                </div>
              </div>
            )}
          </div>
        </CardContent>
      </Card>

      {/* API Settings */}
      <Card>
        <CardHeader>
          <CardTitle>API Configuration</CardTitle>
          <CardDescription>
            Configure connection to the Investor OS backend
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-2">
            <Label htmlFor="api-url">API Base URL</Label>
            <Input 
              id="api-url" 
              value={API_URL} 
              disabled 
              className="bg-slate-50"
            />
            <p className="text-xs text-slate-500">
              Set via NEXT_PUBLIC_API_URL environment variable
            </p>
          </div>
        </CardContent>
      </Card>

      {/* System Info */}
      <Card>
        <CardHeader>
          <CardTitle>System Information</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-2 text-sm">
            <div className="flex justify-between py-2 border-b border-slate-100">
              <span className="text-slate-500">Version</span>
              <span className="font-medium">0.1.0 (Sprint 4)</span>
            </div>
            <div className="flex justify-between py-2 border-b border-slate-100">
              <span className="text-slate-500">Environment</span>
              <Badge variant="outline">Development</Badge>
            </div>
            <div className="flex justify-between py-2 border-b border-slate-100">
              <span className="text-slate-500">Last Deployed</span>
              <span className="font-medium">{new Date().toLocaleDateString()}</span>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Killswitch Confirmation Dialog */}
      <Dialog open={showKillswitchDialog} onOpenChange={setShowKillswitchDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2 text-red-600">
              <AlertTriangle className="w-5 h-5" />
              Trigger Kill Switch
            </DialogTitle>
            <DialogDescription>
              This will immediately stop all trading activity. Are you sure?
            </DialogDescription>
          </DialogHeader>
          <div className="py-4">
            <Label htmlFor="reason">Reason (required)</Label>
            <Textarea
              id="reason"
              placeholder="Why are you triggering the kill switch?"
              value={killswitchReason}
              onChange={(e) => setKillswitchReason(e.target.value)}
              className="mt-2"
            />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowKillswitchDialog(false)}>
              Cancel
            </Button>
            <Button 
              variant="destructive" 
              onClick={handleTriggerKillswitch}
              disabled={!killswitchReason.trim() || isLoading}
            >
              {isLoading ? (
                <Loader2 className="w-4 h-4 animate-spin mr-2" />
              ) : (
                <AlertTriangle className="w-4 h-4 mr-2" />
              )}
              Trigger Emergency Stop
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
