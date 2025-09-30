// Session Creation Performance Benchmark
// Per spec-kit/008-testing-spec.md - Performance Tests
//
// Target: Session creation latency < 200ms (p95)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::sync::Arc;
use tokio::runtime::Runtime;
use web_terminal::pty::{PtyConfig, PtyManager, ShellConfig};
use web_terminal::session::{SessionConfig, SessionId, SessionManager, UserId};

/// Benchmark basic session creation
fn bench_session_creation_basic(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("create_session_basic", |b| {
        b.to_async(&rt).iter(|| async {
            let config = SessionConfig::default();
            let manager = SessionManager::new(config);

            let user_id = UserId::new(black_box("benchmark_user".to_string()));
            let session = manager.create_session(user_id).await.unwrap();

            // Cleanup
            let _ = manager.destroy_session(&session.id).await;

            session
        });
    });
}

/// Benchmark session creation with different user counts
fn bench_session_creation_multi_user(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("session_creation_multi_user");

    for user_count in [1, 5, 10].iter() {
        group.throughput(Throughput::Elements(*user_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(user_count),
            user_count,
            |b, &count| {
                b.to_async(&rt).iter(|| async move {
                    let config = SessionConfig::default();
                    let manager = SessionManager::new(config);

                    let mut sessions = Vec::new();
                    for i in 0..count {
                        let user_id = UserId::new(format!("user_{}", i));
                        if let Ok(session) = manager.create_session(user_id).await {
                            sessions.push(session);
                        }
                    }

                    // Cleanup
                    for session in &sessions {
                        let _ = manager.destroy_session(&session.id).await;
                    }

                    sessions.len()
                });
            },
        );
    }

    group.finish();
}

/// Benchmark session lookup performance
fn bench_session_lookup(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("session_lookup", |b| {
        b.to_async(&rt).iter_with_setup(
            || {
                // Setup: Create sessions
                let rt = Runtime::new().unwrap();
                rt.block_on(async {
                    let config = SessionConfig::default();
                    let manager = SessionManager::new(config);
                    let user_id = UserId::new("benchmark_user".to_string());
                    let session = manager.create_session(user_id).await.unwrap();
                    (manager, session.id.clone())
                })
            },
            |(manager, session_id)| async move {
                // Benchmark: Look up session
                let result = manager.get_session(&session_id).await;
                let _ = manager.destroy_session(&session_id).await;
                result
            },
        );
    });
}

/// Benchmark session state operations
fn bench_session_state_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("session_state_operations", |b| {
        b.to_async(&rt).iter_with_setup(
            || {
                // Setup: Create session
                let rt = Runtime::new().unwrap();
                rt.block_on(async {
                    let config = SessionConfig::default();
                    let manager = SessionManager::new(config);
                    let user_id = UserId::new("benchmark_user".to_string());
                    let session = manager.create_session(user_id).await.unwrap();
                    (manager, session)
                })
            },
            |(manager, session)| async move {
                // Benchmark: Update session state
                session.set_pty("pty_test_id".to_string()).await;
                session.add_to_history("echo 'test'".to_string()).await;
                let _env = session.get_environment().await;

                let _ = manager.destroy_session(&session.id).await;
            },
        );
    });
}

/// Benchmark session destruction
fn bench_session_destruction(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("destroy_session", |b| {
        b.to_async(&rt).iter_with_setup(
            || {
                // Setup: Create session
                let rt = Runtime::new().unwrap();
                rt.block_on(async {
                    let config = SessionConfig::default();
                    let manager = SessionManager::new(config);
                    let user_id = UserId::new("benchmark_user".to_string());
                    let session = manager.create_session(user_id).await.unwrap();
                    (manager, session.id.clone())
                })
            },
            |(manager, session_id)| async move {
                // Benchmark: Destroy session
                manager.destroy_session(&session_id).await
            },
        );
    });
}

/// Benchmark session lifecycle (create + use + destroy)
fn bench_session_full_lifecycle(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("session_full_lifecycle", |b| {
        b.to_async(&rt).iter(|| async {
            let config = SessionConfig::default();
            let manager = SessionManager::new(config);

            // Create
            let user_id = UserId::new("benchmark_user".to_string());
            let session = manager.create_session(user_id).await.unwrap();

            // Use (update state)
            session
                .set_env("TEST".to_string(), "value".to_string())
                .await;
            let _env = session.get_environment().await;

            // Destroy
            manager.destroy_session(&session.id).await.unwrap();
        });
    });
}

/// Benchmark concurrent session creation
fn bench_concurrent_session_creation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_session_creation");

    for count in [10, 50, 100].iter() {
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            count,
            |b, &session_count| {
                b.to_async(&rt).iter(|| async move {
                    let config = SessionConfig::default();
                    let manager = Arc::new(SessionManager::new(config));

                    let mut handles = Vec::new();

                    for i in 0..session_count {
                        let manager_clone = Arc::clone(&manager);
                        let handle = tokio::spawn(async move {
                            let user_id = UserId::new(format!("user_{}", i));
                            manager_clone.create_session(user_id).await
                        });
                        handles.push(handle);
                    }

                    // Wait for all sessions to be created
                    let sessions: Vec<_> = futures_util::future::join_all(handles)
                        .await
                        .into_iter()
                        .filter_map(|r| r.ok())
                        .filter_map(|r| r.ok())
                        .collect();

                    // Cleanup
                    for session in &sessions {
                        let _ = manager.destroy_session(&session.id).await;
                    }

                    sessions.len()
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_session_creation_basic,
    bench_session_creation_multi_user,
    bench_session_lookup,
    bench_session_state_operations,
    bench_session_destruction,
    bench_session_full_lifecycle,
    bench_concurrent_session_creation
);
criterion_main!(benches);
