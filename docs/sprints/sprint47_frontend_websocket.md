# Sprint 47: Frontend WebSocket Real-Time Integration

## Overview
Connect React frontend to HRM WebSocket endpoint for real-time streaming updates. Implement robust WebSocket hook with automatic reconnection and state management.

## Goals
- WebSocket hook (`useHRMWebSocket`) for React components
- Real-time conviction updates in dashboard
- Connection status indicator
- Auto-reconnect on disconnect
- Message buffering during reconnection

## Implementation

### WebSocket Hook
```typescript
// hooks/useHRMWebSocket.ts
export function useHRMWebSocket(url: string) {
  const [connection, setConnection] = useState<WebSocketConnection>();
  const [lastMessage, setLastMessage] = useState<HRMResult | null>();
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>('connecting');
  
  useEffect(() => {
    const ws = new WebSocket(url);
    // ... connection logic
    return () => ws.close();
  }, [url]);
  
  const sendMessage = useCallback((msg: WSClientMessage) => {
    // ... send logic
  }, []);
  
  return { connectionStatus, lastMessage, sendMessage };
}
```

### Hook Features
- ✅ Auto-connect on mount
- ✅ Exponential backoff reconnect
- ✅ Connection status tracking
- ✅ Message queuing while disconnected
- ✅ TypeScript types for all messages
- ✅ Error handling
- ✅ Cleanup on unmount

### Real-Time Dashboard
```typescript
// components/HRMRealtimeDashboard.tsx
export function HRMRealtimeDashboard() {
  const { connectionStatus, lastMessage, sendMessage } = useHRMWebSocket(
    'ws://localhost:8080/ws/hrm'
  );
  
  return (
    <div>
      <ConnectionStatus status={connectionStatus} />
      <ConvictionGauge value={lastMessage?.conviction} />
      <RegimeIndicator regime={lastMessage?.regime} />
    </div>
  );
}
```

## WebSocket Message Types

### Client → Server
```typescript
type WSClientMessage = 
  | { type: 'ping' }
  | { type: 'analyze'; pegy: number; insider: number; sentiment: number; vix: number; regime: number; time: number };
```

### Server → Client
```typescript
type WSServerMessage =
  | { type: 'hrm_result'; timestamp: string; conviction: number; confidence: number; regime: string; should_trade: boolean; recommended_strategy: string; latency_ms: number }
  | { type: 'pong' }
  | { type: 'error'; message: string }
  | { type: 'connected'; message: string };
```

## Connection States
- `connecting` - Initial connection attempt
- `connected` - WebSocket open
- `reconnecting` - Auto-reconnect in progress
- `disconnected` - Connection closed
- `error` - Connection error

## Test Coverage
- Hook initialization
- Message receiving
- Reconnect logic
- Error handling
- Cleanup on unmount

## Status: 🔄 IN PROGRESS

---
**Prev**: Sprint 46 - Performance Monitoring  
**Next**: Sprint 48 - GPU Acceleration (planned)
