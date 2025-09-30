/**
 * Vitest setup file
 * Per spec-kit/008-testing-spec.md
 */

import { vi } from 'vitest';

// Mock navigator.clipboard for paste tests
Object.defineProperty(navigator, 'clipboard', {
  value: {
    readText: vi.fn(),
    writeText: vi.fn(),
  },
  writable: true,
});

// Mock window.prompt for search dialog
global.prompt = vi.fn();

// Mock WebSocket for WebSocketClient tests
class MockWebSocket {
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSING = 2;
  static CLOSED = 3;

  readyState: number = MockWebSocket.CONNECTING;
  onopen: ((event: Event) => void) | null = null;
  onclose: ((event: CloseEvent) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;

  constructor(public url: string) {
    // Simulate async connection
    setTimeout(() => {
      this.readyState = MockWebSocket.OPEN;
      this.onopen?.(new Event('open'));
    }, 0);
  }

  send(data: string): void {
    if (this.readyState !== MockWebSocket.OPEN) {
      throw new Error('WebSocket is not open');
    }
  }

  close(code?: number, reason?: string): void {
    this.readyState = MockWebSocket.CLOSED;
    const event = new CloseEvent('close', { code: code ?? 1000, reason: reason ?? '' });
    this.onclose?.(event);
  }
}

global.WebSocket = MockWebSocket as unknown as typeof WebSocket;
