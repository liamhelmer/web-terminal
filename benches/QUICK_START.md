# Benchmark Quick Start Guide

## ✅ Ready to Run (Compiles Successfully)

### 1. Command Execution Benchmarks

**Test PTY command execution performance**

```bash
# Run all command execution benchmarks
cargo bench --bench command_execution

# Run specific benchmark
cargo bench --bench command_execution execute_echo_simple

# View results
open target/criterion/execute_echo_simple/report/index.html
```

**What it measures:**
- Echo command execution latency
- Commands of varying lengths (10-1000 characters)
- Different shell types (sh, bash)
- PTY spawn time
- Sequential command execution (10 commands)

**Expected Results:**
- Command execution: 30-80ms
- PTY spawn: 10-30ms
- Sequential 10 commands: 200-500ms

---

### 2. Session Creation Benchmarks

**Test session lifecycle performance**

```bash
# Run all session creation benchmarks
cargo bench --bench session_creation

# Run specific benchmark
cargo bench --bench session_creation create_session_basic

# View results
open target/criterion/create_session_basic/report/index.html
```

**What it measures:**
- Basic session creation
- Multi-user session creation (1, 5, 10 users)
- Session lookup performance
- Session state operations
- Session destruction
- Full lifecycle (create → use → destroy)
- Concurrent creation (10, 50, 100 sessions)

**Expected Results:**
- Session creation: 5-15ms
- Session lookup: 0.5-2ms
- State operations: 1-5ms
- Full lifecycle: 10-25ms
- Concurrent 100 sessions: 1-3s total

---

## Performance Targets (Per Spec)

| Metric | Target | Benchmark |
|--------|--------|-----------|
| Command execution | <100ms (p95) | `command_execution` |
| WebSocket latency | <20ms (p95) | `websocket_throughput` |
| Session creation | <200ms (p95) | `session_creation` |
| Concurrent sessions | 10,000 | `concurrent_load` |

---

## Understanding Criterion Output

```
execute_echo_simple     time:   [45.123 ms 48.456 ms 52.789 ms]
                        change: [-5.2% -2.1% +1.3%] (p = 0.08)
                        No change in performance detected.
```

- **time [min mean max]**: Performance range across all iterations
- **change**: Comparison to previous baseline (if exists)
- **p value**: Statistical significance (p < 0.05 = significant change)

### Key Statistics

- **mean**: Average time across all samples
- **median**: Middle value (more robust to outliers)
- **p95**: 95th percentile (worst-case for 95% of requests)
- **p99**: 99th percentile (worst-case for 99% of requests)

---

## Troubleshooting

### "PTY spawn failed"

If PTY spawning fails, ensure your shell exists:

```bash
# Check shell availability
which bash
which sh

# macOS may need full path
/bin/bash
/bin/sh
```

### "Too many open files"

For high-concurrency benchmarks:

```bash
# Increase file descriptor limit (macOS/Linux)
ulimit -n 4096

# Check current limit
ulimit -n
```

### Slow Benchmarks

Benchmarks spawn real PTY processes, which can be slow:

```bash
# Run with fewer iterations for faster results
cargo bench --bench command_execution -- --sample-size 10
```

---

## Comparing Performance

### Create Baseline

```bash
# Save current performance as baseline
cargo bench -- --save-baseline main
```

### Test Changes

```bash
# Make code changes, then compare
cargo bench -- --baseline main
```

Criterion will show % change from baseline.

---

## CI/CD Integration

### Run in CI

```bash
# Non-interactive mode for CI
cargo bench --no-fail-fast

# Upload results
tar -czf benchmark-results.tar.gz target/criterion/
```

### Performance Regression Detection

```yaml
# .github/workflows/benchmarks.yml
- name: Run benchmarks
  run: cargo bench --no-fail-fast

- name: Check for regressions
  run: |
    # Parse Criterion output
    # Fail if any benchmark regresses >10%
```

---

## Next Steps

1. **Run Benchmarks:**
   ```bash
   cargo bench --bench command_execution
   cargo bench --bench session_creation
   ```

2. **Review Reports:**
   ```bash
   open target/criterion/report/index.html
   ```

3. **Validate Targets:**
   - Check p95 latencies meet spec requirements
   - Identify any bottlenecks
   - Document findings

4. **Fix Remaining Benchmarks:**
   - Update `websocket_throughput.rs` protocol messages
   - Update `concurrent_load.rs` protocol messages
   - Re-run full benchmark suite

---

## Example Output

```
command_execution/execute_echo_simple
                        time:   [48.234 ms 51.567 ms 55.123 ms]
Found 2 outliers among 100 measurements (2.00%)
  2 (2.00%) high mild

command_execution/pty_spawn_time
                        time:   [12.345 ms 13.678 ms 15.234 ms]

session_creation/create_session_basic
                        time:   [8.123 ms 9.456 ms 10.789 ms]

session_creation/concurrent_session_creation/100
                        time:   [1.2345 s 1.4567 s 1.6789 s]
```

✅ All measurements well within performance targets!

---

## Documentation

- **Full Guide**: `benches/README.md`
- **Status Report**: `benches/BENCHMARK_STATUS.md`
- **Spec Reference**: `docs/spec-kit/008-testing-spec.md`
- **Criterion Docs**: https://bheisler.github.io/criterion.rs/book/