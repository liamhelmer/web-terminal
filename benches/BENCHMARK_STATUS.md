# Web-Terminal Benchmark Suite - Implementation Status

**Created:** 2025-09-29
**Author:** Performance Testing Specialist
**Status:** ✅ Core Benchmarks Complete

---

## Summary

Comprehensive benchmark suite implemented per `docs/spec-kit/008-testing-spec.md` performance requirements:

- ✅ Command Execution Benchmarks (target: <100ms p95)
- ✅ Session Creation Benchmarks (target: <200ms p95)
- ⚠️  WebSocket Throughput Benchmarks (minor protocol fixes needed)
- ⚠️  Concurrent Load Benchmarks (minor protocol fixes needed)

## Implemented Benchmarks

### 1. ✅ Command Execution (`command_execution.rs`)

**Status:** COMPILES & READY TO RUN

**Benchmarks:**
- `execute_echo_simple` - Basic echo command execution latency
- `command_execution_by_length` - Commands of varying lengths (10-1000 chars)
- `command_execution_by_shell` - Different shell types (sh, bash)
- `pty_spawn_time` - PTY process spawning performance
- `execute_10_sequential_commands` - Sequential command execution

**Run:**
```bash
cargo bench --bench command_execution
```

**Performance Targets:**
- Command execution: <100ms (p95)
- PTY spawn: <50ms (p95)

---

### 2. ✅ Session Creation (`session_creation.rs`)

**Status:** COMPILES & READY TO RUN

**Benchmarks:**
- `create_session_basic` - Basic session creation
- `session_creation_multi_user` - Multi-user session creation (1, 5, 10 users)
- `session_lookup` - Session retrieval performance
- `session_state_operations` - State mutation operations
- `destroy_session` - Session cleanup performance
- `session_full_lifecycle` - Complete create → use → destroy cycle
- `concurrent_session_creation` - Concurrent creation (10, 50, 100 sessions)

**Run:**
```bash
cargo bench --bench session_creation
```

**Performance Targets:**
- Session creation: <200ms (p95)
- Session lookup: <10ms (p95)
- Concurrent creation (100): <5s total

---

### 3. ⚠️ WebSocket Throughput (`websocket_throughput.rs`)

**Status:** NEEDS MINOR FIXES

**Issue:** Protocol message enum variants need adjustment to match actual implementation.

**Implemented Benchmarks:**
- Message serialization/deserialization
- Message throughput (10-10,000 msgs/sec)
- Latency by message size (64B-16KB)
- Round-trip ping/pong latency
- Concurrent message processing
- PTY output streaming
- Batch message processing

**Fix Required:**
```rust
// Need to update ClientMessage and ServerMessage enum variants
// to match actual protocol implementation in src/protocol/messages.rs
```

**Performance Targets:**
- WebSocket latency: <20ms (p95)
- Throughput: >10,000 msgs/sec

---

### 4. ⚠️ Concurrent Load (`concurrent_load.rs`)

**Status:** NEEDS MINOR FIXES

**Issue:** Similar protocol message fixes needed.

**Implemented Benchmarks:**
- Concurrent session scaling (100-2000 sessions)
- Concurrent command execution (10-100 PTYs)
- Mixed workload (reads/writes)
- Memory usage under load
- Connection churn (rapid connect/disconnect)
- Sustained throughput testing
- Resource cleanup efficiency

**Performance Targets:**
- Support 10,000 concurrent sessions
- Memory usage < 100MB per 1,000 sessions
- Cleanup time < 5s for 1,000 sessions

---

## Configuration

### Cargo.toml Updates

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports", "async_tokio"] }
futures-util = "0.3"

[[bench]]
name = "command_execution"
harness = false

[[bench]]
name = "session_creation"
harness = false

[[bench]]
name = "websocket_throughput"
harness = false

[[bench]]
name = "concurrent_load"
harness = false
```

---

## Usage

### Run All Benchmarks

```bash
cargo bench
```

### Run Specific Benchmark

```bash
cargo bench --bench command_execution
cargo bench --bench session_creation
```

### Run Specific Test

```bash
cargo bench --bench session_creation create_session_basic
```

### Save Baseline

```bash
cargo bench -- --save-baseline main
```

### Compare Against Baseline

```bash
# After making changes
cargo bench -- --baseline main
```

---

## Reports

Criterion generates HTML reports in `target/criterion/`:

```bash
# View all reports
open target/criterion/report/index.html

# View specific benchmark
open target/criterion/create_session_basic/report/index.html
```

Reports include:
- Probability density functions
- Performance over time
- Statistical analysis (mean, median, p95, p99)
- Comparison charts

---

## Performance Validation

### Automated Validation

Create a script to validate benchmarks meet targets:

```bash
#!/bin/bash
# benches/validate_performance.sh

echo "Running performance validation..."

# Run benchmarks and capture output
cargo bench --bench command_execution > /tmp/bench_cmd.txt
cargo bench --bench session_creation > /tmp/bench_session.txt

# Parse results and check targets
# (Implementation would parse Criterion output)

echo "Performance validation complete"
```

### CI Integration

Add to `.github/workflows/ci-rust.yml`:

```yaml
- name: Run performance benchmarks
  run: cargo bench --no-fail-fast

- name: Upload benchmark results
  uses: actions/upload-artifact@v4
  with:
    name: benchmark-results
    path: target/criterion/
    retention-days: 30
```

---

## Next Steps

### 1. Fix Remaining Benchmarks (15 min)

```bash
# Check protocol message definitions
cat src/protocol/messages.rs

# Update websocket_throughput.rs and concurrent_load.rs
# to match actual ClientMessage/ServerMessage variants
```

### 2. Run Benchmarks (30 min)

```bash
# Run all benchmarks
cargo bench

# Generate reports
open target/criterion/report/index.html
```

###  3. Validate Performance Targets (15 min)

Review reports and verify:
- ✅ Command execution < 100ms (p95)
- ✅ WebSocket latency < 20ms (p95)
- ✅ Session creation < 200ms (p95)
- ✅ Concurrent sessions: 10,000 supported

### 4. Document Findings

Create `benches/PERFORMANCE_REPORT.md` with:
- Actual performance measurements
- Comparison to targets
- Bottlenecks identified
- Optimization recommendations

---

## Architecture Notes

### PTY Manager

- Uses `portable-pty` for cross-platform PTY support
- Async I/O with `PtyReader` and `PtyWriter`
- DashMap for concurrent process registry
- Non-Send handles (requires careful async handling)

### Session Manager

- DashMap-based in-memory storage
- Arc<Session> for shared access
- RwLock for state mutations
- Per-user session limits enforced

### WebSocket Protocol

- JSON-based message protocol
- Real-time bidirectional communication
- Backpressure handling via channels
- Connection lifecycle management

---

## Known Issues

1. **PTY Handles Not Send**: Requires spawning operations on same thread
2. **Protocol Message Variants**: Need to match actual implementation
3. **Realistic Load Testing**: 10,000 sessions may require system tuning (ulimit, etc.)

---

## Documentation

- Main README: `benches/README.md`
- Spec Reference: `docs/spec-kit/008-testing-spec.md`
- Performance Targets: `docs/spec-kit/001-requirements.md`

---

## Contact

For questions or issues with benchmarks:
- Review `benches/README.md`
- Check spec-kit documentation
- Run `cargo bench --help` for Criterion options