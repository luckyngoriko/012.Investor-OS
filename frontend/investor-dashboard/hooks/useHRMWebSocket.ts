"use client";

import { useState, useEffect, useCallback, useRef } from "react";

/**
 * WebSocket Connection Status
 */
export type ConnectionStatus = 
  | "connecting" 
  | "connected" 
  | "reconnecting" 
  | "disconnected" 
  | "error";

/**
 * HRM Result from WebSocket
 */
export interface HRMResult {
  timestamp: string;
  conviction: number;
  confidence: number;
  regime: string;
  should_trade: boolean;
  recommended_strategy: string;
  latency_ms: number;
}

/**
 * WebSocket Error Message
 */
export interface WSError {
  message: string;
}

/**
 * Connection Status Message
 */
export interface WSConnected {
  message: string;
}

/**
 * Union type for all server messages
 */
export type WSServerMessage =
  | { type: "hrm_result"; data: HRMResult }
  | { type: "pong" }
  | { type: "error"; data: WSError }
  | { type: "connected"; data: WSConnected };

/**
 * Client message types
 */
export interface WSAnalyzeMessage {
  type: "analyze";
  pegy: number;
  insider: number;
  sentiment: number;
  vix: number;
  regime: number;
  time: number;
}

export type WSClientMessage = 
  | { type: "ping" }
  | WSAnalyzeMessage;

/**
 * Hook return type
 */
export interface UseHRMWebSocketReturn {
  /** Current connection status */
  connectionStatus: ConnectionStatus;
  /** Last received HRM result */
  lastResult: HRMResult | null;
  /** Connection error if any */
  error: string | null;
  /** Send analyze request to server */
  analyze: (params: Omit<WSAnalyzeMessage, "type">) => void;
  /** Send ping to server */
  ping: () => void;
  /** Manually reconnect */
  reconnect: () => void;
  /** Historical results (last 100) */
  history: HRMResult[];
}

/**
 * Configuration options
 */
export interface UseHRMWebSocketOptions {
  /** WebSocket URL */
  url: string;
  /** Auto-connect on mount (default: true) */
  autoConnect?: boolean;
  /** Reconnect on disconnect (default: true) */
  autoReconnect?: boolean;
  /** Initial reconnect delay in ms (default: 1000) */
  reconnectDelay?: number;
  /** Maximum reconnect delay in ms (default: 30000) */
  maxReconnectDelay?: number;
  /** Reconnect backoff multiplier (default: 2) */
  reconnectBackoff?: number;
  /** Maximum reconnect attempts (default: infinite) */
  maxReconnectAttempts?: number;
  /** Ping interval in ms (default: 30000) */
  pingInterval?: number;
  /** Enable debug logging (default: false) */
  debug?: boolean;
}

/**
 * useHRMWebSocket Hook
 * 
 * Provides real-time HRM updates via WebSocket with automatic reconnection.
 * 
 * @example
 * ```tsx
 * function MyComponent() {
 *   const { connectionStatus, lastResult, analyze } = useHRMWebSocket({
 *     url: 'ws://localhost:8080/ws/hrm'
 *   });
 * 
 *   return (
 *     <div>
 *       <p>Status: {connectionStatus}</p>
 *       <p>Conviction: {lastResult?.conviction}</p>
 *       <button onClick={() => analyze({ pegy: 0.8, insider: 0.9, sentiment: 0.7, vix: 15, regime: 0, time: 0.5 })}>
 *         Analyze
 *       </button>
 *     </div>
 *   );
 * }
 * ```
 */
export function useHRMWebSocket(options: UseHRMWebSocketOptions): UseHRMWebSocketReturn {
  const {
    url,
    autoConnect = true,
    autoReconnect = true,
    reconnectDelay = 1000,
    maxReconnectDelay = 30000,
    reconnectBackoff = 2,
    maxReconnectAttempts = Infinity,
    pingInterval = 30000,
    debug = false,
  } = options;

  // State
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>(
    autoConnect ? "connecting" : "disconnected"
  );
  const [lastResult, setLastResult] = useState<HRMResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [history, setHistory] = useState<HRMResult[]>([]);

  // Refs for mutable values that don't trigger re-renders
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectAttemptsRef = useRef(0);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const pingIntervalRef = useRef<NodeJS.Timeout | null>(null);
  const messageQueueRef = useRef<WSClientMessage[]>([]);

  const log = useCallback((...args: unknown[]) => {
    if (debug) {
      console.log("[useHRMWebSocket]", ...args);
    }
  }, [debug]);

  // Clear all timers
  const clearTimers = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }
    if (pingIntervalRef.current) {
      clearInterval(pingIntervalRef.current);
      pingIntervalRef.current = null;
    }
  }, []);

  // Process message queue
  const processMessageQueue = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN && messageQueueRef.current.length > 0) {
      log("Processing message queue:", messageQueueRef.current.length, "messages");
      while (messageQueueRef.current.length > 0) {
        const msg = messageQueueRef.current.shift();
        if (msg) {
          wsRef.current.send(JSON.stringify(msg));
        }
      }
    }
  }, [log]);

  // Connect to WebSocket
  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      log("Already connected");
      return;
    }

    log("Connecting to", url);
    setConnectionStatus("connecting");
    setError(null);

    try {
      const ws = new WebSocket(url);
      wsRef.current = ws;

      ws.onopen = () => {
        log("Connected");
        setConnectionStatus("connected");
        setError(null);
        reconnectAttemptsRef.current = 0;
        
        // Process queued messages
        processMessageQueue();

        // Start ping interval
        pingIntervalRef.current = setInterval(() => {
          if (ws.readyState === WebSocket.OPEN) {
            ws.send(JSON.stringify({ type: "ping" }));
          }
        }, pingInterval);
      };

      ws.onmessage = (event) => {
        try {
          const message: WSServerMessage = JSON.parse(event.data);
          log("Received:", message.type);

          switch (message.type) {
            case "hrm_result":
              setLastResult(message.data);
              setHistory((prev) => {
                const newHistory = [...prev, message.data];
                // Keep last 100 results
                return newHistory.slice(-100);
              });
              break;

            case "connected":
              log("Server says:", message.data.message);
              break;

            case "pong":
              log("Pong received");
              break;

            case "error":
              log("Error:", message.data.message);
              setError(message.data.message);
              break;
          }
        } catch (e) {
          log("Failed to parse message:", e);
        }
      };

      ws.onclose = (event) => {
        log("Disconnected:", event.code, event.reason);
        clearTimers();

        if (autoReconnect && reconnectAttemptsRef.current < maxReconnectAttempts) {
          const delay = Math.min(
            reconnectDelay * Math.pow(reconnectBackoff, reconnectAttemptsRef.current),
            maxReconnectDelay
          );
          
          reconnectAttemptsRef.current++;
          log(`Reconnecting in ${delay}ms (attempt ${reconnectAttemptsRef.current})`);
          setConnectionStatus("reconnecting");
          
          reconnectTimeoutRef.current = setTimeout(() => {
            connect();
          }, delay);
        } else {
          setConnectionStatus("disconnected");
        }
      };

      ws.onerror = (event) => {
        log("Error:", event);
        setConnectionStatus("error");
        setError("WebSocket error occurred");
      };
    } catch (e) {
      log("Connection error:", e);
      setConnectionStatus("error");
      setError(e instanceof Error ? e.message : "Unknown error");
    }
  }, [url, autoReconnect, reconnectDelay, maxReconnectDelay, reconnectBackoff, maxReconnectAttempts, pingInterval, log, processMessageQueue, clearTimers]);

  // Disconnect
  const disconnect = useCallback(() => {
    log("Disconnecting");
    clearTimers();
    
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }
    
    setConnectionStatus("disconnected");
  }, [clearTimers, log]);

  // Send analyze request
  const analyze = useCallback((params: Omit<WSAnalyzeMessage, "type">) => {
    const message: WSAnalyzeMessage = { type: "analyze", ...params };
    
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      log("Sending analyze request");
      wsRef.current.send(JSON.stringify(message));
    } else {
      log("Queueing message (not connected)");
      messageQueueRef.current.push(message);
    }
  }, [log]);

  // Send ping
  const ping = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({ type: "ping" }));
    }
  }, []);

  // Manual reconnect
  const reconnect = useCallback(() => {
    disconnect();
    reconnectAttemptsRef.current = 0;
    connect();
  }, [disconnect, connect]);

  // Connect on mount
  useEffect(() => {
    if (autoConnect) {
      connect();
    }

    return () => {
      disconnect();
    };
  }, [autoConnect, connect, disconnect]);

  return {
    connectionStatus,
    lastResult,
    error,
    analyze,
    ping,
    reconnect,
    history,
  };
}

export default useHRMWebSocket;
