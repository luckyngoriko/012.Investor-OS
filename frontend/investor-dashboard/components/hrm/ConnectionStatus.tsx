"use client";

import React from "react";
import { Badge } from "@/components/ui/badge";
import { 
  Wifi, 
  WifiOff, 
  Loader2, 
  AlertCircle,
  RefreshCw
} from "lucide-react";
import { ConnectionStatus as Status } from "@/hooks/useHRMWebSocket";

interface ConnectionStatusProps {
  status: Status;
  error?: string | null;
  onReconnect?: () => void;
  className?: string;
}

/**
 * ConnectionStatus Component
 * 
 * Displays WebSocket connection status with appropriate icon and color.
 */
export function ConnectionStatus({
  status,
  error,
  onReconnect,
  className = "",
}: ConnectionStatusProps) {
  const config = {
    connecting: {
      icon: Loader2,
      label: "Connecting",
      variant: "outline" as const,
      className: "animate-spin",
      color: "text-yellow-500",
    },
    connected: {
      icon: Wifi,
      label: "Live",
      variant: "default" as const,
      className: "",
      color: "text-green-500",
    },
    reconnecting: {
      icon: RefreshCw,
      label: "Reconnecting",
      variant: "secondary" as const,
      className: "animate-spin",
      color: "text-yellow-500",
    },
    disconnected: {
      icon: WifiOff,
      label: "Offline",
      variant: "destructive" as const,
      className: "",
      color: "text-red-500",
    },
    error: {
      icon: AlertCircle,
      label: "Error",
      variant: "destructive" as const,
      className: "",
      color: "text-red-500",
    },
  };

  const { icon: Icon, label, variant, className: iconClass, color } = config[status];

  return (
    <div className={`flex items-center gap-2 ${className}`}>
      <Badge 
        variant={variant} 
        className={`flex items-center gap-1.5 ${color}`}
      >
        <Icon className={`h-3 w-3 ${iconClass}`} />
        {label}
      </Badge>
      
      {error && status === "error" && (
        <span className="text-xs text-destructive">
          {error}
        </span>
      )}
      
      {(status === "disconnected" || status === "error") && onReconnect && (
        <button
          onClick={onReconnect}
          className="text-xs text-primary hover:underline"
        >
          Reconnect
        </button>
      )}
    </div>
  );
}

export default ConnectionStatus;
