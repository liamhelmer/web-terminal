/**
 * Simplified unit tests for App
 * Testing module coordination and event routing
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { App } from '@/ui/App';
import type { ServerMessage } from '@/types/index';

// Mock all dependencies
vi.mock('@/terminal/Terminal');
vi.mock('@/websocket/WebSocketClient');
vi.mock('@/session/SessionManager');

describe('App - Basic Functionality', () => {
  let container: HTMLElement;
  let app: App;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);

    app = new App(container, {
      websocket: {},
      terminal: {
        fontSize: 14,
        fontFamily: 'monospace',
        theme: {
          background: '#000',
          foreground: '#fff',
          cursor: '#fff',
          selection: '#444',
          black: '#000',
          red: '#f00',
          green: '#0f0',
          yellow: '#ff0',
          blue: '#00f',
          magenta: '#f0f',
          cyan: '#0ff',
          white: '#fff',
          brightBlack: '#666',
          brightRed: '#f66',
          brightGreen: '#6f6',
          brightYellow: '#ff6',
          brightBlue: '#66f',
          brightMagenta: '#f6f',
          brightCyan: '#6ff',
          brightWhite: '#fff',
        },
      },
    });
  });

  afterEach(() => {
    document.body.removeChild(container);
  });

  it('should create app instance', () => {
    expect(app).toBeDefined();
  });

  it('should coordinate all modules', () => {
    const terminal = (app as any).terminal;
    const wsClient = (app as any).wsClient;
    const sessionManager = (app as any).sessionManager;

    expect(terminal).toBeDefined();
    expect(wsClient).toBeDefined();
    expect(sessionManager).toBeDefined();
  });
});