# Sprint 43: HRM REST API

## Overview
Implement REST API endpoints for HRM inference with JSON request/response formats.

## Endpoints

### POST /api/v1/hrm/infer
Single inference request.

**Request:**
```json
{
  "pegy": 0.8,
  "insider": 0.9,
  "sentiment": 0.7,
  "vix": 15.0,
  "regime": 0.0,
  "time": 0.5
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "conviction": 0.9294,
    "confidence": 0.9956,
    "regime": "StrongUptrend",
    "should_trade": true,
    "recommended_strategy": "Momentum",
    "signal_strength": 0.9253,
    "source": "MLModel",
    "latency_ms": 0.3
  }
}
```

### POST /api/v1/hrm/batch
Batch inference (up to 100 samples).

### GET /api/v1/hrm/health
Health check endpoint.

## Implementation

### Thread-Safe Engine
```rust
// src/api/handlers/hrm.rs
static HRM_ENGINE: OnceLock<Mutex<StrategySelectorEngine>> = OnceLock::new();

pub async fn hrm_infer(
    Json(request): Json<HRMInferenceRequest>,
) -> Json<ApiResponse<HRMInferenceResponse>> {
    let engine = get_hrm_engine().lock().unwrap();
    let result = engine.calculate_conviction(&signals);
    // ... return response
}
```

## Status: ✅ COMPLETE

- [x] Single inference endpoint
- [x] Batch inference endpoint
- [x] Health check endpoint
- [x] Input validation
- [x] Error handling
- [x] 8 API tests passing

---
**Prev**: Sprint 42 - Strategy Integration  
**Next**: Sprint 44 - Frontend Dashboard
