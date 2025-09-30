/**
 * Unit tests for App
 * Per spec-kit/008-testing-spec.md and 004-frontend-spec.md section 3.4
 */

import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { App } from '@/ui/App';
import type { ServerMessage } from '@/types/index';

// Mock Terminal class
vi.mock('@/terminal/Terminal', () => ({
  Terminal: vi.fn().mockImplementation(() => ({
    open: vi.fn(),
    write: vi.fn(),
    writeError: vi.fn(),
    writeInfo: vi.fn(),
    dispose: vi.fn(),
    onData: vi.fn(),
    onResize: vi.fn(),
  })),
}));

// Mock WebSocketClient class
vi.mock('@/websocket/WebSocketClient', () => ({
  WebSocketClient: vi.fn().mockImplementation(() => ({
    connect: vi.fn().mockResolvedValue(undefined),
    disconnect: vi.fn().mockResolvedValue(undefined),
    send: vi.fn(),
    onMessage: vi.fn(),
    onConnectionChange: vi.fn(),
  })),
}));

// Mock SessionManager class
vi.mock('@/session/SessionManager', () => ({
  SessionManager: vi.fn().mockImplementation(() => ({
    createSession: vi.fn().mockResolvedValue({
      id: 'test-session-123',
      created: Date.now(),
      lastActivity: Date.now(),
    }),
    getCurrentSession: vi.fn(() => null),
  })),
}));

describe('App', () => {
  let container: HTMLElement;
  let app: App;
  let mockTerminal: any;
  let mockWsClient: any;
  let mockSessionManager: any;

  beforeEach(async () => {
    container = document.createElement('div');
    document.body.appendChild(container);

    // Reset all mocks
    vi.clearAllMocks();

    // Create app (this creates mocked instances)
    app = new App(container, {
      websocket: { token: 'test-token' },
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

    // Get references to mocked instances
    mockTerminal = (app as any).terminal;
    mockWsClient = (app as any).wsClient;
    mockSessionManager = (app as any).sessionManager;
  });

  afterEach(async () => {
    try {
      await app.stop();
    } catch (e) {
      // Ignore errors in cleanup
    }
    document.body.removeChild(container);
  });

  describe('Initialization', () => {
    it('should create app instance', () => {
      expect(app).toBeDefined();
    });

    it('should setup event handlers on creation', () => {
      // Handlers should be registered
      expect(mockTerminal.onData).toHaveBeenCalled();
      expect(mockWsClient.onMessage).toHaveBeenCalled();
      expect(mockWsClient.onConnectionChange).toHaveBeenCalled();
      expect(mockTerminal.onResize).toHaveBeenCalled();
    });
  });

  describe('Application Lifecycle', () => {
    it('should start successfully', async () => {
      await app.start();

      expect(mockWsClient.connect).toHaveBeenCalled();
      expect(mockTerminal.open).toHaveBeenCalled();
      expect(mockSessionManager.createSession).toHaveBeenCalled();
    });

    it('should start components in correct order', async () => {
      const callOrder: string[] = [];

      mockWsClient.connect = vi.fn(() => {
        callOrder.push('connect');
        return Promise.resolve();
      });

      mockTerminal.open = vi.fn(() => {
        callOrder.push('open');
      });

      mockSessionManager.createSession = vi.fn(() => {
        callOrder.push('createSession');
        return Promise.resolve({ id: 'test-session', created: Date.now(), lastActivity: Date.now() });
      });

      await app.start();

      expect(callOrder).toEqual(['connect', 'open', 'createSession']);
    });

    it('should prevent double start', async () => {
      await app.start();

      const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

      await app.start();

      expect(consoleSpy).toHaveBeenCalledWith('App already started');
      expect(mockWsClient.connect).toHaveBeenCalledTimes(1);

      consoleSpy.mockRestore();
    });

    it('should handle start failure', async () => {
      const error = new Error('Connection failed');
      mockWsClient.connect = vi.fn().mockRejectedValue(error);

      await expect(app.start()).rejects.toThrow('Connection failed');

      expect(mockTerminal.writeError).toHaveBeenCalled();
    });

    it('should stop successfully', async () => {
      await app.start();
      await app.stop();

      expect(mockTerminal.dispose).toHaveBeenCalled();
      expect(mockWsClient.disconnect).toHaveBeenCalled();
    });

    it('should handle stop when not started', async () => {
      // Should not throw
      await app.stop();

      expect(mockTerminal.dispose).not.toHaveBeenCalled();
      expect(mockWsClient.disconnect).not.toHaveBeenCalled();
    });
  });

  describe('Terminal to WebSocket', () => {
    it('should send terminal input to WebSocket', () => {
      const dataCallback = mockTerminal.onData.mock.calls[0][0];
      dataCallback('ls -la\n');

      expect(mockWsClient.send).toHaveBeenCalledWith({
        type: 'command',
        data: 'ls -la\n',
      });
    });

    it('should send resize events to WebSocket', () => {
      const resizeCallback = mockTerminal.onResize.mock.calls[0][0];
      resizeCallback(80, 24);

      expect(mockWsClient.send).toHaveBeenCalledWith({
        type: 'resize',
        cols: 80,
        rows: 24,
      });
    });
  });

  describe('WebSocket to Terminal', () => {
    it('should write output messages to terminal', () => {
      const messageHandler = mockWsClient.onMessage.mock.calls[0][0];
      const message: ServerMessage = { type: 'output', stream: 'stdout', data: 'Hello World\n' };
      messageHandler(message);

      expect(mockTerminal.write).toHaveBeenCalledWith('Hello World\n');
    });

    it('should display error messages', async () => {
      await app.start();

      const messageHandler = app.getMessageHandler();
      messageHandler!({
        type: 'error',
        code: 'COMMAND_NOT_FOUND',
        message: 'Command not found',
      });

      expect(mockTerminal.writeError).toHaveBeenCalledWith('Error: Command not found');
    });

    it('should display process started messages', async () => {
      await app.start();

      const messageHandler = app.getMessageHandler();
      messageHandler!({
        type: 'process_started',
        pid: 1234,
        command: 'ls -la',
      });

      expect(mockTerminal.writeInfo).toHaveBeenCalledWith('[Process 1234 started: ls -la]');
    });

    it('should display process exited messages', async () => {
      await app.start();

      const messageHandler = app.getMessageHandler();
      messageHandler!({
        type: 'process_exited',
        pid: 1234,
        exit_code: 0,
        signal: null,
      });

      expect(mockTerminal.writeInfo).toHaveBeenCalledWith('[Process 1234 exited with code 0]');
    });

    it('should display process terminated messages', async () => {
      await app.start();

      const messageHandler = app.getMessageHandler();
      messageHandler!({
        type: 'process_exited',
        pid: 1234,
        exit_code: 1,
        signal: 'SIGTERM',
      });

      expect(mockTerminal.writeInfo).toHaveBeenCalledWith('[Process 1234 terminated by SIGTERM]');
    });

    it('should handle ack messages silently', async () => {
      await app.start();

      const consoleSpy = vi.spyOn(console, 'debug').mockImplementation(() => {});

      const messageHandler = app.getMessageHandler();
      messageHandler!({
        type: 'ack',
        message_id: 'msg-123',
      });

      expect(consoleSpy).toHaveBeenCalledWith('Server acknowledged request:', 'msg-123');

      consoleSpy.mockRestore();
    });

    it('should warn about unknown message types', async () => {
      await app.start();

      const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

      const messageHandler = app.getMessageHandler();
      messageHandler!({ type: 'unknown_type' });

      expect(consoleSpy).toHaveBeenCalledWith('Unknown message type:', 'unknown_type');

      consoleSpy.mockRestore();
    });
  });

  describe('Connection Status Handling', () => {
    it('should display connected status', async () => {
      await app.start();

      const statusHandler = app.getStatusHandler();
      statusHandler!('connected');

      expect(mockTerminal.writeInfo).toHaveBeenCalledWith('✓ Connected to server');
    });

    it('should display disconnected status', async () => {
      await app.start();

      const statusHandler = app.getStatusHandler();
      statusHandler!('disconnected');

      expect(mockTerminal.writeError).toHaveBeenCalledWith('✗ Disconnected from server');
    });

    it('should display reconnecting status', async () => {
      await app.start();

      const statusHandler = app.getStatusHandler();
      statusHandler!('reconnecting');

      expect(mockTerminal.writeInfo).toHaveBeenCalledWith('⟳ Reconnecting to server...');
    });

    it('should handle connection_status messages', async () => {
      await app.start();

      const messageHandler = app.getMessageHandler();
      messageHandler!({
        type: 'connection_status',
        status: 'connected',
        session_id: 'test-session-123',
      });

      expect(mockTerminal.writeInfo).toHaveBeenCalledWith('✓ Connected to server');
    });
  });

  describe('Session Management', () => {
    it('should create session on start', async () => {
      await app.start();

      expect(mockSessionManager.createSession).toHaveBeenCalled();
    });

    it('should log session ID on start', async () => {
      const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

      await app.start();

      expect(consoleSpy).toHaveBeenCalledWith('App started with session: test-session-123');

      consoleSpy.mockRestore();
    });
  });

  describe('Error Handling', () => {
    it('should handle WebSocket connection failure', async () => {
      mockWsClient.connect.mockRejectedValue(new Error('Network error'));

      await expect(app.start()).rejects.toThrow('Network error');

      expect(mockTerminal.writeError).toHaveBeenCalledWith(
        'Failed to start application: Network error'
      );
    });

    it('should handle session creation failure', async () => {
      mockSessionManager.createSession.mockRejectedValue(new Error('Session error'));

      await expect(app.start()).rejects.toThrow('Session error');
    });

    it('should handle stop errors gracefully', async () => {
      await app.start();

      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      mockTerminal.dispose.mockImplementation(() => {
        throw new Error('Dispose error');
      });

      // Should not throw
      await app.stop();

      expect(consoleSpy).toHaveBeenCalled();

      consoleSpy.mockRestore();
    });

    it('should handle non-Error exceptions', async () => {
      mockWsClient.connect.mockRejectedValue('String error');

      await expect(app.start()).rejects.toBe('String error');

      expect(mockTerminal.writeError).toHaveBeenCalledWith(
        'Failed to start application: String error'
      );
    });
  });

  describe('Module Coordination', () => {
    it('should coordinate all modules', async () => {
      await app.start();

      // Verify all modules were initialized and coordinated
      expect(mockWsClient.connect).toHaveBeenCalled();
      expect(mockTerminal.open).toHaveBeenCalled();
      expect(mockSessionManager.createSession).toHaveBeenCalled();
    });

    it('should route messages between modules', async () => {
      await app.start();

      // Terminal → WebSocket
      const dataCallback = mockTerminal.onData.mock.calls[0][0];
      dataCallback('echo test\n');
      expect(mockWsClient.send).toHaveBeenCalled();

      // WebSocket → Terminal
      const messageHandler = app.getMessageHandler();
      messageHandler!({ type: 'output', data: 'test\n' });
      expect(mockTerminal.write).toHaveBeenCalled();
    });

    it('should cleanup all modules on stop', async () => {
      await app.start();
      await app.stop();

      expect(mockTerminal.dispose).toHaveBeenCalled();
      expect(mockWsClient.disconnect).toHaveBeenCalled();
    });
  });

  describe('Edge Cases', () => {
    it('should handle rapid start/stop cycles', async () => {
      await app.start();
      await app.stop();
      await app.start();
      await app.stop();

      expect(mockWsClient.connect).toHaveBeenCalledTimes(2);
      expect(mockWsClient.disconnect).toHaveBeenCalledTimes(2);
    });

    it('should handle message before start', () => {
      const messageHandler = app.getMessageHandler();

      // Should not throw
      messageHandler!({ type: 'output', data: 'test\n' });

      expect(mockTerminal.write).toHaveBeenCalledWith('test\n');
    });

    it('should handle multiple messages in sequence', async () => {
      await app.start();

      const messageHandler = app.getMessageHandler();

      messageHandler!({ type: 'output', data: 'line1\n' });
      messageHandler!({ type: 'output', data: 'line2\n' });
      messageHandler!({ type: 'output', data: 'line3\n' });

      expect(mockTerminal.write).toHaveBeenCalledTimes(3);
    });
  });
});