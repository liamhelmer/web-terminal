# Web-Terminal: Testing Specification

**Version:** 1.0.0
**Status:** Draft
**Author:** Liam Helmer
**Last Updated:** 2025-09-29

---

## Overview

This document defines the testing strategy, requirements, and acceptance criteria for the web-terminal project.

**Testing Goal:** Achieve >80% code coverage with comprehensive unit, integration, and end-to-end tests.

---

## Testing Pyramid

```
           ╱╲
          ╱  ╲
         ╱ E2E╲         5-10% (Critical user flows)
        ╱──────╲
       ╱        ╲
      ╱Integration╲     15-20% (Component interaction)
     ╱────────────╲
    ╱              ╲
   ╱  Unit Tests    ╲   70-80% (Individual functions)
  ╱──────────────────╲
```

---

## Test Categories

### 1. Unit Tests

**Scope:** Individual functions and methods in isolation

**Coverage Target:** 80%+

**Tools:**
- Rust: `cargo test`, `mockall` (mocking)
- TypeScript: `vitest`

**Location:**
- Rust: `src/**/tests/` or inline `#[cfg(test)]`
- TypeScript: `tests/unit/`

#### Example Unit Test (Rust)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let manager = SessionManager::new();
        let user_id = UserId::new("user123");

        let result = manager.create_session(user_id);
        assert!(result.is_ok());

        let session = result.unwrap();
        assert_eq!(session.user_id, user_id);
        assert!(session.id.len() > 0);
    }

    #[test]
    fn test_command_validation() {
        let executor = CommandExecutor::new();

        // Valid command
        assert!(executor.validate_command("ls -la").is_ok());

        // Blocked command
        assert!(executor.validate_command("rm -rf /").is_err());

        // Empty command
        assert!(executor.validate_command("").is_err());
    }
}
```

#### Example Unit Test (TypeScript)

```typescript
import { describe, it, expect } from 'vitest';
import { Terminal } from '../src/terminal/terminal';

describe('Terminal', () => {
  it('should initialize with default config', () => {
    const container = document.createElement('div');
    const terminal = new Terminal(container, {
      fontSize: 14,
      fontFamily: 'monospace',
      theme: {},
    });

    expect(terminal).toBeDefined();
  });

  it('should write text to terminal', () => {
    const container = document.createElement('div');
    const terminal = new Terminal(container, {});

    terminal.open();
    terminal.write('Hello World');

    // Assert output contains text
    expect(container.textContent).toContain('Hello World');
  });
});
```

---

### 2. Integration Tests

**Scope:** Multiple components working together

**Coverage Target:** 15-20%

**Tools:**
- Rust: `cargo test --test integration_*`
- TypeScript: `vitest` with mocked backend

**Location:**
- Rust: `tests/`
- TypeScript: `tests/integration/`

#### Example Integration Test (Rust)

```rust
// tests/session_integration_test.rs

use web_terminal::*;

#[tokio::test]
async fn test_session_lifecycle() {
    let config = Config::from_env().unwrap();
    let manager = SessionManager::new(config.session);
    let executor = CommandExecutor::new();

    // Create session
    let session = manager.create_session(UserId::new("test"))
        .await
        .unwrap();

    // Execute command
    let result = executor.execute(session.id.clone(), "echo 'test'")
        .await
        .unwrap();

    assert_eq!(result.exit_code, 0);
    assert!(result.stdout.contains("test"));

    // Destroy session
    manager.destroy_session(&session.id).await.unwrap();

    // Verify session destroyed
    assert!(manager.get_session(&session.id).await.is_err());
}
```

#### Example Integration Test (TypeScript)

```typescript
import { describe, it, expect, vi } from 'vitest';
import { App } from '../src/app';
import { MockWebSocket } from './mocks/websocket';

describe('App Integration', () => {
  it('should handle full command execution flow', async () => {
    const container = document.createElement('div');
    const mockWs = new MockWebSocket();

    const app = new App(container, {
      websocket: { url: 'mock' },
      terminal: {},
    });

    await app.start();

    // Simulate user input
    app.terminal.onData('ls -la\n');

    // Simulate server response
    mockWs.simulateMessage({
      type: 'output',
      data: 'file1.txt\nfile2.txt\n',
    });

    // Verify terminal displays output
    expect(container.textContent).toContain('file1.txt');
    expect(container.textContent).toContain('file2.txt');
  });
});
```

---

### 3. End-to-End Tests

**Scope:** Full user workflows from browser to backend

**Coverage Target:** 5-10% (critical paths only)

**Tools:**
- Playwright (browser automation)

**Location:** `tests/e2e/`

#### Example E2E Test

```typescript
// tests/e2e/terminal.spec.ts

import { test, expect } from '@playwright/test';

test.describe('Terminal E2E', () => {
  // Use environment variable or relative URL for testing
  const baseUrl = process.env.TEST_BASE_URL || 'http://localhost:8080';

  test('should execute commands and display output', async ({ page }) => {
    // Navigate to terminal (single-port server)
    await page.goto(baseUrl);

    // Wait for terminal to load
    await page.waitForSelector('.terminal');

    // Type command
    await page.keyboard.type('echo "Hello World"');
    await page.keyboard.press('Enter');

    // Wait for output
    await page.waitForSelector('text=Hello World', { timeout: 5000 });

    // Verify output displayed
    const terminalText = await page.textContent('.terminal');
    expect(terminalText).toContain('Hello World');
  });

  test('should handle file upload', async ({ page }) => {
    await page.goto(baseUrl);
    await page.waitForSelector('.terminal');

    // Click upload button
    await page.click('[data-testid="upload-button"]');

    // Upload file
    const fileInput = await page.$('input[type="file"]');
    await fileInput.setInputFiles('./fixtures/test.txt');

    // Wait for upload confirmation
    await page.waitForSelector('text=Upload complete', { timeout: 10000 });

    // Verify file exists
    await page.keyboard.type('ls -la');
    await page.keyboard.press('Enter');
    await page.waitForSelector('text=test.txt');
  });

  test('should recover from disconnection', async ({ page }) => {
    await page.goto(baseUrl);
    await page.waitForSelector('.terminal');

    // Execute command
    await page.keyboard.type('echo "before disconnect"');
    await page.keyboard.press('Enter');
    await page.waitForSelector('text=before disconnect');

    // Simulate disconnection (stop server, or inject network error)
    await page.evaluate(() => {
      // Close WebSocket
      (window as any).wsClient.disconnect();
    });

    // Wait for reconnection indicator
    await page.waitForSelector('text=Reconnecting', { timeout: 2000 });

    // Wait for reconnection
    await page.waitForSelector('text=Connected', { timeout: 10000 });

    // Execute command after reconnection
    await page.keyboard.type('echo "after reconnect"');
    await page.keyboard.press('Enter');
    await page.waitForSelector('text=after reconnect');
  });
});
```

---

### 4. Performance Tests

**Scope:** Load testing, stress testing, latency benchmarks

**Tools:**
- `criterion` (Rust benchmarks)
- `k6` (load testing)

#### Benchmark Test (Rust)

```rust
// benches/command_execution.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use web_terminal::*;

fn benchmark_command_execution(c: &mut Criterion) {
    let executor = CommandExecutor::new();

    c.bench_function("execute echo command", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                executor.execute(
                    SessionId::new("test"),
                    black_box("echo 'benchmark'".to_string())
                ).await
            })
        });
    });
}

criterion_group!(benches, benchmark_command_execution);
criterion_main!(benches);
```

#### Load Test (k6)

```javascript
// tests/load/concurrent_sessions.js

import ws from 'k6/ws';
import { check } from 'k6';

export let options = {
  stages: [
    { duration: '30s', target: 100 },   // Ramp up to 100 users
    { duration: '1m', target: 100 },    // Stay at 100 users
    { duration: '30s', target: 500 },   // Ramp up to 500 users
    { duration: '1m', target: 500 },    // Stay at 500 users
    { duration: '30s', target: 0 },     // Ramp down
  ],
};

export default function () {
  // Use environment variable for flexible testing across environments
  const host = __ENV.TEST_HOST || 'localhost:8080';
  const url = `ws://${host}/ws?token=test-token`;

  const res = ws.connect(url, function (socket) {
    socket.on('open', () => {
      // Send command
      socket.send(JSON.stringify({
        type: 'command',
        data: 'echo "load test"',
      }));
    });

    socket.on('message', (data) => {
      const msg = JSON.parse(data);

      check(msg, {
        'received output': (m) => m.type === 'output',
      });
    });

    socket.setTimeout(() => {
      socket.close();
    }, 5000);
  });

  check(res, { 'status is 101': (r) => r && r.status === 101 });
}
```

---

### 5. Security Tests

**Scope:** Vulnerability testing, penetration testing

**Tools:**
- OWASP ZAP (automated scanning)
- Manual penetration testing

#### Security Test Checklist

```typescript
// tests/security/auth.spec.ts

import { test, expect } from '@playwright/test';

test.describe('Security Tests', () => {
  const baseUrl = process.env.TEST_BASE_URL || 'http://localhost:8080';

  test('should reject unauthenticated WebSocket connection', async ({ page }) => {
    let errorCaught = false;

    page.on('websocket', ws => {
      ws.on('close', () => {
        errorCaught = true;
      });
    });

    await page.goto(baseUrl);

    // Attempt connection without token using relative path (single-port architecture)
    await page.evaluate(() => {
      // WebSocket URL constructed from current page location
      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const ws = new WebSocket(`${protocol}//${window.location.host}/ws`);
    });

    // Wait for connection to be rejected
    await page.waitForTimeout(2000);
    expect(errorCaught).toBe(true);
  });

  test('should prevent path traversal', async ({ page }) => {
    await page.goto(baseUrl);
    await page.waitForSelector('.terminal');

    // Attempt path traversal
    await page.keyboard.type('cd ../../../../../../etc');
    await page.keyboard.press('Enter');

    // Should see error
    await page.waitForSelector('text=Permission denied');
  });

  test('should block dangerous commands', async ({ page }) => {
    await page.goto(baseUrl);
    await page.waitForSelector('.terminal');

    // Attempt dangerous command
    await page.keyboard.type('rm -rf /');
    await page.keyboard.press('Enter');

    // Should see error
    await page.waitForSelector('text=Command not allowed');
  });
});
```

---

## Test Coverage Requirements

### Coverage Targets

| Category | Minimum Coverage |
|----------|-----------------|
| Overall | 80% |
| Backend (Rust) | 85% |
| Frontend (TypeScript) | 75% |
| Critical Path | 100% |

### Critical Path Components

1. Authentication flow
2. Session creation/destruction
3. Command execution
4. WebSocket communication
5. File system operations
6. Security validation

---

## Continuous Integration

### CI Pipeline

```yaml
# .github/workflows/test.yml

name: Tests

on: [push, pull_request]

jobs:
  test-backend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - run: cargo test --all-features
      - run: cargo test --test integration_*
      - run: cargo tarpaulin --out Xml

  test-frontend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: pnpm/action-setup@v2
      - run: pnpm install
      - run: pnpm test
      - run: pnpm test:coverage

  test-e2e:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: docker-compose up -d
      - run: pnpm test:e2e
      - uses: actions/upload-artifact@v3
        if: failure()
        with:
          name: playwright-report
          path: playwright-report/

  security-scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo audit
      - run: pnpm audit
```

---

## Test Data Management

### Fixtures

```
tests/
├── fixtures/
│   ├── users.json           # Test user data
│   ├── sessions.json        # Test session data
│   ├── commands.txt         # Test commands
│   └── files/
│       ├── test.txt
│       └── test.bin
```

### Test Database

```sql
-- tests/fixtures/schema.sql

CREATE TABLE test_users (
  id VARCHAR(36) PRIMARY KEY,
  username VARCHAR(255) NOT NULL,
  email VARCHAR(255) NOT NULL
);

INSERT INTO test_users VALUES
  ('user1', 'alice', 'alice@test.com'),
  ('user2', 'bob', 'bob@test.com');
```

---

## Acceptance Criteria

### Feature Acceptance

A feature is considered complete when:

1. ✅ Unit tests pass with >80% coverage
2. ✅ Integration tests pass
3. ✅ E2E tests pass for happy path
4. ✅ Performance benchmarks meet targets
5. ✅ Security tests pass
6. ✅ Code review approved
7. ✅ Documentation updated

### Release Acceptance

A release is ready when:

1. ✅ All tests passing in CI
2. ✅ Coverage >80%
3. ✅ No critical security vulnerabilities
4. ✅ Load tests pass (10k concurrent sessions)
5. ✅ E2E tests pass on all supported browsers
6. ✅ Performance regression tests pass
7. ✅ Manual QA sign-off

---

## Testing Best Practices

### 1. Test Naming Convention

```rust
// Rust
#[test]
fn test_<component>_<scenario>_<expected_outcome>() { }

// Example:
#[test]
fn test_session_manager_create_session_returns_valid_session() { }
```

```typescript
// TypeScript
describe('<Component>', () => {
  it('should <expected behavior> when <scenario>', () => {});
});

// Example:
describe('Terminal', () => {
  it('should display output when command executed', () => {});
});
```

### 2. AAA Pattern

```rust
#[test]
fn test_example() {
    // Arrange
    let manager = SessionManager::new();
    let user_id = UserId::new("test");

    // Act
    let result = manager.create_session(user_id).await;

    // Assert
    assert!(result.is_ok());
}
```

### 3. Test Independence

- Each test should be independent
- No shared state between tests
- Tests can run in any order

### 4. Mock External Dependencies

```rust
use mockall::predicate::*;
use mockall::*;

#[automock]
trait Database {
    fn get_user(&self, id: UserId) -> Result<User>;
}

#[test]
fn test_with_mock() {
    let mut mock_db = MockDatabase::new();
    mock_db.expect_get_user()
        .with(eq(UserId::new("test")))
        .returning(|_| Ok(User::default()));

    // Test code using mock_db
}
```

---

## Version History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0 | 2025-09-29 | Liam Helmer | Initial testing specification |