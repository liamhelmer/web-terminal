/**
 * WebSocket client module - Manages WebSocket connection to backend
 * Per spec-kit/004-frontend-spec.md section 3.2
 * Per spec-kit/007-websocket-spec.md
 *
 * CRITICAL: Uses relative URLs, dynamically constructs from window.location
 * Single-port architecture: WebSocket uses same host and port as HTTP
 */

import type {
  ClientMessage,
  ServerMessage,
  ConnectionStatus,
  WebSocketConfig as BaseWebSocketConfig,
} from '../types/index';

/**
 * Extended WebSocket client configuration
 * Extends the base WebSocketConfig with connection behavior options
 */
export interface WebSocketClientConfig extends BaseWebSocketConfig {
  /** Maximum reconnection attempts (default: 10) */
  maxReconnectAttempts?: number;
  /** Initial reconnection delay in ms (default: 1000) */
  reconnectDelay?: number;
  /** Maximum queued messages when disconnected (default: 1000) */
  messageQueueLimit?: number;
}

/**
 * Message handler callback type
 */
type MessageHandler = (message: ServerMessage) => void;

/**
 * Connection status handler callback type
 */
type StatusHandler = (status: ConnectionStatus) => void;

/**
 * WebSocket client for terminal communication
 *
 * Features:
 * - Dynamic URL construction from window.location (single-port architecture)
 * - Automatic reconnection with exponential backoff
 * - Message queuing when disconnected
 * - Observer pattern for message and status handlers
 * - Full protocol support per 007-websocket-spec.md
 */
export class WebSocketClient {
  private ws: WebSocket | null = null;
  private reconnectAttempts = 0;
  private readonly maxReconnectAttempts: number;
  private readonly reconnectDelay: number;
  private readonly messageQueueLimit: number;
  private messageQueue: ClientMessage[] = [];
  private messageHandlers: Set<MessageHandler> = new Set();
  private statusHandlers: Set<StatusHandler> = new Set();
  private reconnectTimeoutId: number | null = null;
  private isManualDisconnect = false;
  private flowControlPaused = false;
  private config: WebSocketClientConfig;

  constructor(config: WebSocketClientConfig = {}) {
    this.config = config;
    this.maxReconnectAttempts = config.maxReconnectAttempts ?? 10;
    this.reconnectDelay = config.reconnectDelay ?? 1000;
    this.messageQueueLimit = config.messageQueueLimit ?? 1000;
  }

  /**
   * Connect to the WebSocket server
   * URL is dynamically constructed from window.location (single-port architecture)
   * @returns Promise that resolves when connected
   */
  async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      const url = this.buildUrl();

      console.log(`[WebSocket] Connecting to ${url}`);

      try {
        this.ws = new WebSocket(url);
      } catch (error) {
        console.error('[WebSocket] Failed to create WebSocket:', error);
        reject(error);
        return;
      }

      this.ws.onopen = () => {
        console.log('[WebSocket] Connected');
        this.reconnectAttempts = 0;
        this.isManualDisconnect = false;
        this.notifyStatus('connected');
        this.flushMessageQueue();
        resolve();
      };

      this.ws.onerror = (error) => {
        console.error('[WebSocket] Error:', error);
        // Don't reject here - let onclose handle it
        // This allows reconnection logic to work properly
      };

      this.ws.onclose = (event) => {
        console.log(`[WebSocket] Closed: code=${event.code}, reason="${event.reason}"`);
        this.ws = null;
        this.notifyStatus('disconnected');

        // Only reject on initial connection failure
        if (this.reconnectAttempts === 0) {
          reject(new Error(`WebSocket closed: ${event.reason || event.code}`));
        }

        this.handleDisconnection();
      };

      this.ws.onmessage = (event) => {
        this.handleMessage(event.data);
      };
    });
  }

  /**
   * Disconnect from the WebSocket server
   * Prevents automatic reconnection
   */
  async disconnect(): Promise<void> {
    this.isManualDisconnect = true;

    // Clear any pending reconnection
    if (this.reconnectTimeoutId !== null) {
      clearTimeout(this.reconnectTimeoutId);
      this.reconnectTimeoutId = null;
    }

    if (this.ws && this.ws.readyState !== WebSocket.CLOSED) {
      this.ws.close(1000, 'Client disconnect');
      this.ws = null;
    }
  }

  /**
   * Send a message to the server
   * If disconnected, message is queued for sending upon reconnection
   * @param message - Client message to send
   */
  send(message: ClientMessage): void {
    // Check flow control
    if (this.flowControlPaused) {
      console.warn('[WebSocket] Flow control paused, queuing message');
      this.queueMessage(message);
      return;
    }

    // Check connection state
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      try {
        this.ws.send(JSON.stringify(message));
      } catch (error) {
        console.error('[WebSocket] Failed to send message:', error);
        this.queueMessage(message);
      }
    } else {
      // Queue message for later
      this.queueMessage(message);
    }
  }

  /**
   * Register a message handler
   * @param handler - Callback to invoke when messages are received
   * @returns Unsubscribe function
   */
  onMessage(handler: MessageHandler): () => void {
    this.messageHandlers.add(handler);
    return () => this.messageHandlers.delete(handler);
  }

  /**
   * Register a connection status handler
   * @param handler - Callback to invoke when connection status changes
   * @returns Unsubscribe function
   */
  onConnectionChange(handler: StatusHandler): () => void {
    this.statusHandlers.add(handler);
    return () => this.statusHandlers.delete(handler);
  }

  /**
   * Get current connection status
   */
  getConnectionStatus(): ConnectionStatus {
    if (this.ws?.readyState === WebSocket.OPEN) {
      return 'connected';
    } else if (this.reconnectTimeoutId !== null) {
      return 'reconnecting';
    } else {
      return 'disconnected';
    }
  }

  /**
   * Get number of queued messages
   */
  getQueuedMessageCount(): number {
    return this.messageQueue.length;
  }

  /**
   * Build WebSocket URL dynamically from window.location
   * CRITICAL: Single-port architecture - no hardcoded URLs
   *
   * Protocol: ws:// for http://, wss:// for https://
   * Host: Same as current page (includes port)
   * Path: /ws
   * Query: ?token=<jwt> if token provided
   */
  private buildUrl(): string {
    // Detect protocol based on page protocol
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';

    // Use same host as current page (includes hostname and port)
    const host = window.location.host;

    // Construct base URL
    let url = `${protocol}//${host}/ws`;

    // Add token if provided
    if (this.config.token) {
      url += `?token=${encodeURIComponent(this.config.token)}`;
    }

    return url;
  }

  /**
   * Handle incoming WebSocket message
   */
  private handleMessage(data: string): void {
    try {
      const message: ServerMessage = JSON.parse(data);

      // Handle flow control messages
      if (message.type === 'flow_control') {
        this.handleFlowControl(message.action);
        return;
      }

      // Notify all message handlers
      this.notifyMessage(message);
    } catch (error) {
      console.error('[WebSocket] Failed to parse message:', error, data);
    }
  }

  /**
   * Handle flow control messages from server
   */
  private handleFlowControl(action: 'pause' | 'resume'): void {
    if (action === 'pause') {
      console.log('[WebSocket] Flow control: PAUSE');
      this.flowControlPaused = true;
    } else if (action === 'resume') {
      console.log('[WebSocket] Flow control: RESUME');
      this.flowControlPaused = false;
      this.flushMessageQueue();
    }
  }

  /**
   * Handle WebSocket disconnection
   * Implements exponential backoff reconnection strategy
   */
  private handleDisconnection(): void {
    // Don't reconnect if manual disconnect
    if (this.isManualDisconnect) {
      console.log('[WebSocket] Manual disconnect, not reconnecting');
      return;
    }

    // Check if max attempts reached
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error(`[WebSocket] Max reconnection attempts (${this.maxReconnectAttempts}) reached`);
      this.notifyStatus('disconnected');
      return;
    }

    // Calculate delay with exponential backoff
    const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts);
    this.reconnectAttempts++;

    console.log(`[WebSocket] Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})`);
    this.notifyStatus('reconnecting');

    // Schedule reconnection
    this.reconnectTimeoutId = window.setTimeout(() => {
      this.reconnectTimeoutId = null;
      console.log(`[WebSocket] Attempting reconnection (${this.reconnectAttempts}/${this.maxReconnectAttempts})`);

      this.connect().catch((error) => {
        console.error('[WebSocket] Reconnection failed:', error);
        // handleDisconnection will be called again via onclose
      });
    }, delay);
  }

  /**
   * Queue a message for later sending
   * Implements message queue size limit and message expiration
   */
  private queueMessage(message: ClientMessage): void {
    // Check queue size limit
    if (this.messageQueue.length >= this.messageQueueLimit) {
      console.warn('[WebSocket] Message queue full, dropping oldest message');
      this.messageQueue.shift(); // Remove oldest message
    }

    this.messageQueue.push(message);
    console.log(`[WebSocket] Message queued (${this.messageQueue.length} in queue)`);
  }

  /**
   * Flush queued messages to the server
   * Called when connection is re-established
   */
  private flushMessageQueue(): void {
    if (this.messageQueue.length === 0) {
      return;
    }

    console.log(`[WebSocket] Flushing ${this.messageQueue.length} queued messages`);

    // Send all queued messages
    while (this.messageQueue.length > 0) {
      const message = this.messageQueue.shift()!;
      this.send(message);
    }
  }

  /**
   * Notify all message handlers of a new message
   */
  private notifyMessage(message: ServerMessage): void {
    this.messageHandlers.forEach((handler) => {
      try {
        handler(message);
      } catch (error) {
        console.error('[WebSocket] Message handler error:', error);
      }
    });
  }

  /**
   * Notify all status handlers of a status change
   */
  private notifyStatus(status: ConnectionStatus): void {
    this.statusHandlers.forEach((handler) => {
      try {
        handler(status);
      } catch (error) {
        console.error('[WebSocket] Status handler error:', error);
      }
    });
  }
}