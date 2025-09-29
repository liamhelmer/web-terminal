# Web-Terminal Test Suite

Comprehensive test suite for the web-terminal project, following SPARC methodology and spec-kit/008-testing-spec.md requirements.

## Quick Start

```bash
# Run all tests
cargo test --all

# Run unit tests only
cargo test --lib

# Run integration tests only
cargo test --test integration_tests

# Run specific test module
cargo test pty_manager

# Run with coverage
cargo tarpaulin --out Html --output-dir coverage

# Open coverage report
open coverage/index.html
```

## Test Structure

```
tests/
├── unit/                           # Unit tests (70-80% of suite)
│   ├── mod.rs
│   ├── pty_manager_test.rs        # PTY lifecycle tests (30+ tests)
│   └── session_manager_test.rs    # Session management tests (templates)
│
├── integration/                    # Integration tests (15-20% of suite)
│   ├── mod.rs
│   ├── terminal_session_test.rs   # Session lifecycle (10 tests)
│   ├── websocket_test.rs          # WebSocket protocol (13 tests)
│   └── concurrent_sessions_test.rs # Concurrency tests (12 tests)
│
├── integration_tests.rs            # Integration test entry point
├── TEST_REPORT.md                  # Detailed test report
└── README.md                       # This file
```

## Coverage Requirements

Per spec-kit/008-testing-spec.md:

| Category | Target | Status |
|----------|--------|--------|
| Overall | 80% | In Progress |
| Backend (Rust) | 85% | In Progress |
| Critical Path | 100% | In Progress |

## Test Categories

### Unit Tests

**PTY Manager Tests** (`tests/unit/pty_manager_test.rs`)
- ✅ 30+ test cases covering PTY lifecycle
- ✅ Concurrent access safety
- ✅ Resource cleanup
- ✅ Performance constraints

**Session Manager Tests** (`tests/unit/session_manager_test.rs`)
- 📋 11 test templates (awaiting SessionManager implementation)

### Integration Tests

**Terminal Session Tests** (`tests/integration/terminal_session_test.rs`)
- 📋 10 test scenarios for complete session lifecycle
- Tests create → execute → destroy flow
- Reconnection and multiple session support

**WebSocket Tests** (`tests/integration/websocket_test.rs`)
- 📋 13 test scenarios for WebSocket protocol
- Authentication, message flow, reconnection
- Latency and throughput testing

**Concurrent Session Tests** (`tests/integration/concurrent_sessions_test.rs`)
- 📋 12 test scenarios for concurrency
- Isolation, resource sharing, stress testing
- Maximum 10,000 session test

## Running Tests

### Development Workflow

```bash
# 1. Run quick unit tests during development
cargo test --lib --quiet

# 2. Run full test suite before commit
cargo test --all

# 3. Generate coverage report
cargo tarpaulin --out Html --output-dir coverage

# 4. Run specific test
cargo test test_pty_manager_spawn -- --nocapture

# 5. Run ignored tests (expensive stress tests)
cargo test -- --ignored
```

### Continuous Integration

Tests run automatically in GitHub Actions on:
- Every push to main
- Every pull request
- Daily (security scans)

See `.github/workflows/ci-rust.yml` for CI configuration.

## Writing Tests

### Test Naming Convention

```rust
// Format: test_<component>_<scenario>_<expected>
#[tokio::test]
async fn test_pty_manager_spawn_returns_valid_handle() {
    // ...
}
```

### AAA Pattern (Arrange-Act-Assert)

```rust
#[tokio::test]
async fn test_example() {
    // Arrange: Set up test conditions
    let manager = PtyManager::with_defaults();

    // Act: Execute the behavior being tested
    let handle = manager.spawn(None).expect("Failed to spawn PTY");

    // Assert: Verify expected outcomes
    assert_eq!(manager.count(), 1);
    assert!(manager.is_alive(handle.id()).await);

    // Cleanup: Clean up resources (important for async tests)
    manager.kill(handle.id()).await.expect("Failed to kill PTY");
}
```

### Test Independence

- Each test creates its own instances
- No shared mutable state between tests
- Tests can run in any order
- Explicit cleanup in async tests

### Async Testing

```rust
#[tokio::test]
async fn test_async_operation() {
    // Use tokio::test for async functions
    // Automatic Tokio runtime setup
    let result = some_async_function().await;
    assert!(result.is_ok());
}
```

## Test Coverage by Requirement

### Functional Requirements

| Requirement | Test Coverage | File |
|-------------|---------------|------|
| FR-1.2 Process Management | ✅ Complete | pty_manager_test.rs |
| FR-2.1.5 Terminal Dimensions | ✅ Complete | pty_manager_test.rs |
| FR-3 Real-time Communication | 📋 Template | websocket_test.rs |
| FR-4.1 Session Management | 📋 Template | session_manager_test.rs |
| FR-4.1.5 Resource Cleanup | ✅ Complete | pty_manager_test.rs |

### Non-Functional Requirements

| Requirement | Test Coverage | File |
|-------------|---------------|------|
| NFR-1.1.1 Command latency <100ms | 📋 Template | terminal_session_test.rs |
| NFR-1.1.3 WebSocket latency <20ms | 📋 Template | websocket_test.rs |
| NFR-1.1.5 Session creation <200ms | ✅ Complete | pty_manager_test.rs |
| NFR-3.3 10k concurrent sessions | 📋 Template | concurrent_sessions_test.rs |

## Performance Testing

### Benchmarks (Planned)

```bash
# Run benchmarks
cargo bench

# Profile specific benchmark
cargo bench --bench command_execution
```

Benchmark targets:
- Command execution < 100ms (p95)
- Session creation < 200ms
- WebSocket latency < 20ms

### Load Testing (Planned)

Using k6 for load testing:

```bash
# Run load test
k6 run tests/load/concurrent_sessions.js

# Custom load profile
k6 run -u 1000 -d 60s tests/load/concurrent_sessions.js
```

## Troubleshooting

### Tests Failing

1. Check compilation: `cargo build --tests`
2. Run with output: `cargo test -- --nocapture`
3. Run single test: `cargo test test_name -- --nocapture`
4. Check logs: `RUST_LOG=debug cargo test`

### Coverage Not Generating

1. Install tarpaulin: `cargo install cargo-tarpaulin`
2. Ensure tests pass: `cargo test`
3. Run with verbose: `cargo tarpaulin --verbose`

### Slow Tests

1. Run in parallel: `cargo test -- --test-threads=8`
2. Skip integration tests: `cargo test --lib`
3. Use release mode: `cargo test --release`

## Next Steps

1. ✅ Fix compilation errors in source code
2. ⏳ Run unit tests and verify coverage
3. ⏳ Implement SessionManager
4. ⏳ Implement WebSocket handler
5. ⏳ Enable integration tests
6. ⏳ Add E2E tests with Playwright
7. ⏳ Set up load testing with k6
8. ⏳ Configure coverage reporting in CI

## References

- [Testing Specification](../docs/spec-kit/008-testing-spec.md)
- [Architecture Specification](../docs/spec-kit/002-architecture.md)
- [Backend Specification](../docs/spec-kit/003-backend-spec.md)
- [Test Report](./TEST_REPORT.md)

## Contributing

When adding new tests:

1. Follow the AAA pattern
2. Use descriptive names
3. Document requirement references
4. Include cleanup for async tests
5. Test both happy and error paths
6. Consider edge cases and concurrency

---

**Test Suite Status:** 🟡 In Progress

**Coverage:** ⏳ Awaiting execution

**Last Updated:** 2025-09-29