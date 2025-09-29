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

### ⚠️ CRITICAL: GitHub Actions CI is MANDATORY

**Per spec-kit requirements updated 2025-09-29:**
- **GitHub Actions workflows are REQUIRED for feature completion**
- **All workflows MUST pass before merge**
- **Security scans are BLOCKING** (cargo audit, npm audit)
- **Test coverage enforcement in CI** (>80% required)

### CI Architecture (2025 Best Practices)

The web-terminal project uses **multiple specialized GitHub Actions workflows** instead of a single monolithic workflow:

1. **`ci-rust.yml`** - Rust backend CI (tests, clippy, fmt, audit, coverage)
2. **`ci-frontend.yml`** - TypeScript frontend CI (lint, typecheck, tests, coverage)
3. **`ci-integration.yml`** - Full integration tests with real server
4. **`security.yml`** - Security scanning (daily + on PR)
5. **`release.yml`** - Automated releases with cross-platform builds

**Why multiple workflows?**
- ✅ Parallel execution (faster CI, <5 min target)
- ✅ Clear failure isolation (know exactly what broke)
- ✅ Selective re-runs (don't re-run everything on minor changes)
- ✅ Different schedules (security daily, tests on PR)

### CI Pipeline Requirements

#### Required Actions (2025-Compliant)

**🚨 DEPRECATED ACTIONS (DO NOT USE):**
- ❌ `actions-rs/toolchain` - DEPRECATED
- ❌ `actions-rs/cargo` - DEPRECATED
- ❌ `actions/upload-artifact@v3` - UNSUPPORTED as of Jan 2025

**✅ APPROVED ACTIONS (Use These):**
- ✅ `dtolnay/rust-toolchain@stable` - Rust toolchain (minimalist, recommended)
- ✅ `Swatinem/rust-cache@v2` - Rust dependency caching (50% faster builds)
- ✅ `actions/upload-artifact@v4` - Artifact upload (10x performance improvement)
- ✅ `actions/setup-node@v4` - Node.js with built-in pnpm caching
- ✅ `actions-rust-lang/audit@v1` - Security vulnerability scanning
- ✅ `EmbarkStudios/cargo-deny-action@v2` - License + security compliance
- ✅ `taiki-e/upload-rust-binary-action@v1` - Cross-platform binary releases

#### Security Hardening

```yaml
# Pin actions to commit SHA (not tags) for supply chain security
- uses: actions/checkout@b4ffde65f46336ab88eb53be808477a3936bae11 # v4.1.1
- uses: dtolnay/rust-toolchain@stable
- uses: Swatinem/rust-cache@23bce251a8cd2ffc3c1075eaa2367cf899916d84 # v2.7.3
```

**Additional Requirements:**
- Use Dependabot to auto-update GitHub Actions
- Require PR approval for `.github/workflows/` changes
- Use OIDC for cloud deployments (no long-lived secrets)
- Pin action versions to commit SHA (prevents supply chain attacks)

### Workflow Files

All workflow files are located in `.github/workflows/`:

#### 1. `ci-rust.yml` - Rust Backend CI

**Purpose:** Test, lint, format check, security audit, code coverage

**Triggers:** Push, Pull Request

**Jobs:**
- `test` - Run all unit and integration tests
- `clippy` - Rust linting
- `fmt` - Code formatting check
- `audit` - Security vulnerability scanning (cargo audit)
- `deny` - License compliance and banned dependencies
- `coverage` - Code coverage report → Codecov

**Performance:** < 3 minutes (with caching)

**Example:**
```yaml
name: Rust CI

on:
  push:
    branches: [main]
  pull_request:

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --all-features

  clippy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy -- -D warnings

  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/audit@v1
        with:
          denyWarnings: true
```

#### 2. `ci-frontend.yml` - Frontend CI

**Purpose:** Lint, typecheck, unit tests, E2E tests, code coverage

**Triggers:** Push, Pull Request

**Jobs:**
- `lint` - ESLint checks
- `typecheck` - TypeScript compilation check
- `test` - Vitest unit tests
- `e2e` - Playwright end-to-end tests
- `coverage` - Code coverage → Codecov

**Performance:** < 2 minutes (with caching)

#### 3. `ci-integration.yml` - Integration Tests

**Purpose:** Full-stack integration testing with real server

**Triggers:** Push, Pull Request

**Process:**
1. Build Rust backend (port 8080)
2. Build TypeScript frontend
3. Start backend server on port 8080 (single-port architecture)
4. Run Playwright E2E tests against running server
5. Upload test reports on failure

**Performance:** < 4 minutes

**Critical:** Tests MUST use relative URLs and respect single-port architecture

#### 4. `security.yml` - Security Scanning

**Purpose:** Continuous security monitoring

**Triggers:** Daily at 00:00 UTC, Pull Request (blocking)

**Scans:**
- `cargo audit` - Rust dependency vulnerabilities
- `npm audit` - Node.js dependency vulnerabilities
- `cargo deny` - License compliance and banned crates
- OWASP ZAP (optional, in dedicated workflow)

**Policy:** Security vulnerabilities are **BLOCKING** - PRs cannot merge with critical vulnerabilities

#### 5. `release.yml` - Automated Releases

**Purpose:** Create GitHub releases with cross-platform binaries

**Triggers:** Version tags (`v*`)

**Process:**
1. Run all CI checks (enforce passing tests)
2. Build cross-platform binaries (Linux, macOS, Windows)
3. Create GitHub Release with changelog
4. Upload binary artifacts
5. Build and push Docker image
6. Deploy to staging/production

**Cross-Platform Matrix:**
- Linux: x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu
- macOS: universal binaries (x86_64 + aarch64)
- Windows: x86_64-pc-windows-msvc

### Coverage Reporting

**Tool:** Codecov (cloud-based)

**Integration:**
```yaml
- name: Generate coverage
  run: cargo tarpaulin --out Xml

- name: Upload to Codecov
  uses: codecov/codecov-action@v4
  with:
    token: ${{ secrets.CODECOV_TOKEN }}
    files: ./cobertura.xml
    fail_ci_if_error: true  # Block on upload failure
```

**Requirements:**
- Coverage reports uploaded from CI
- Coverage badge in README
- Enforce 80% minimum coverage (fail CI if below)

### Caching Strategy

**Rust:**
```yaml
- uses: Swatinem/rust-cache@v2
  with:
    # Cache key based on Cargo.lock + rust-toolchain
    # Automatic cache cleaning for stale artifacts
```

**Node.js:**
```yaml
- uses: actions/setup-node@v4
  with:
    node-version: 20
    cache: 'pnpm'  # Built-in pnpm caching
```

**Benefits:**
- 50% faster Rust builds (from research)
- 30-40% faster Node.js builds
- Automatic cache invalidation on dependency changes

### Artifact Management

**Test Reports:**
```yaml
- uses: actions/upload-artifact@v4
  if: failure()
  with:
    name: playwright-report
    path: playwright-report/
    retention-days: 7  # Reduced from default 90 days
```

**Binary Releases:**
```yaml
- uses: actions/upload-artifact@v4
  with:
    name: web-terminal-${{ matrix.platform }}
    path: target/release/web-terminal
    retention-days: 30
```

### Branch Protection Rules

**Required for `main` branch:**
- ✅ Require pull request reviews (1 approval minimum)
- ✅ Require status checks to pass:
  - `ci-rust.yml` / `test`
  - `ci-rust.yml` / `clippy`
  - `ci-rust.yml` / `audit`
  - `ci-frontend.yml` / `test`
  - `ci-frontend.yml` / `typecheck`
  - `ci-integration.yml` / `integration-tests`
  - `security.yml` / `cargo-audit`
- ✅ Require branches to be up to date
- ✅ Require conversation resolution before merging
- ✅ Do not allow bypassing (no admin exceptions)

### Performance Targets

Per spec-kit/009-deployment-spec.md:
- ✅ **Total CI pipeline duration: < 5 minutes** (all workflows combined)
- ✅ **Rust CI: < 3 minutes** (with caching)
- ✅ **Frontend CI: < 2 minutes** (with caching)
- ✅ **Integration tests: < 4 minutes**
- ✅ **Security scans: < 2 minutes**

**Optimization techniques:**
- Parallel job execution
- Smart caching (Swatinem/rust-cache, pnpm cache)
- Matrix builds only on release
- Selective test running (path filters)

### Monitoring and Alerts

**Failure Notifications:**
- GitHub Actions email notifications (enabled by default)
- Slack integration (optional)
- Discord webhook (optional)

**Metrics to Track:**
- CI success rate (target: >95%)
- Average CI duration (target: <5 min)
- Flaky test rate (target: <2%)
- Security vulnerability response time (target: <24h for critical)

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

**Note:** With in-memory storage architecture (per ADR 012-data-storage-decision.md), all session data is stored in DashMap structures. Test fixtures provide data for initializing in-memory state during tests. No persistent database is used.

---

## Acceptance Criteria

### Feature Acceptance

**🚨 MANDATORY: A feature is ONLY considered complete when ALL GitHub Actions CI workflows pass.**

A feature is considered complete when:

1. ✅ **GitHub Actions CI workflows pass** (REQUIRED - blocking criteria)
   - `ci-rust.yml` - All Rust tests, linting, security scans pass
   - `ci-frontend.yml` - All TypeScript tests, linting, type checks pass
   - `ci-integration.yml` - Full integration tests pass with real server
2. ✅ Unit tests pass with >80% coverage (enforced in CI)
3. ✅ Integration tests pass (enforced in CI)
4. ✅ E2E tests pass for happy path (enforced in CI)
5. ✅ Performance benchmarks meet targets (enforced in CI)
6. ✅ Security tests pass (enforced in CI)
7. ✅ Code review approved
8. ✅ Documentation updated

**Failure Policy:** If ANY GitHub Actions workflow fails, the feature is NOT complete. No merge allowed until all checks are green.

### Release Acceptance

**🚨 MANDATORY: A release is ONLY ready when ALL GitHub Actions workflows pass, including security scans.**

A release is ready when:

1. ✅ **All GitHub Actions CI workflows passing** (REQUIRED - blocking criteria)
   - `ci-rust.yml` ✅
   - `ci-frontend.yml` ✅
   - `ci-integration.yml` ✅
   - `security.yml` ✅ (no critical vulnerabilities)
2. ✅ Coverage >80% (enforced in CI, uploaded to Codecov)
3. ✅ No critical security vulnerabilities (cargo audit + npm audit in CI)
4. ✅ Load tests pass (10k concurrent sessions)
5. ✅ E2E tests pass on all supported browsers (enforced in CI)
6. ✅ Performance regression tests pass (enforced in CI)
7. ✅ Manual QA sign-off
8. ✅ `release.yml` workflow successfully creates GitHub Release with artifacts

**Failure Policy:** Release process is BLOCKED if any workflow fails. All issues must be resolved before release.

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