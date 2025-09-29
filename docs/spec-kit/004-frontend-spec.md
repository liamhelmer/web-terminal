# Web-Terminal: Frontend Specification

**Version:** 1.0.0
**Status:** Draft
**Author:** Liam Helmer
**Last Updated:** 2025-09-29
**References:** [002-architecture.md](./002-architecture.md)

---

## Table of Contents

1. [Technology Stack](#technology-stack)
2. [Project Structure](#project-structure)
3. [Component Architecture](#component-architecture)
4. [State Management](#state-management)
5. [WebSocket Client](#websocket-client)
6. [Terminal Integration](#terminal-integration)
7. [Build and Deployment](#build-and-deployment)

---

## Technology Stack

| Technology | Version | Purpose |
|-----------|---------|---------|
| TypeScript | 5.x | Type-safe JavaScript |
| xterm.js | 5.x | Terminal emulator |
| Vite | 5.x | Build tool and dev server |
| pnpm | 8.x | Package manager |
| Vitest | 1.x | Unit testing |
| Playwright | 1.x | E2E testing |

---

## Project Structure

```
frontend/
├── src/
│   ├── main.ts                 # Application entry point
│   ├── app.ts                  # Main application class
│   ├── terminal/
│   │   ├── terminal.ts         # Terminal manager
│   │   ├── renderer.ts         # Custom rendering logic
│   │   └── addons.ts           # xterm.js addons
│   ├── websocket/
│   │   ├── client.ts           # WebSocket client
│   │   ├── protocol.ts         # Message protocol
│   │   └── reconnect.ts        # Reconnection logic
│   ├── session/
│   │   ├── manager.ts          # Session management
│   │   └── state.ts            # Session state
│   ├── ui/
│   │   ├── components/         # UI components
│   │   ├── styles/             # CSS styles
│   │   └── theme.ts            # Theme configuration
│   ├── utils/
│   │   ├── logger.ts           # Logging utility
│   │   └── storage.ts          # Local storage wrapper
│   └── types/
│       ├── messages.ts         # TypeScript types for messages
│       └── session.ts          # Session types
├── public/
│   ├── index.html              # HTML template
│   └── favicon.ico
├── tests/
│   ├── unit/                   # Unit tests
│   └── e2e/                    # E2E tests
├── package.json
├── tsconfig.json
├── vite.config.ts
└── vitest.config.ts
```

---

## Component Architecture

### 1. Application Class (src/app.ts)

```typescript
import { Terminal } from './terminal/terminal';
import { WebSocketClient } from './websocket/client';
import { SessionManager } from './session/manager';

export class App {
  private terminal: Terminal;
  private wsClient: WebSocketClient;
  private sessionManager: SessionManager;

  constructor(private container: HTMLElement, private config: AppConfig) {
    this.terminal = new Terminal(container, config.terminal);
    this.wsClient = new WebSocketClient(config.websocket);
    this.sessionManager = new SessionManager();

    this.setupEventHandlers();
  }

  async start(): Promise<void> {
    // Connect to server
    await this.wsClient.connect();

    // Initialize terminal
    this.terminal.open();

    // Request session
    const session = await this.sessionManager.createSession();

    console.log(`App started with session: ${session.id}`);
  }

  private setupEventHandlers(): void {
    // Terminal input -> WebSocket
    this.terminal.onData((data) => {
      this.wsClient.send({
        type: 'command',
        data,
      });
    });

    // WebSocket output -> Terminal
    this.wsClient.onMessage((message) => {
      this.handleServerMessage(message);
    });

    // Connection status
    this.wsClient.onConnectionChange((status) => {
      this.handleConnectionStatus(status);
    });

    // Terminal resize
    this.terminal.onResize((cols, rows) => {
      this.wsClient.send({
        type: 'resize',
        cols,
        rows,
      });
    });
  }

  private handleServerMessage(message: ServerMessage): void {
    switch (message.type) {
      case 'output':
        this.terminal.write(message.data);
        break;
      case 'error':
        this.terminal.writeError(message.message);
        break;
      case 'process_exited':
        this.terminal.writeInfo(`Process exited with code ${message.exit_code}`);
        break;
    }
  }

  private handleConnectionStatus(status: ConnectionStatus): void {
    switch (status) {
      case 'connected':
        this.terminal.writeInfo('Connected to server');
        break;
      case 'disconnected':
        this.terminal.writeError('Disconnected from server');
        break;
      case 'reconnecting':
        this.terminal.writeInfo('Reconnecting...');
        break;
    }
  }

  async stop(): Promise<void> {
    this.terminal.dispose();
    await this.wsClient.disconnect();
  }
}

export interface AppConfig {
  websocket: {
    token?: string;
    // Note: WebSocket URL constructed automatically from window.location
    // Single-port architecture: HTTP and WebSocket on same port
  };
  terminal: {
    fontSize: number;
    fontFamily: string;
    theme: TerminalTheme;
  };
}
```

---

### 2. Terminal Manager (src/terminal/terminal.ts)

```typescript
import { Terminal as XTerm } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import { WebLinksAddon } from 'xterm-addon-web-links';
import { SearchAddon } from 'xterm-addon-search';

export class Terminal {
  private xterm: XTerm;
  private fitAddon: FitAddon;
  private searchAddon: SearchAddon;
  private container: HTMLElement;

  constructor(container: HTMLElement, config: TerminalConfig) {
    this.container = container;

    // Create xterm instance
    this.xterm = new XTerm({
      fontSize: config.fontSize,
      fontFamily: config.fontFamily,
      theme: config.theme,
      cursorBlink: true,
      cursorStyle: 'block',
      scrollback: 10000,
      allowTransparency: false,
    });

    // Setup addons
    this.fitAddon = new FitAddon();
    this.searchAddon = new SearchAddon();

    this.xterm.loadAddon(this.fitAddon);
    this.xterm.loadAddon(this.searchAddon);
    this.xterm.loadAddon(new WebLinksAddon());

    this.setupEventHandlers();
  }

  open(): void {
    this.xterm.open(this.container);
    this.fit();

    // Handle window resize
    window.addEventListener('resize', () => this.fit());
  }

  fit(): void {
    this.fitAddon.fit();
  }

  write(data: string): void {
    this.xterm.write(data);
  }

  writeError(message: string): void {
    this.xterm.write(`\x1b[31m${message}\x1b[0m\r\n`);
  }

  writeInfo(message: string): void {
    this.xterm.write(`\x1b[34m${message}\x1b[0m\r\n`);
  }

  clear(): void {
    this.xterm.clear();
  }

  onData(callback: (data: string) => void): void {
    this.xterm.onData(callback);
  }

  onResize(callback: (cols: number, rows: number) => void): void {
    this.xterm.onResize(({ cols, rows }) => {
      callback(cols, rows);
    });
  }

  search(term: string, forward: boolean = true): boolean {
    return forward
      ? this.searchAddon.findNext(term)
      : this.searchAddon.findPrevious(term);
  }

  dispose(): void {
    this.xterm.dispose();
  }

  private setupEventHandlers(): void {
    // Keyboard shortcuts
    this.xterm.attachCustomKeyEventHandler((event) => {
      // Ctrl+C
      if (event.ctrlKey && event.key === 'c') {
        return true; // Allow default (copy or interrupt)
      }

      // Ctrl+V
      if (event.ctrlKey && event.key === 'v') {
        this.handlePaste();
        return false;
      }

      // Ctrl+F
      if (event.ctrlKey && event.key === 'f') {
        this.showSearchDialog();
        return false;
      }

      return true;
    });
  }

  private async handlePaste(): Promise<void> {
    try {
      const text = await navigator.clipboard.readText();
      this.xterm.paste(text);
    } catch (error) {
      console.error('Failed to paste:', error);
    }
  }

  private showSearchDialog(): void {
    // Implement search dialog
    const term = prompt('Search:');
    if (term) {
      this.search(term);
    }
  }
}

export interface TerminalConfig {
  fontSize: number;
  fontFamily: string;
  theme: TerminalTheme;
}

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
```

---

### 3. WebSocket Client (src/websocket/client.ts)

```typescript
export class WebSocketClient {
  private ws: WebSocket | null = null;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 10;
  private reconnectDelay = 1000;
  private messageQueue: ClientMessage[] = [];
  private messageHandlers: Set<MessageHandler> = new Set();
  private statusHandlers: Set<StatusHandler> = new Set();

  constructor(private config: WebSocketConfig) {}

  async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      const url = this.buildUrl();
      this.ws = new WebSocket(url);

      this.ws.onopen = () => {
        console.log('WebSocket connected');
        this.reconnectAttempts = 0;
        this.notifyStatus('connected');
        this.flushMessageQueue();
        resolve();
      };

      this.ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        reject(error);
      };

      this.ws.onclose = (event) => {
        console.log('WebSocket closed:', event.code, event.reason);
        this.notifyStatus('disconnected');
        this.handleDisconnection();
      };

      this.ws.onmessage = (event) => {
        this.handleMessage(event.data);
      };
    });
  }

  async disconnect(): Promise<void> {
    if (this.ws) {
      this.ws.close(1000, 'Client disconnect');
      this.ws = null;
    }
  }

  send(message: ClientMessage): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    } else {
      // Queue message for later
      this.messageQueue.push(message);
    }
  }

  onMessage(handler: MessageHandler): () => void {
    this.messageHandlers.add(handler);
    return () => this.messageHandlers.delete(handler);
  }

  onConnectionChange(handler: StatusHandler): () => void {
    this.statusHandlers.add(handler);
    return () => this.statusHandlers.delete(handler);
  }

  private buildUrl(): string {
    // Dynamically construct WebSocket URL from current page location
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host; // includes hostname and port
    let url = `${protocol}//${host}/ws`;

    if (this.config.token) {
      url += `?token=${this.config.token}`;
    }

    return url;
  }

  private handleMessage(data: string): void {
    try {
      const message: ServerMessage = JSON.parse(data);
      this.notifyMessage(message);
    } catch (error) {
      console.error('Failed to parse message:', error);
    }
  }

  private handleDisconnection(): void {
    if (this.reconnectAttempts < this.maxReconnectAttempts) {
      this.notifyStatus('reconnecting');

      const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts);
      this.reconnectAttempts++;

      setTimeout(() => {
        console.log(`Reconnecting (attempt ${this.reconnectAttempts})...`);
        this.connect().catch((error) => {
          console.error('Reconnection failed:', error);
        });
      }, delay);
    } else {
      console.error('Max reconnection attempts reached');
    }
  }

  private flushMessageQueue(): void {
    while (this.messageQueue.length > 0) {
      const message = this.messageQueue.shift()!;
      this.send(message);
    }
  }

  private notifyMessage(message: ServerMessage): void {
    this.messageHandlers.forEach((handler) => {
      try {
        handler(message);
      } catch (error) {
        console.error('Message handler error:', error);
      }
    });
  }

  private notifyStatus(status: ConnectionStatus): void {
    this.statusHandlers.forEach((handler) => {
      try {
        handler(status);
      } catch (error) {
        console.error('Status handler error:', error);
      }
    });
  }
}

export interface WebSocketConfig {
  token?: string;
  // Note: URL is automatically constructed from window.location
  // No hardcoded URLs needed for single-port architecture
}

type MessageHandler = (message: ServerMessage) => void;
type StatusHandler = (status: ConnectionStatus) => void;

export type ConnectionStatus = 'connected' | 'disconnected' | 'reconnecting';
```

---

### 4. Protocol Types (src/types/messages.ts)

```typescript
export type ClientMessage =
  | { type: 'command'; data: string }
  | { type: 'resize'; cols: number; rows: number }
  | { type: 'signal'; signal: Signal };

export type ServerMessage =
  | { type: 'output'; data: string }
  | { type: 'error'; message: string }
  | { type: 'process_exited'; exit_code: number }
  | { type: 'connection_status'; status: ConnectionStatus };

export enum Signal {
  SIGINT = 2,
  SIGTERM = 15,
  SIGKILL = 9,
}

export type ConnectionStatus = 'connected' | 'disconnected' | 'reconnecting';
```

---

### 5. Session Manager (src/session/manager.ts)

```typescript
export class SessionManager {
  private sessions: Map<string, Session> = new Map();
  private currentSession: Session | null = null;

  async createSession(): Promise<Session> {
    const session: Session = {
      id: this.generateSessionId(),
      createdAt: new Date(),
      state: {
        workingDir: '/workspace',
        environment: {},
        history: [],
      },
    };

    this.sessions.set(session.id, session);
    this.currentSession = session;

    // Save to local storage
    this.saveToStorage();

    return session;
  }

  getSession(id: string): Session | undefined {
    return this.sessions.get(id);
  }

  getCurrentSession(): Session | null {
    return this.currentSession;
  }

  switchSession(id: string): boolean {
    const session = this.sessions.get(id);
    if (session) {
      this.currentSession = session;
      return true;
    }
    return false;
  }

  deleteSession(id: string): void {
    this.sessions.delete(id);
    if (this.currentSession?.id === id) {
      this.currentSession = null;
    }
    this.saveToStorage();
  }

  listSessions(): Session[] {
    return Array.from(this.sessions.values());
  }

  private generateSessionId(): string {
    return `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
  }

  private saveToStorage(): void {
    const data = {
      sessions: Array.from(this.sessions.entries()),
      currentSessionId: this.currentSession?.id,
    };
    localStorage.setItem('terminal-sessions', JSON.stringify(data));
  }

  loadFromStorage(): void {
    const data = localStorage.getItem('terminal-sessions');
    if (data) {
      try {
        const parsed = JSON.parse(data);
        this.sessions = new Map(parsed.sessions);
        if (parsed.currentSessionId) {
          this.currentSession = this.sessions.get(parsed.currentSessionId) ?? null;
        }
      } catch (error) {
        console.error('Failed to load sessions from storage:', error);
      }
    }
  }
}

export interface Session {
  id: string;
  createdAt: Date;
  state: SessionState;
}

export interface SessionState {
  workingDir: string;
  environment: Record<string, string>;
  history: string[];
}
```

---

## State Management

### Local Storage Strategy

```typescript
// src/utils/storage.ts

export class Storage {
  static set(key: string, value: any): void {
    try {
      localStorage.setItem(key, JSON.stringify(value));
    } catch (error) {
      console.error('Failed to save to storage:', error);
    }
  }

  static get<T>(key: string): T | null {
    try {
      const item = localStorage.getItem(key);
      return item ? JSON.parse(item) : null;
    } catch (error) {
      console.error('Failed to load from storage:', error);
      return null;
    }
  }

  static remove(key: string): void {
    localStorage.removeItem(key);
  }

  static clear(): void {
    localStorage.clear();
  }
}
```

---

## Build and Deployment

### Vite Configuration (vite.config.ts)

```typescript
import { defineConfig } from 'vite';

export default defineConfig({
  build: {
    target: 'es2020',
    outDir: 'dist',
    sourcemap: true,
    minify: 'terser',
  },
  // Note: Development server configuration
  // In production, frontend is served by backend on same port (single-port architecture)
  server: {
    port: 8080,  // Use same port as backend for consistency
    proxy: {
      '/ws': {
        target: 'ws://localhost:8080',
        ws: true,
        changeOrigin: true,
      },
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true,
      },
    },
  },
});
```

**Production Note:** In production deployment, Vite's dev server is NOT used. The compiled frontend assets are served directly by the Rust backend on a single port (default 8080).

### Package.json Scripts

```json
{
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "test": "vitest",
    "test:e2e": "playwright test",
    "lint": "eslint src --ext .ts",
    "format": "prettier --write 'src/**/*.ts'"
  }
}
```

---

## Version History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0 | 2025-09-29 | Liam Helmer | Initial frontend specification |