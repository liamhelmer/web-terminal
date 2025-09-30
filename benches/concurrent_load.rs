// Concurrent Load Performance Benchmark
// Per spec-kit/008-testing-spec.md - Performance Tests
//
// Target: Support 10,000 concurrent sessions

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::sync::Arc;
use tokio::runtime::Runtime;
use web_terminal::pty::{PtyConfig, PtyManager, ShellConfig};
use web_terminal::session::{SessionConfig, SessionManager, UserId};

/// Benchmark concurrent session creation at scale
fn bench_concurrent_sessions_at_scale(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_sessions_scale");
    group.sample_size(10); // Reduced sample size for heavy benchmarks

    for count in [100, 500, 1000, 2000].iter() {
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            count,
            |b, &session_count| {
                b.to_async(&rt).iter(|| async move {
                    let config = SessionConfig::default();
                    let manager = Arc::new(SessionManager::new(config));

                    let mut handles = Vec::new();

                    // Create sessions concurrently
                    for i in 0..session_count {
                        let manager_clone = Arc::clone(&manager);
                        let handle = tokio::spawn(async move {
                            let user_id = UserId::new(format!("user_{}", i));
                            manager_clone.create_session(user_id).await
                        });
                        handles.push(handle);
                    }

                    // Wait for all to complete
                    let results = futures_util::future::join_all(handles).await;
                    let sessions: Vec<_> = results
                        .into_iter()
                        .filter_map(|r| r.ok())
                        .filter_map(|r| r.ok())
                        .collect();

                    // Cleanup
                    let cleanup_handles: Vec<_> = sessions
                        .iter()
                        .map(|session| {
                            let manager_clone = Arc::clone(&manager);
                            let session_id = session.id.clone();
                            tokio::spawn(
                                async move { manager_clone.destroy_session(&session_id).await },
                            )
                        })
                        .collect();

                    futures_util::future::join_all(cleanup_handles).await;

                    sessions.len()
                });
            },
        );
    }

    group.finish();
}

/// Benchmark concurrent command execution across multiple sessions
fn bench_concurrent_command_execution(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_command_execution");
    group.sample_size(10);

    for count in [10, 50, 100].iter() {
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            count,
            |b, &session_count| {
                b.to_async(&rt).iter(|| async move {
                    let pty_config = PtyConfig::default();

                    // Create managers sequentially but execute concurrently
                    let mut managers = Vec::new();
                    let mut ids = Vec::new();

                    for _i in 0..session_count {
                        let pty_manager = PtyManager::new(pty_config.clone());
                        let pty_handle = pty_manager.spawn(None).unwrap();
                        let pty_id = pty_handle.id().to_string();
                        managers.push(pty_manager);
                        ids.push(pty_id);
                    }

                    // Execute commands sequentially (benchmarking the execution time)
                    for (i, (manager, id)) in managers.iter().zip(ids.iter()).enumerate() {
                        let writer = manager.create_writer(id).unwrap();
                        let cmd = format!("echo 'session {}'\n", i);
                        let _ = writer.write(cmd.as_bytes()).await;
                    }

                    // Wait briefly
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

                    // Cleanup
                    for (manager, id) in managers.iter().zip(ids.iter()) {
                        let _ = manager.kill(id).await;
                    }

                    session_count
                });
            },
        );
    }

    group.finish();
}

/// Benchmark mixed workload (reads and writes)
fn bench_mixed_workload(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("mixed_workload");
    group.sample_size(10);

    for session_count in [50, 100, 200].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(session_count),
            session_count,
            |b, &count| {
                b.to_async(&rt).iter(|| async move {
                    let session_config = SessionConfig::default();
                    let manager = Arc::new(SessionManager::new(session_config));

                    let mut handles = Vec::new();

                    for i in 0..count {
                        let manager_clone = Arc::clone(&manager);
                        let handle = tokio::spawn(async move {
                            // Create session
                            let user_id = UserId::new(format!("user_{}", i));
                            let session = manager_clone.create_session(user_id).await.ok()?;

                            // Simulate mixed operations
                            session.update_last_activity().await;
                            let _state = session.state().await;
                            session.set_pty(format!("pty_{}", i)).await;

                            // Lookup session
                            let _lookup = manager_clone.get_session(&session.id).await.ok()?;

                            Some(session.id.clone())
                        });
                        handles.push(handle);
                    }

                    // Wait for all operations
                    let results = futures_util::future::join_all(handles).await;
                    let session_ids: Vec<_> = results
                        .into_iter()
                        .filter_map(|r| r.ok())
                        .flatten()
                        .collect();

                    // Cleanup
                    for session_id in &session_ids {
                        let _ = manager.destroy_session(session_id).await;
                    }

                    session_ids.len()
                });
            },
        );
    }

    group.finish();
}

/// Benchmark memory usage under load
fn bench_memory_under_load(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("memory_usage_1000_sessions", |b| {
        b.to_async(&rt).iter(|| async {
            let config = SessionConfig::default();
            let manager = Arc::new(SessionManager::new(config));

            // Create 1000 sessions
            let mut handles = Vec::new();
            for i in 0..1000 {
                let manager_clone = Arc::clone(&manager);
                let handle = tokio::spawn(async move {
                    let user_id = UserId::new(format!("user_{}", i));
                    manager_clone.create_session(user_id).await
                });
                handles.push(handle);
            }

            let results = futures_util::future::join_all(handles).await;
            let sessions: Vec<_> = results
                .into_iter()
                .filter_map(|r| r.ok())
                .filter_map(|r| r.ok())
                .collect();

            // Hold sessions for a moment to measure steady-state memory
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;

            // Cleanup
            for session in &sessions {
                let _ = manager.destroy_session(&session.id).await;
            }

            sessions.len()
        });
    });
}

/// Benchmark connection churn (create + destroy rapidly)
fn bench_connection_churn(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("connection_churn");

    for ops_count in [100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*ops_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(ops_count),
            ops_count,
            |b, &count| {
                b.to_async(&rt).iter(|| async move {
                    let config = SessionConfig::default();
                    let manager = Arc::new(SessionManager::new(config));

                    for i in 0..count {
                        // Create session
                        let user_id = UserId::new(format!("user_{}", i));
                        if let Ok(session) = manager.create_session(user_id).await {
                            // Immediately destroy
                            let _ = manager.destroy_session(&session.id).await;
                        }
                    }

                    count
                });
            },
        );
    }

    group.finish();
}

/// Benchmark sustained throughput over time
fn bench_sustained_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("sustained_throughput_100_sessions_10s", |b| {
        b.to_async(&rt).iter(|| async {
            let session_config = SessionConfig::default();
            let manager = Arc::new(SessionManager::new(session_config));

            // Create 100 sessions
            let mut sessions = Vec::new();
            for i in 0..100 {
                let user_id = UserId::new(format!("user_{}", i));
                if let Ok(session) = manager.create_session(user_id).await {
                    sessions.push(session);
                }
            }

            // Simulate sustained activity
            let start = std::time::Instant::now();
            let mut operations = 0;

            while start.elapsed() < std::time::Duration::from_secs(1) {
                // Random operations on random sessions
                for session in &sessions {
                    session.update_last_activity().await;
                    let _state = session.state().await;
                    operations += 2;
                }
            }

            // Cleanup
            for session in &sessions {
                let _ = manager.destroy_session(&session.id).await;
            }

            operations
        });
    });
}

/// Benchmark resource cleanup efficiency
fn bench_resource_cleanup(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("cleanup_1000_sessions", |b| {
        b.to_async(&rt).iter_with_setup(
            || {
                // Setup: Create 1000 sessions
                let rt = Runtime::new().unwrap();
                rt.block_on(async {
                    let config = SessionConfig::default();
                    let manager = Arc::new(SessionManager::new(config));

                    let mut sessions = Vec::new();
                    for i in 0..1000 {
                        let user_id = UserId::new(format!("user_{}", i));
                        if let Ok(session) = manager.create_session(user_id).await {
                            sessions.push(session.id.clone());
                        }
                    }

                    (manager, sessions)
                })
            },
            |(manager, sessions)| async move {
                // Benchmark: Cleanup all sessions
                let cleanup_handles: Vec<_> = sessions
                    .iter()
                    .map(|session_id| {
                        let manager_clone = Arc::clone(&manager);
                        let id = session_id.clone();
                        tokio::spawn(async move { manager_clone.destroy_session(&id).await })
                    })
                    .collect();

                futures_util::future::join_all(cleanup_handles).await;
            },
        );
    });
}

criterion_group!(
    benches,
    bench_concurrent_sessions_at_scale,
    bench_concurrent_command_execution,
    bench_mixed_workload,
    bench_memory_under_load,
    bench_connection_churn,
    bench_sustained_throughput,
    bench_resource_cleanup
);
criterion_main!(benches);
