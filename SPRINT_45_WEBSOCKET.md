# Sprint 45: WebSocket Streaming

## Цел
Real-time HRM предсказания през WebSocket връзка.

## Features
1. **WebSocket Endpoint** - `/ws/hrm` на backend
2. **Streaming Data** - Пазарни данни → HRM → Client
3. **Auto-reconnect** - При прекъсване
4. **Heartbeat** - Поддържане на връзката
5. **StreamingDashboard** - Real-time UI

## Backend (Rust)
- WebSocket handler с tokio-tungstenite
- Market data streaming
- HRM inference на всеки tick
- Broadcast към всички clients

## Frontend (React)
- WebSocketManager клас
- StreamingDashboard компонент
- Real-time conviction updates
- Connection status indicator
