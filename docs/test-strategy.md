# Web Terminal E2E Test Strategy

## Overview

This document defines the comprehensive Playwright end-to-end testing strategy for the web-terminal application. The goal is to achieve >90% coverage for critical paths while ensuring robust, maintainable, and fast test execution.

## Test Organization Structure

### Directory Layout

```
tests/
├── e2e/
│   ├── fixtures/
│   │   ├── terminal.fixture.ts       # Terminal instance management
│   │   ├── server.fixture.ts         # Backend server lifecycle
│   │   ├── websocket.fixture.ts      # WebSocket utilities
│   │   └── proxy.fixture.ts          # Reverse proxy setup
│   ├── terminal/
│   │   ├── basic-io.spec.ts          # Input/output tests
│   │   ├── ansi-colors.spec.ts       # Color rendering tests
│   │   ├── ansi-escape.spec.ts       # Escape sequence tests
│   │   ├── unicode.spec.ts           # Unicode/emoji tests
│   │   ├── scrollback.spec.ts        # Scrollback functionality
│   │   └── resize.spec.ts            # Terminal resizing
│   ├── multi-terminal/
│   │   ├── creation.spec.ts          # Terminal creation (API + UI)
│   │   ├── switching.spec.ts         # Terminal switching
│   │   ├── state-isolation.spec.ts   # Independent state per terminal
│   │   └── cleanup.spec.ts           # Terminal closure and cleanup
│   ├── cli-arguments/
│   │   ├── default-command.spec.ts   # Default command execution
│   │   ├── custom-command.spec.ts    # Custom command via --cmd
│   │   ├── argument-parsing.spec.ts  # Complex argument handling
│   │   └── env-config.spec.ts        # Environment variable config
│   ├── process-lifecycle/
│   │   ├── startup.spec.ts           # Backend startup
│   │   ├── exit-handling.spec.ts     # Exit code propagation
│   │   ├── stderr-logging.spec.ts    # Error stream handling
│   │   └── cleanup.spec.ts           # Subprocess cleanup
│   ├── websocket/
│   │   ├── connection.spec.ts        # Connection establishment
│   │   ├── reconnection.spec.ts      # Reconnection logic
│   │   ├── message-handling.spec.ts  # Binary vs text messages
│   │   ├── large-payloads.spec.ts    # Large data handling
│   │   └── concurrent.spec.ts        # Multiple connections
│   ├── proxy/
│   │   ├── path-prefix.spec.ts       # Reverse proxy with prefix
│   │   ├── websocket-upgrade.spec.ts # WS through proxy
│   │   └── relative-paths.spec.ts    # Path resolution
│   └── api/
│       ├── config.spec.ts            # GET /api/config
│       ├── sessions-list.spec.ts     # GET /api/sessions
│       ├── sessions-create.spec.ts   # POST /api/sessions
│       ├── sessions-delete.spec.ts   # DELETE /api/sessions/:id
│       └── error-handling.spec.ts    # Error responses
├── helpers/
│   ├── terminal-utils.ts             # Terminal interaction helpers
│   ├── ansi-parser.ts                # ANSI code parsing utilities
│   ├── websocket-client.ts           # WebSocket test client
│   └── process-helpers.ts            # Process management utilities
├── data/
│   ├── ansi-test-sequences.json      # ANSI escape sequences
│   ├── unicode-test-data.json        # Unicode test strings
│   └── command-test-cases.json       # CLI argument test cases
└── playwright.config.ts              # Playwright configuration
```

## Fixture Requirements

### 1. Terminal Fixture (`terminal.fixture.ts`)

**Purpose:** Manage terminal instance lifecycle and provide helper methods.

```typescript
interface TerminalFixture {
  // Terminal instance access
  page: Page;
  terminalElement: Locator;

  // Helper methods
  typeCommand(command: string): Promise<void>;
  waitForOutput(text: string, timeout?: number): Promise<void>;
  getOutputText(): Promise<string>;
  getOutputHtml(): Promise<string>;
  clearTerminal(): Promise<void>;

  // ANSI helpers
  getColorAtPosition(row: number, col: number): Promise<string>;
  getCursorPosition(): Promise<{ row: number; col: number }>;

  // Scrollback helpers
  scrollToTop(): Promise<void>;
  scrollToBottom(): Promise<void>;
  getScrollPosition(): Promise<number>;
  getTotalLines(): Promise<number>;
}
```

### 2. Server Fixture (`server.fixture.ts`)

**Purpose:** Manage backend server lifecycle for tests.

```typescript
interface ServerFixture {
  // Server control
  start(options?: ServerOptions): Promise<void>;
  stop(): Promise<void>;
  restart(): Promise<void>;

  // Server info
  url: string;
  wsUrl: string;
  port: number;

  // Process info
  getPid(): number;
  getExitCode(): Promise<number | null>;
  isRunning(): boolean;

  // Log access
  getStdout(): string[];
  getStderr(): string[];
  waitForLog(pattern: RegExp, timeout?: number): Promise<string>;
}

interface ServerOptions {
  command?: string;
  args?: string[];
  env?: Record<string, string>;
  port?: number;
  cols?: number;
  rows?: number;
}
```

### 3. WebSocket Fixture (`websocket.fixture.ts`)

**Purpose:** Provide WebSocket testing utilities.

```typescript
interface WebSocketFixture {
  // Connection management
  connect(url: string): Promise<void>;
  disconnect(): Promise<void>;
  reconnect(): Promise<void>;

  // Message handling
  send(data: string | Buffer): Promise<void>;
  waitForMessage(timeout?: number): Promise<MessageEvent>;
  getMessages(): MessageEvent[];
  clearMessages(): void;

  // State
  isConnected(): boolean;
  getReadyState(): number;

  // Events
  onMessage(callback: (data: any) => void): void;
  onError(callback: (error: Error) => void): void;
  onClose(callback: (code: number, reason: string) => void): void;
}
```

### 4. Proxy Fixture (`proxy.fixture.ts`)

**Purpose:** Set up reverse proxy for testing.

```typescript
interface ProxyFixture {
  // Proxy control
  start(config: ProxyConfig): Promise<void>;
  stop(): Promise<void>;

  // Proxy info
  proxyUrl: string;
  targetUrl: string;
  pathPrefix: string;

  // Configuration
  setPathPrefix(prefix: string): void;
  setHeaders(headers: Record<string, string>): void;
}

interface ProxyConfig {
  targetPort: number;
  proxyPort: number;
  pathPrefix?: string;
  headers?: Record<string, string>;
}
```

## Test Data Needs

### 1. ANSI Test Sequences (`ansi-test-sequences.json`)

```json
{
  "colors": {
    "16-color": [
      { "code": "\x1b[31m", "name": "red", "rgb": [255, 0, 0] },
      { "code": "\x1b[32m", "name": "green", "rgb": [0, 255, 0] }
    ],
    "256-color": [
      { "code": "\x1b[38;5;196m", "name": "red-256", "rgb": [255, 0, 0] }
    ],
    "true-color": [
      { "code": "\x1b[38;2;255;0;0m", "name": "red-true", "rgb": [255, 0, 0] }
    ]
  },
  "cursor-movement": [
    { "code": "\x1b[A", "name": "cursor-up", "movement": "up" },
    { "code": "\x1b[B", "name": "cursor-down", "movement": "down" },
    { "code": "\x1b[H", "name": "cursor-home", "movement": "home" }
  ],
  "screen-control": [
    { "code": "\x1b[2J", "name": "clear-screen" },
    { "code": "\x1b[K", "name": "clear-line" }
  ]
}
```

### 2. Unicode Test Data (`unicode-test-data.json`)

```json
{
  "emoji": [
    { "char": "😀", "name": "grinning-face", "codepoint": "U+1F600" },
    { "char": "🚀", "name": "rocket", "codepoint": "U+1F680" }
  ],
  "international": [
    { "text": "こんにちは", "language": "Japanese" },
    { "text": "مرحبا", "language": "Arabic" },
    { "text": "Привет", "language": "Russian" }
  ],
  "combining": [
    { "text": "é", "description": "e + combining acute" },
    { "text": "ñ", "description": "n + combining tilde" }
  ],
  "zero-width": [
    { "char": "\u200B", "name": "zero-width-space" },
    { "char": "\u200C", "name": "zero-width-non-joiner" }
  ]
}
```

### 3. CLI Test Cases (`command-test-cases.json`)

```json
{
  "simple-commands": [
    { "cmd": "echo hello", "expected": "hello" },
    { "cmd": "pwd", "expected_pattern": "^/.*" }
  ],
  "quoted-arguments": [
    { "cmd": "echo \"hello world\"", "expected": "hello world" },
    { "cmd": "echo 'single quotes'", "expected": "single quotes" }
  ],
  "special-characters": [
    { "cmd": "echo $PATH", "expected_pattern": ".*:.*" },
    { "cmd": "echo \\$escaped", "expected": "$escaped" }
  ],
  "edge-cases": [
    { "cmd": "", "expected": "" },
    { "cmd": "   ", "expected": "" }
  ]
}
```

## Playwright Configuration

### Base Configuration (`playwright.config.ts`)

```typescript
import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests/e2e',

  // Test execution
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,

  // Timeouts
  timeout: 30000,
  expect: {
    timeout: 5000,
  },

  // Reporters
  reporter: [
    ['html', { outputFolder: 'test-results/html' }],
    ['json', { outputFile: 'test-results/results.json' }],
    ['junit', { outputFile: 'test-results/junit.xml' }],
    ['list'],
  ],

  // Global setup/teardown
  globalSetup: require.resolve('./tests/e2e/global-setup.ts'),
  globalTeardown: require.resolve('./tests/e2e/global-teardown.ts'),

  use: {
    // Base URL
    baseURL: 'http://localhost:3000',

    // Tracing
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',

    // Browser context
    viewport: { width: 1280, height: 720 },
    ignoreHTTPSErrors: true,
  },

  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'firefox',
      use: { ...devices['Desktop Firefox'] },
    },
    {
      name: 'webkit',
      use: { ...devices['Desktop Safari'] },
    },
    {
      name: 'mobile-chrome',
      use: { ...devices['Pixel 5'] },
    },
    {
      name: 'mobile-safari',
      use: { ...devices['iPhone 12'] },
    },
  ],

  // Web server
  webServer: {
    command: 'npm run start:test',
    port: 3000,
    timeout: 120000,
    reuseExistingServer: !process.env.CI,
  },
});
```

### Test-Specific Configurations

#### Terminal Tests
```typescript
// Terminal tests may need larger viewport
use: {
  viewport: { width: 1920, height: 1080 },
  deviceScaleFactor: 1,
}
```

#### WebSocket Tests
```typescript
// WebSocket tests need longer timeouts
timeout: 60000,
expect: {
  timeout: 10000,
}
```

#### Proxy Tests
```typescript
// Proxy tests run against different base URL
use: {
  baseURL: 'http://localhost:8080/terminal',
}
```

## Test Implementation Patterns

### 1. Basic Terminal I/O Test

```typescript
test('should echo typed input', async ({ terminal }) => {
  // Arrange
  const input = 'hello world';

  // Act
  await terminal.typeCommand(input);
  await terminal.page.keyboard.press('Enter');

  // Assert
  await terminal.waitForOutput(input);
  const output = await terminal.getOutputText();
  expect(output).toContain(input);
});
```

### 2. ANSI Color Test

```typescript
test('should render 16-color ANSI codes', async ({ terminal }) => {
  // Arrange
  const redText = '\x1b[31mRED TEXT\x1b[0m';

  // Act
  await terminal.typeCommand(`echo -e "${redText}"`);
  await terminal.page.keyboard.press('Enter');

  // Assert
  await terminal.waitForOutput('RED TEXT');
  const color = await terminal.getColorAtPosition(1, 0);
  expect(color).toBe('rgb(255, 0, 0)');
});
```

### 3. Multi-Terminal Test

```typescript
test('should maintain independent state per terminal', async ({ page }) => {
  // Create two terminals
  const terminal1 = await createTerminal(page);
  const terminal2 = await createTerminal(page);

  // Type different commands
  await terminal1.typeCommand('cd /tmp');
  await terminal2.typeCommand('cd /home');

  // Verify independent state
  await terminal1.typeCommand('pwd');
  expect(await terminal1.getOutputText()).toContain('/tmp');

  await terminal2.typeCommand('pwd');
  expect(await terminal2.getOutputText()).toContain('/home');
});
```

### 4. CLI Argument Test

```typescript
test('should execute custom command from CLI', async ({ server }) => {
  // Start server with custom command
  await server.start({
    command: 'echo',
    args: ['hello', 'from', 'cli'],
  });

  // Wait for output
  await server.waitForLog(/hello from cli/);

  // Verify stdout
  const stdout = server.getStdout();
  expect(stdout.join('\n')).toContain('hello from cli');
});
```

### 5. WebSocket Reconnection Test

```typescript
test('should reconnect on disconnect', async ({ websocket, server }) => {
  // Connect
  await websocket.connect(server.wsUrl);
  expect(websocket.isConnected()).toBe(true);

  // Disconnect
  await server.stop();
  await expect.poll(() => websocket.isConnected()).toBe(false);

  // Restart and reconnect
  await server.start();
  await expect.poll(() => websocket.isConnected(), {
    timeout: 10000,
  }).toBe(true);
});
```

### 6. Proxy Test

```typescript
test('should work behind reverse proxy', async ({ proxy, page }) => {
  // Start proxy with path prefix
  await proxy.start({
    targetPort: 3000,
    proxyPort: 8080,
    pathPrefix: '/terminal',
  });

  // Navigate through proxy
  await page.goto(proxy.proxyUrl);

  // Verify terminal works
  const terminal = await page.locator('.terminal');
  expect(terminal).toBeVisible();

  // Verify WebSocket upgrade
  const wsConnection = await page.waitForEvent('websocket');
  expect(wsConnection.url()).toContain('/terminal/ws');
});
```

## Coverage Goals

### Critical Paths (>95% Coverage)

1. **Terminal Input/Output**
   - Text input rendering
   - Command execution
   - Output display
   - ANSI code rendering

2. **WebSocket Communication**
   - Connection establishment
   - Message transmission
   - Error handling
   - Reconnection logic

3. **API Endpoints**
   - All REST endpoints
   - Error responses
   - Authentication (if implemented)

4. **Process Lifecycle**
   - Startup sequence
   - Exit handling
   - Cleanup procedures

### Important Paths (>90% Coverage)

1. **Multi-Terminal Management**
   - Terminal creation
   - Terminal switching
   - State isolation

2. **CLI Arguments**
   - Argument parsing
   - Command execution
   - Environment config

3. **ANSI Features**
   - Color codes (16, 256, true color)
   - Cursor movement
   - Screen control

### Nice-to-Have (>80% Coverage)

1. **Advanced Features**
   - Scrollback (10000+ lines)
   - Terminal resizing
   - Unicode support

2. **Edge Cases**
   - Large payloads
   - Special characters
   - Concurrent operations

## CI/CD Integration

### GitHub Actions Workflow

```yaml
name: E2E Tests

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        browser: [chromium, firefox, webkit]
        shard: [1/4, 2/4, 3/4, 4/4]

    steps:
      - uses: actions/checkout@v3

      - uses: actions/setup-node@v3
        with:
          node-version: '18'
          cache: 'npm'

      - name: Install dependencies
        run: npm ci

      - name: Install Playwright Browsers
        run: npx playwright install --with-deps ${{ matrix.browser }}

      - name: Run E2E tests
        run: npx playwright test --project=${{ matrix.browser }} --shard=${{ matrix.shard }}

      - name: Upload test results
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: test-results-${{ matrix.browser }}-${{ matrix.shard }}
          path: test-results/
          retention-days: 7

      - name: Upload coverage
        if: always()
        uses: codecov/codecov-action@v3
        with:
          files: ./coverage/lcov.info
          flags: e2e
```

### Test Execution Strategy

#### Local Development
```bash
# Run all tests
npm run test:e2e

# Run specific suite
npm run test:e2e -- tests/e2e/terminal

# Run in headed mode
npm run test:e2e -- --headed

# Run in debug mode
npm run test:e2e -- --debug

# Generate coverage
npm run test:e2e -- --coverage
```

#### CI/CD Pipeline
```bash
# Run tests in parallel with sharding
npx playwright test --shard=1/4 &
npx playwright test --shard=2/4 &
npx playwright test --shard=3/4 &
npx playwright test --shard=4/4 &
wait

# Generate combined report
npx playwright merge-reports --reporter html test-results/
```

## Performance Targets

### Test Execution Time

- **Unit tests:** <5 seconds per suite
- **Integration tests:** <30 seconds per suite
- **E2E tests:** <2 minutes per suite
- **Full test run:** <15 minutes (with sharding)

### Resource Usage

- **Memory:** <500MB per test worker
- **CPU:** <80% utilization per worker
- **Network:** <100MB total data transfer
- **Disk:** <1GB test artifacts

## Maintenance Strategy

### Test Health Monitoring

1. **Flaky Test Detection**
   - Track test retry rate
   - Alert on >5% retry rate
   - Quarantine flaky tests

2. **Coverage Tracking**
   - Monitor coverage trends
   - Alert on coverage drop >2%
   - Block PRs with coverage <90%

3. **Performance Monitoring**
   - Track test execution time
   - Alert on >20% slowdown
   - Optimize slow tests

### Test Maintenance Schedule

- **Daily:** Run full test suite in CI
- **Weekly:** Review flaky tests and coverage
- **Monthly:** Audit test quality and remove dead tests
- **Quarterly:** Major test refactoring and optimization

## Appendix

### A. Test Utilities

#### Terminal Interaction Helper

```typescript
export class TerminalHelper {
  constructor(private page: Page) {}

  async typeText(text: string): Promise<void> {
    await this.page.keyboard.type(text);
  }

  async pressKey(key: string): Promise<void> {
    await this.page.keyboard.press(key);
  }

  async waitForText(text: string, timeout = 5000): Promise<void> {
    await this.page.waitForSelector(
      `.terminal:has-text("${text}")`,
      { timeout }
    );
  }

  async getLines(): Promise<string[]> {
    return await this.page.evaluate(() => {
      const terminal = document.querySelector('.terminal');
      return Array.from(terminal.querySelectorAll('.line'))
        .map(line => line.textContent);
    });
  }
}
```

#### ANSI Parser

```typescript
export class AnsiParser {
  static parseColor(code: string): RGB {
    // Parse ANSI color codes to RGB values
    const match = code.match(/\x1b\[(\d+);(\d+);(\d+)m/);
    if (match) {
      return {
        r: parseInt(match[1]),
        g: parseInt(match[2]),
        b: parseInt(match[3]),
      };
    }
    throw new Error('Invalid ANSI color code');
  }

  static stripCodes(text: string): string {
    // Remove all ANSI escape codes
    return text.replace(/\x1b\[[0-9;]*m/g, '');
  }
}
```

### B. Common Test Patterns

#### Retry with Backoff

```typescript
async function retryWithBackoff<T>(
  fn: () => Promise<T>,
  maxRetries = 3,
  baseDelay = 1000
): Promise<T> {
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await fn();
    } catch (error) {
      if (i === maxRetries - 1) throw error;
      await new Promise(resolve =>
        setTimeout(resolve, baseDelay * Math.pow(2, i))
      );
    }
  }
  throw new Error('Unreachable');
}
```

#### Wait for Condition

```typescript
async function waitForCondition(
  condition: () => Promise<boolean>,
  timeout = 5000,
  interval = 100
): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeout) {
    if (await condition()) return;
    await new Promise(resolve => setTimeout(resolve, interval));
  }
  throw new Error('Condition not met within timeout');
}
```

### C. Debugging Tips

1. **Use Playwright Inspector**
   ```bash
   PWDEBUG=1 npm run test:e2e
   ```

2. **Capture Screenshots**
   ```typescript
   await page.screenshot({
     path: 'debug-screenshot.png',
     fullPage: true
   });
   ```

3. **Log Network Activity**
   ```typescript
   page.on('request', req => console.log('>>>', req.method(), req.url()));
   page.on('response', res => console.log('<<<', res.status(), res.url()));
   ```

4. **Enable Verbose Logging**
   ```bash
   DEBUG=pw:api npm run test:e2e
   ```

### D. References

- [Playwright Documentation](https://playwright.dev)
- [xterm.js Documentation](https://xtermjs.org)
- [WebSocket API](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket)
- [ANSI Escape Codes](https://en.wikipedia.org/wiki/ANSI_escape_code)

---

**Document Version:** 1.0
**Last Updated:** 2025-09-29
**Next Review:** 2025-10-29