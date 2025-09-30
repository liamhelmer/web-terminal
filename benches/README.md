# Web-Terminal Performance Benchmarks

Comprehensive benchmark suite for web-terminal backend performance testing.

## Overview

This benchmark suite validates the performance requirements defined in `docs/spec-kit/008-testing-spec.md`:

- **Command Execution**: <100ms (p95)
- **WebSocket Latency**: <20ms (p95)
- **Session Creation**: <200ms (p95)
- **Concurrent Sessions**: Support 10,000 sessions

## Benchmark Suites

### 1. Command Execution (`command_execution.rs`)

Tests command execution latency through PTY processes.

**Benchmarks:**
- `execute_echo_simple` - Basic echo command
- `command_execution_by_length` - Commands of varying lengths (10-1000 chars)
- `command_execution_by_shell` - Different shell types (sh, bash)
- `pty_spawn_time` - PTY process spawning latency
- `execute_10_sequential_commands` - Sequential command execution

**Run:**
```bash
cargo bench --bench command_execution
```

### 2. WebSocket Throughput (`websocket_throughput.rs`)

Tests WebSocket message processing and latency.

**Benchmarks:**
- `websocket_message_serialization` - Message encoding/decoding
- `websocket_message_throughput` - Messages per second (10-10,000 msgs)
- `websocket_latency_by_message_size` - Latency vs message size (64B-16KB)
- `websocket_round_trip_ping_pong` - Round-trip latency
- `websocket_concurrent_messages` - Concurrent message processing (10-100)
- `pty_output_to_websocket_messages` - PTY output streaming
- `websocket_batch_processing` - Batch message handling (10-500)

**Run:**
```bash
cargo bench --bench websocket_throughput
```

### 3. Session Creation (`session_creation.rs`)

Tests session lifecycle performance.

**Benchmarks:**
- `create_session_basic` - Basic session creation
- `session_creation_with_config` - Various configurations
- `create_session_with_pty` - Session with PTY initialization
- `concurrent_session_creation` - Concurrent creation (10-500 sessions)
- `session_lookup` - Session retrieval performance
- `session_state_update` - State mutation performance
- `destroy_session` - Cleanup performance
- `session_full_lifecycle` - Create → Use → Destroy
- `session_memory_footprint` - Memory usage (100 sessions)

**Run:**
```bash
cargo bench --bench session_creation
```

### 4. Concurrent Load (`concurrent_load.rs`)

Tests system behavior under heavy concurrent load.

**Benchmarks:**
- `concurrent_sessions_scale` - Scale testing (100-2000 sessions)
- `concurrent_command_execution` - Multi-session commands (10-200)
- `mixed_workload` - Read/write operations (50-200 sessions)
- `memory_usage_1000_sessions` - Memory under load
- `connection_churn` - Rapid connect/disconnect (100-1000 ops)
- `sustained_throughput_100_sessions_10s` - Sustained operations
- `cleanup_1000_sessions` - Resource cleanup efficiency

**Run:**
```bash
cargo bench --bench concurrent_load
```

## Running Benchmarks

### Run All Benchmarks

```bash
cargo bench
```

### Run Specific Benchmark Suite

```bash
cargo bench --bench command_execution
cargo bench --bench websocket_throughput
cargo bench --bench session_creation
cargo bench --bench concurrent_load
```

### Run Specific Test Within Suite

```bash
cargo bench --bench command_execution execute_echo_simple
cargo bench --bench session_creation concurrent_session_creation
```

### Save Baseline for Comparison

```bash
cargo bench -- --save-baseline main
```

### Compare Against Baseline

```bash
# After making changes
cargo bench -- --baseline main
```

## Interpreting Results

### Criterion Output

Criterion provides detailed statistics:

```
execute_echo_simple     time:   [45.123 ms 48.456 ms 52.789 ms]
                        change: [-5.2% -2.1% +1.3%]
```

- **time**: [min mean max] - Performance range
- **change**: Comparison to previous run

### Performance Targets

Check that p95 latencies meet spec requirements:

| Metric | Target | Command |
|--------|--------|---------|
| Command execution | <100ms | `cargo bench command_execution` |
| WebSocket latency | <20ms | `cargo bench websocket_throughput` |
| Session creation | <200ms | `cargo bench session_creation` |
| Concurrent sessions | 10,000 | `cargo bench concurrent_load` |

## Reports

Criterion generates HTML reports in `target/criterion/`:

```bash
# View reports
open target/criterion/report/index.html

# Specific benchmark
open target/criterion/execute_echo_simple/report/index.html
```

Reports include:
- Probability density function (PDF)
- Slope analysis
- Historical comparison charts
- Statistical analysis

## CI Integration

Benchmarks run in CI via GitHub Actions:

```yaml
- name: Run benchmarks
  run: cargo bench --no-fail-fast

- name: Upload benchmark results
  uses: actions/upload-artifact@v4
  with:
    name: benchmark-results
    path: target/criterion/
```

## Optimization Tips

### If Command Execution is Slow

1. Check PTY spawn overhead (`pty_spawn_time`)
2. Profile shell initialization
3. Optimize I/O buffering
4. Consider PTY pooling

### If WebSocket is Slow

1. Check serialization overhead
2. Optimize message batching
3. Use binary encoding (MessagePack)
4. Profile async runtime

### If Session Creation is Slow

1. Reduce initialization work
2. Lazy-load session resources
3. Optimize state management
4. Profile lock contention

### If Concurrent Load Fails

1. Check memory leaks
2. Profile resource cleanup
3. Optimize concurrent data structures (DashMap)
4. Review backpressure handling

## Advanced Usage

### Profiling with Flamegraph

```bash
# Install cargo-flamegraph
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --bench command_execution
```

### Memory Profiling

```bash
# Install valgrind (Linux/macOS)
valgrind --tool=massif --massif-out-file=massif.out \
  target/release/deps/command_execution-*

# Analyze
ms_print massif.out
```

### Custom Benchmarks

Add new benchmarks by creating `benches/my_benchmark.rs`:

```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn my_benchmark(c: &mut Criterion) {
    c.bench_function("my_test", |b| {
        b.iter(|| {
            // Your code here
        });
    });
}

criterion_group!(benches, my_benchmark);
criterion_main!(benches);
```

Update `Cargo.toml`:

```toml
[[bench]]
name = "my_benchmark"
harness = false
```

## References

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [Web-Terminal Testing Spec](../docs/spec-kit/008-testing-spec.md)
- [Performance Requirements](../docs/spec-kit/001-requirements.md)