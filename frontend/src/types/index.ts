/**
 * TypeScript type definitions for web-terminal
 * Per spec-kit/004-frontend-spec.md section 4
 * Per spec-kit/007-websocket-spec.md for WebSocket protocol types
 */

// ============================================================================
// Session Types
// ============================================================================

/**
 * Terminal session information
 */
export interface SessionInfo {
  id: string;
  created: number;
  lastActivity: number;
}

// ============================================================================
// WebSocket Protocol Types (per 007-websocket-spec.md)
// ============================================================================

/**
 * Signal types that can be sent to running processes
 */
export type Signal = 'SIGINT' | 'SIGTERM' | 'SIGKILL';

/**
 * Signal constants for convenience
 */
export const Signals = {
  SIGINT: 'SIGINT' as const,
  SIGTERM: 'SIGTERM' as const,
  SIGKILL: 'SIGKILL' as const,
};

/**
 * Connection status states
 */
export type ConnectionStatus = 'connected' | 'reconnecting' | 'disconnected';

/**
 * Output stream types
 */
export type OutputStream = 'stdout' | 'stderr';

/**
 * Flow control actions
 */
export type FlowControlAction = 'pause' | 'resume';

// ============================================================================
// Client Message Types (sent from browser to server)
// ============================================================================

/**
 * Execute a shell command
 */
export interface CommandMessage {
  type: 'command';
  data: string;
}

/**
 * Notify server of terminal size change
 */
export interface ResizeMessage {
  type: 'resize';
  cols: number;
  rows: number;
}

/**
 * Send signal to running process
 */
export interface SignalMessage {
  type: 'signal';
  signal: Signal;
}

/**
 * Initiate file upload
 */
export interface FileUploadStartMessage {
  type: 'file_upload_start';
  path: string;
  size: number;
  checksum: string;
}

/**
 * Signal upload completion
 */
export interface FileUploadCompleteMessage {
  type: 'file_upload_complete';
  chunk_count: number;
}

/**
 * Request file download
 */
export interface FileDownloadMessage {
  type: 'file_download';
  path: string;
}

/**
 * Set environment variable
 */
export interface EnvSetMessage {
  type: 'env_set';
  key: string;
  value: string;
}

/**
 * Change working directory
 */
export interface ChdirMessage {
  type: 'chdir';
  path: string;
}

/**
 * Echo test message
 */
export interface EchoMessage {
  type: 'echo';
  data: string;
}

/**
 * Ping for latency test
 */
export interface PingMessage {
  type: 'ping';
  timestamp: number;
}

/**
 * Acknowledgment message
 */
export interface AckMessage {
  type: 'ack';
  message_id?: string;
}

/**
 * Discriminated union of all client message types
 */
export type ClientMessage =
  | CommandMessage
  | ResizeMessage
  | SignalMessage
  | FileUploadStartMessage
  | FileUploadCompleteMessage
  | FileDownloadMessage
  | EnvSetMessage
  | ChdirMessage
  | EchoMessage
  | PingMessage
  | AckMessage;

// ============================================================================
// Server Message Types (sent from server to browser)
// ============================================================================

/**
 * Command output (stdout/stderr)
 */
export interface OutputMessage {
  type: 'output';
  stream: OutputStream;
  data: string;
}

/**
 * Error message
 */
export interface ErrorMessage {
  type: 'error';
  code: ErrorCode;
  message: string;
  details?: Record<string, unknown>;
}

/**
 * Process execution started
 */
export interface ProcessStartedMessage {
  type: 'process_started';
  pid: number;
  command: string;
}

/**
 * Process execution completed
 */
export interface ProcessExitedMessage {
  type: 'process_exited';
  pid: number;
  exit_code: number;
  signal: string | null;
}

/**
 * Connection state change
 */
export interface ConnectionStatusMessage {
  type: 'connection_status';
  status: ConnectionStatus;
  session_id: string;
}

/**
 * Working directory updated
 */
export interface CwdChangedMessage {
  type: 'cwd_changed';
  path: string;
}

/**
 * Environment variable updated
 */
export interface EnvUpdatedMessage {
  type: 'env_updated';
  key: string;
  value: string;
}

/**
 * File download beginning
 */
export interface FileDownloadStartMessage {
  type: 'file_download_start';
  path: string;
  size: number;
  checksum: string;
  chunk_size: number;
}

/**
 * File download finished
 */
export interface FileDownloadCompleteMessage {
  type: 'file_download_complete';
  chunk_count: number;
}

/**
 * Session resource usage update
 */
export interface ResourceUsageMessage {
  type: 'resource_usage';
  cpu_percent: number;
  memory_bytes: number;
  disk_bytes: number;
}

/**
 * Acknowledge client message
 */
export interface ServerAckMessage {
  type: 'ack';
  message_id?: string;
}

/**
 * Flow control message
 */
export interface FlowControlMessage {
  type: 'flow_control';
  action: FlowControlAction;
}

/**
 * Echo response
 */
export interface ServerEchoMessage {
  type: 'echo';
  data: string;
}

/**
 * Pong response
 */
export interface PongMessage {
  type: 'pong';
  timestamp: number;
  latency_ms: number;
}

/**
 * Discriminated union of all server message types
 */
export type ServerMessage =
  | OutputMessage
  | ErrorMessage
  | ProcessStartedMessage
  | ProcessExitedMessage
  | ConnectionStatusMessage
  | CwdChangedMessage
  | EnvUpdatedMessage
  | FileDownloadStartMessage
  | FileDownloadCompleteMessage
  | ResourceUsageMessage
  | ServerAckMessage
  | FlowControlMessage
  | ServerEchoMessage
  | PongMessage;

// ============================================================================
// Error Codes (per 007-websocket-spec.md)
// ============================================================================

/**
 * Error codes returned by the server
 */
export type ErrorCode =
  | 'COMMAND_NOT_FOUND'
  | 'COMMAND_FAILED'
  | 'COMMAND_TIMEOUT'
  | 'COMMAND_KILLED'
  | 'PERMISSION_DENIED'
  | 'PATH_NOT_FOUND'
  | 'PATH_INVALID'
  | 'SESSION_EXPIRED'
  | 'RESOURCE_LIMIT'
  | 'QUOTA_EXCEEDED'
  | 'INVALID_MESSAGE'
  | 'INTERNAL_ERROR';

/**
 * Error code constants for convenience
 */
export const ErrorCodes = {
  COMMAND_NOT_FOUND: 'COMMAND_NOT_FOUND' as const,
  COMMAND_FAILED: 'COMMAND_FAILED' as const,
  COMMAND_TIMEOUT: 'COMMAND_TIMEOUT' as const,
  COMMAND_KILLED: 'COMMAND_KILLED' as const,
  PERMISSION_DENIED: 'PERMISSION_DENIED' as const,
  PATH_NOT_FOUND: 'PATH_NOT_FOUND' as const,
  PATH_INVALID: 'PATH_INVALID' as const,
  SESSION_EXPIRED: 'SESSION_EXPIRED' as const,
  RESOURCE_LIMIT: 'RESOURCE_LIMIT' as const,
  QUOTA_EXCEEDED: 'QUOTA_EXCEEDED' as const,
  INVALID_MESSAGE: 'INVALID_MESSAGE' as const,
  INTERNAL_ERROR: 'INTERNAL_ERROR' as const,
};

// ============================================================================
// WebSocket Close Codes (per 007-websocket-spec.md)
// ============================================================================

/**
 * WebSocket close codes
 */
export type CloseCode = 1000 | 1001 | 1002 | 1003 | 1008 | 1011 | 4000 | 4001 | 4002;

/**
 * WebSocket close code constants for convenience
 */
export const CloseCodes = {
  NORMAL_CLOSURE: 1000 as const,
  GOING_AWAY: 1001 as const,
  PROTOCOL_ERROR: 1002 as const,
  UNSUPPORTED_DATA: 1003 as const,
  POLICY_VIOLATION: 1008 as const,
  INTERNAL_ERROR: 1011 as const,
  AUTHENTICATION_FAILED: 4000 as const,
  SESSION_EXPIRED: 4001 as const,
  RATE_LIMIT: 4002 as const,
};

// ============================================================================
// File Transfer Types
// ============================================================================

/**
 * File upload chunk (binary message format)
 * Format: [chunk_id: u32][data: bytes]
 */
export interface FileChunk {
  chunk_id: number;
  data: Uint8Array;
}

/**
 * File upload/download state
 */
export interface FileTransferState {
  path: string;
  size: number;
  checksum: string;
  chunk_size: number;
  chunks_received: number;
  chunks_total: number;
  in_progress: boolean;
}

// ============================================================================
// Type Guards
// ============================================================================

/**
 * Type guard for client messages
 */
export function isClientMessage(msg: unknown): msg is ClientMessage {
  if (typeof msg !== 'object' || msg === null) return false;
  const m = msg as Record<string, unknown>;
  return typeof m.type === 'string';
}

/**
 * Type guard for server messages
 */
export function isServerMessage(msg: unknown): msg is ServerMessage {
  if (typeof msg !== 'object' || msg === null) return false;
  const m = msg as Record<string, unknown>;
  return typeof m.type === 'string';
}

/**
 * Type guard for error messages
 */
export function isErrorMessage(msg: ServerMessage): msg is ErrorMessage {
  return msg.type === 'error';
}

/**
 * Type guard for output messages
 */
export function isOutputMessage(msg: ServerMessage): msg is OutputMessage {
  return msg.type === 'output';
}

/**
 * Type guard for process exited messages
 */
export function isProcessExitedMessage(msg: ServerMessage): msg is ProcessExitedMessage {
  return msg.type === 'process_exited';
}

// ============================================================================
// Configuration Types (per 004-frontend-spec.md)
// ============================================================================

/**
 * Terminal configuration
 */
export interface TerminalConfig {
  fontSize: number;
  fontFamily: string;
  theme: TerminalTheme;
}

/**
 * Terminal theme interface
 * All properties are required per xterm.js ITheme
 */
export interface TerminalTheme {
  background: string;
  foreground: string;
  cursor: string;
  selection: string;
  black: string;
  red: string;
  green: string;
  yellow: string;
  blue: string;
  magenta: string;
  cyan: string;
  white: string;
  brightBlack: string;
  brightRed: string;
  brightGreen: string;
  brightYellow: string;
  brightBlue: string;
  brightMagenta: string;
  brightCyan: string;
  brightWhite: string;
}

/**
 * WebSocket configuration
 */
export interface WebSocketConfig {
  token?: string;
  // Note: WebSocket URL constructed automatically from window.location
  // Single-port architecture: HTTP and WebSocket on same port
}

/**
 * Application configuration
 */
export interface AppConfig {
  websocket: WebSocketConfig;
  terminal: TerminalConfig;
}