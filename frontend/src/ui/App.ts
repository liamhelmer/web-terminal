/**
 * Application entry point - Initializes and coordinates all modules
 * Per spec-kit/004-frontend-spec.md section 3.4
 *
 * CRITICAL REQUIREMENTS:
 * 1. Coordinate Terminal, WebSocketClient, and SessionManager
 * 2. Setup bidirectional event handlers (terminal ↔ websocket)
 * 3. Handle terminal input → send to WebSocket
 * 4. Handle WebSocket messages → write to terminal
 * 5. Handle connection status changes
 * 6. Handle terminal resize → notify server
 * 7. Create and manage session on connection
 *
 * SINGLE-PORT ARCHITECTURE:
 * - All URLs are relative (no hardcoded hosts/ports)
 * - WebSocket URL constructed from window.location
 * - HTTP, WebSocket, and static assets all on same port
 */

import { Terminal } from '../terminal/Terminal';
import { WebSocketClient } from '../websocket/WebSocketClient';
import { SessionManager } from '../session/SessionManager';
import type {
  AppConfig,
  ServerMessage,
  ConnectionStatus,
} from '../types/index';

export class App {
  private terminal: Terminal;
  private wsClient: WebSocketClient;
  private sessionManager: SessionManager;
  private isStarted: boolean = false;

  constructor(container: HTMLElement, config: AppConfig) {
    // Initialize modules (storing container/config is not needed as modules handle them)

    // Initialize modules
    this.terminal = new Terminal(container, config.terminal);
    this.wsClient = new WebSocketClient(config.websocket);
    this.sessionManager = new SessionManager();

    // Setup event handlers
    this.setupEventHandlers();
  }

  /**
   * Start the application
   * 1. Connect to WebSocket server
   * 2. Initialize terminal UI
   * 3. Create terminal session
   */
  async start(): Promise<void> {
    if (this.isStarted) {
      console.warn('App already started');
      return;
    }

    try {
      // Connect to server (WebSocket URL constructed automatically from window.location)
      await this.wsClient.connect();

      // Initialize terminal UI
      this.terminal.open();

      // Request session from server
      const session = await this.sessionManager.createSession();
      console.log(`App started with session: ${session.id}`);

      this.isStarted = true;
    } catch (error) {
      console.error('Failed to start app:', error);
      this.terminal.writeError(
        `Failed to start application: ${error instanceof Error ? error.message : String(error)}`
      );
      throw error;
    }
  }

  /**
   * Stop the application
   * 1. Dispose terminal
   * 2. Disconnect WebSocket
   * 3. Clean up resources
   */
  async stop(): Promise<void> {
    if (!this.isStarted) {
      return;
    }

    try {
      // Dispose terminal first
      this.terminal.dispose();

      // Disconnect WebSocket
      await this.wsClient.disconnect();

      this.isStarted = false;
      console.log('App stopped');
    } catch (error) {
      console.error('Error stopping app:', error);
    }
  }

  /**
   * Setup bidirectional event handlers
   * Per spec-kit/004-frontend-spec.md section 3.4
   */
  private setupEventHandlers(): void {
    // Terminal input → WebSocket (send commands to server)
    this.terminal.onData((data) => {
      this.wsClient.send({
        type: 'command',
        data,
      });
    });

    // WebSocket messages → Terminal (receive server output)
    this.wsClient.onMessage((message) => {
      this.handleServerMessage(message);
    });

    // Connection status changes → Terminal (display status)
    this.wsClient.onConnectionChange((status) => {
      this.handleConnectionStatus(status);
    });

    // Terminal resize → WebSocket (notify server of size change)
    this.terminal.onResize((cols, rows) => {
      this.wsClient.send({
        type: 'resize',
        cols,
        rows,
      });
    });
  }

  /**
   * Handle messages from server
   * Per spec-kit/007-websocket-spec.md section "Server Messages"
   */
  private handleServerMessage(message: ServerMessage): void {
    switch (message.type) {
      case 'output':
        // Write stdout/stderr to terminal
        this.terminal.write(message.data);
        break;

      case 'error':
        // Display error message
        this.terminal.writeError(`Error: ${message.message}`);
        if (message.details) {
          console.error('Error details:', message.details);
        }
        break;

      case 'process_started':
        // Notify user that process started
        this.terminal.writeInfo(`[Process ${message.pid} started: ${message.command}]`);
        break;

      case 'process_exited':
        // Notify user that process exited
        const exitMsg = message.signal
          ? `[Process ${message.pid} terminated by ${message.signal}]`
          : `[Process ${message.pid} exited with code ${message.exit_code}]`;
        this.terminal.writeInfo(exitMsg);
        break;

      case 'connection_status':
        // Handle connection status from server
        this.handleConnectionStatus(message.status);
        // ConnectionStatusMessage includes session_id for logging
        console.debug('Connection status changed, session:', message.session_id);
        break;

      case 'ack':
        // Acknowledgment message (silent, just log)
        console.debug('Server acknowledged request:', message.message_id);
        break;

      default:
        // Unknown message type
        console.warn('Unknown message type:', (message as ServerMessage).type);
    }
  }

  /**
   * Handle connection status changes
   * Per spec-kit/004-frontend-spec.md section 3.4
   */
  private handleConnectionStatus(status: ConnectionStatus): void {
    switch (status) {
      case 'connected':
        this.terminal.writeInfo('✓ Connected to server');
        break;

      case 'disconnected':
        this.terminal.writeError('✗ Disconnected from server');
        break;

      case 'reconnecting':
        this.terminal.writeInfo('⟳ Reconnecting to server...');
        break;

      default:
        console.warn('Unknown connection status:', status);
    }
  }
}

// Export types
export type { AppConfig } from '../types/index';