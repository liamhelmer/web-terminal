// Command Execution Performance Benchmark
// Per spec-kit/008-testing-spec.md - Performance Tests
//
// Target: Command execution latency < 100ms (p95)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::sync::Arc;
use tokio::runtime::Runtime;
use web_terminal::pty::{PtyConfig, PtyManager, ShellConfig};
use web_terminal::session::{SessionConfig, SessionManager};

/// Benchmark basic echo command execution
fn bench_echo_command(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("execute_echo_simple", |b| {
        b.to_async(&rt).iter(|| async {
            let pty_config = PtyConfig::default();
            let pty_manager = PtyManager::new(pty_config);

            // Spawn PTY and execute command
            let handle = pty_manager.spawn(None).unwrap();
            let pty_id = handle.id().to_string();

            // Create writer and write command
            let writer = pty_manager.create_writer(&pty_id).unwrap();
            writer.write(black_box(b"echo 'benchmark'\n")).await.unwrap();

            // Wait briefly for output (realistic scenario)
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;

            // Cleanup
            let _ = pty_manager.kill(&pty_id).await;
        });
    });
}

/// Benchmark command execution with varying command lengths
fn bench_command_lengths(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("command_execution_by_length");

    for length in [10, 50, 100, 500, 1000].iter() {
        let command = format!("echo '{}'", "x".repeat(*length));

        group.throughput(Throughput::Bytes(*length as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(length),
            length,
            |b, _length| {
                b.to_async(&rt).iter(|| async {
                    let pty_config = PtyConfig::default();
                    let pty_manager = PtyManager::new(pty_config);

                    let handle = pty_manager.spawn(None).unwrap();
                    let pty_id = handle.id().to_string();

                    // Execute command
                    let writer = pty_manager.create_writer(&pty_id).unwrap();
                    let cmd = format!("{}\n", black_box(&command));
                    writer.write(cmd.as_bytes()).await.unwrap();

                    // Wait briefly for output
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

                    let _ = pty_manager.kill(&pty_id).await;
                });
            },
        );
    }

    group.finish();
}

/// Benchmark command execution with different shell types
fn bench_shell_types(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("command_execution_by_shell");

    let shells = vec![
        ("sh", ShellConfig::sh()),
        ("bash", ShellConfig::bash()),
    ];

    for (name, shell_config) in shells {
        group.bench_with_input(BenchmarkId::from_parameter(name), &shell_config, |b, shell_cfg| {
            b.to_async(&rt).iter(|| async {
                let pty_config = PtyConfig {
                    shell: shell_cfg.clone(),
                    ..PtyConfig::default()
                };
                let pty_manager = PtyManager::new(pty_config);

                let handle = pty_manager.spawn(None).unwrap();
                let pty_id = handle.id().to_string();

                // Execute command
                let writer = pty_manager.create_writer(&pty_id).unwrap();
                writer.write(black_box(b"echo 'test'\n")).await.unwrap();

                // Wait briefly for output
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;

                let _ = pty_manager.kill(&pty_id).await;
            });
        });
    }

    group.finish();
}

/// Benchmark PTY spawn time (part of command execution)
fn bench_pty_spawn(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("pty_spawn_time", |b| {
        b.to_async(&rt).iter(|| async {
            let pty_config = PtyConfig::default();
            let pty_manager = PtyManager::new(pty_config);

            let handle = pty_manager.spawn(None).unwrap();
            let pty_id = handle.id().to_string();

            // Cleanup
            let _ = pty_manager.kill(&pty_id).await;
        });
    });
}

/// Benchmark sequential command execution (realistic workload)
fn bench_sequential_commands(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("execute_10_sequential_commands", |b| {
        b.to_async(&rt).iter(|| async {
            let pty_config = PtyConfig::default();
            let pty_manager = PtyManager::new(pty_config);

            let handle = pty_manager.spawn(None).unwrap();
            let pty_id = handle.id().to_string();

            let writer = pty_manager.create_writer(&pty_id).unwrap();

            // Execute 10 commands sequentially
            for i in 0..10 {
                let cmd = format!("echo 'command {}'\n", i);
                writer.write(cmd.as_bytes()).await.unwrap();

                // Wait briefly between commands
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }

            let _ = pty_manager.kill(&pty_id).await;
        });
    });
}

criterion_group!(
    benches,
    bench_echo_command,
    bench_command_lengths,
    bench_shell_types,
    bench_pty_spawn,
    bench_sequential_commands
);
criterion_main!(benches);