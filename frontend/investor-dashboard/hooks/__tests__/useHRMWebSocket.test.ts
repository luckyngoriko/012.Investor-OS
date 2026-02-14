/**
 * Tests for useHRMWebSocket hook
 * Sprint 47: Frontend WebSocket Integration
 */

import { renderHook, act, waitFor } from "@testing-library/react";
import { useHRMWebSocket } from "../useHRMWebSocket";

// Mock WebSocket
class MockWebSocket {
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSING = 2;
  static CLOSED = 3;

  url: string;
  readyState = MockWebSocket.CONNECTING;
  onopen: ((event: Event) => void) | null = null;
  onclose: ((event: CloseEvent) => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;

  constructor(url: string) {
    this.url = url;
    setTimeout(() => {
      this.readyState = MockWebSocket.OPEN;
      this.onopen?.(new Event("open"));
    }, 10);
  }

  send(data: string) {}
  close() {
    this.readyState = MockWebSocket.CLOSED;
    this.onclose?.(new CloseEvent("close"));
  }
}

global.WebSocket = MockWebSocket as unknown as typeof WebSocket;

describe("useHRMWebSocket", () => {
  const defaultOptions = {
    url: "ws://localhost:8080/ws/hrm",
    autoConnect: true,
    autoReconnect: false,
  };

  it("should initialize with connecting status", () => {
    const { result } = renderHook(() => useHRMWebSocket(defaultOptions));
    expect(result.current.connectionStatus).toBe("connecting");
    expect(result.current.lastResult).toBeNull();
  });

  it("should connect to WebSocket", async () => {
    const { result } = renderHook(() => useHRMWebSocket(defaultOptions));
    await waitFor(() => {
      expect(result.current.connectionStatus).toBe("connected");
    });
  });
});
