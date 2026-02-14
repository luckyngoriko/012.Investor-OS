# Sprint 45: WebSocket Real-Time Streaming

## Overview
Implement WebSocket endpoint for real-time HRM streaming with automatic updates.

## Endpoint
```
WS /ws/hrm
```

## Protocol

### Client → Server
```json
// Ping
{"type": "ping"}

// Analyze request
{
  "type": "analyze",
  "pegy": 0.8,
  "insider": 0.9,
  "sentiment": 0.7,
  "vix": 15.0,
  "regime": 0.0,
  "time": 0.5
}
```

### Server → Client
```json
// HRM Result
{
  "type": "hrm_result",
  "timestamp": "2026-02-12T08:23:42Z",
  "conviction": 0.92,
  "confidence": 0.99,
  "regime": "StrongUptrend",
  "should_trade": true,
  "recommended_strategy": "Momentum",
  "latency_ms": 0.5
}

// Pong
{"type": "pong"}

// Error
{
  "type": "error",
  "message": "Invalid input: pegy out of range"
}

// Connected
{
  "type": "connected",
  "message": "HRM WebSocket Connected"
}
```

## Implementation

### Thread Safety Note
Burn tensors are !Send, so WebSocket uses heuristic calculations instead of ML model. This is a known limitation with workarounds:
- Option A: Thread-local storage for tensors
- Option B: Separate inference thread with channels
- Option C: Eager initialization before tokio runtime

### Auto-Streaming
Server sends simulated market data every 5 seconds for demo purposes.

## Status: ✅ COMPLETE

- [x] WebSocket endpoint /ws/hrm
- [x] Message protocol
- [x] Auto-streaming
- [x] Error handling
- [x] Thread-safe implementation
- [ ] ML model integration (deferred)

## Known Limitations
- Uses heuristic calculations (not ML) due to burn tensor Send bounds
- ML integration requires architecture changes

---
**Prev**: Sprint 44 - Frontend Dashboard  
**Next**: Sprint 46 - Performance Monitoring (planned)
