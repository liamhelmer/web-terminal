// WebSocket Throughput Performance Benchmark
// Per spec-kit/008-testing-spec.md - Performance Tests
//
// Target: WebSocket latency < 20ms (p95)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use web_terminal::protocol::{ClientMessage, ServerMessage};
use web_terminal::pty::{PtyConfig, PtyManager, ShellConfig};
use web_terminal::session::{SessionConfig, SessionManager, SessionId, UserId};

/// Benchmark message serialization/deserialization
fn bench_message_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("websocket_message_serialization");

    // Test different message types
    let client_messages = vec![
        ("command", ClientMessage::Command {
            data: "echo 'test'".to_string(),
        }),
        ("input", ClientMessage::Input {
            data: "test input".to_string(),
        }),
        ("resize", ClientMessage::Resize {
            rows: 24,
            cols: 80,
        }),
        ("ping", ClientMessage::Ping),
    ];

    for (name, msg) in client_messages.iter() {
        group.bench_with_input(
            BenchmarkId::new("serialize_client", name),
            msg,
            |b, msg| {
                b.iter(|| {
                    let json = serde_json::to_string(black_box(msg)).unwrap();
                    black_box(json)
                });
            },
        );
    }

    let server_messages = vec![
        (
            "output",
            ServerMessage::Output {
                data: "test output\n".to_string(),
            },
        ),
        (
            "status",
            ServerMessage::Status {
                status: web_terminal::protocol::ConnectionStatus::Connected,
            },
        ),
        (
            "error",
            ServerMessage::Error {
                message: "test error".to_string(),
            },
        ),
        ("pong", ServerMessage::Pong),
    ];

    for (name, msg) in server_messages.iter() {
        group.bench_with_input(
            BenchmarkId::new("serialize_server", name),
            msg,
            |b, msg| {
                b.iter(|| {
                    let json = serde_json::to_string(black_box(msg)).unwrap();
                    black_box(json)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark message throughput (messages per second)
fn bench_message_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("websocket_message_throughput");

    for message_count in [10, 100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*message_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(message_count),
            message_count,
            |b, &count| {
                b.to_async(&rt).iter(|| async move {
                    let (tx, mut rx) = mpsc::unbounded_channel();

                    // Simulate sending messages
                    for i in 0..count {
                        let msg = ServerMessage::Output {
                            data: format!("Message {}\n", i),
                        };
                        tx.send(msg).unwrap();
                    }

                    // Simulate receiving and processing messages
                    let mut processed = 0;
                    while let Some(msg) = rx.recv().await {
                        let _json = serde_json::to_string(&msg).unwrap();
                        processed += 1;
                        if processed >= count {
                            break;
                        }
                    }

                    processed
                });
            },
        );
    }

    group.finish();
}

/// Benchmark latency for different message sizes
fn bench_message_latency_by_size(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("websocket_latency_by_message_size");

    for size in [64, 256, 1024, 4096, 16384].iter() {
        let data = "x".repeat(*size);
        group.throughput(Throughput::Bytes(*size as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &data,
            |b, data| {
                b.to_async(&rt).iter(|| async {
                    let msg = ServerMessage::Output {
                        data: data.clone(),
                    };

                    // Serialize
                    let json = serde_json::to_string(black_box(&msg)).unwrap();

                    // Deserialize (simulating round-trip)
                    let _decoded: ServerMessage = serde_json::from_str(&json).unwrap();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark round-trip latency (client -> server -> client)
fn bench_round_trip_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("websocket_round_trip_ping_pong", |b| {
        b.to_async(&rt).iter(|| async {
            // Simulate ping-pong round trip
            let ping = ClientMessage::Ping;
            let ping_json = serde_json::to_string(&ping).unwrap();

            // Simulate network serialization/deserialization
            let _decoded: ClientMessage = serde_json::from_str(&ping_json).unwrap();

            // Server response
            let pong = ServerMessage::Pong;
            let pong_json = serde_json::to_string(&pong).unwrap();

            let _decoded: ServerMessage = serde_json::from_str(&pong_json).unwrap();
        });
    });
}

/// Benchmark concurrent message processing
fn bench_concurrent_messages(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("websocket_concurrent_messages");

    for concurrent_count in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(concurrent_count),
            concurrent_count,
            |b, &count| {
                b.to_async(&rt).iter(|| async move {
                    let mut handles = Vec::new();

                    for i in 0..count {
                        let handle = tokio::spawn(async move {
                            let msg = ServerMessage::Output {
                                data: format!("Concurrent message {}\n", i),
                            };
                            serde_json::to_string(&msg).unwrap()
                        });
                        handles.push(handle);
                    }

                    // Wait for all to complete
                    for handle in handles {
                        let _ = handle.await.unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark PTY output streaming through WebSocket messages
fn bench_pty_output_streaming(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("pty_output_to_websocket_messages", |b| {
        b.to_async(&rt).iter(|| async {
            let pty_config = PtyConfig::default();
            let pty_manager = PtyManager::new(pty_config);

            let handle = pty_manager.spawn(None).unwrap();
            let pty_id = handle.id().to_string();

            // Execute command that produces output
            let writer = pty_manager.create_writer(&pty_id).unwrap();
            writer.write(b"echo 'benchmark test'\n").await.unwrap();

            // Simulate converting output to WebSocket messages
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;

            // Create mock WebSocket message
            let msg = ServerMessage::Output {
                data: "benchmark test\n".to_string(),
            };
            let _serialized = serde_json::to_string(&msg).unwrap();

            let _ = pty_manager.kill(&pty_id).await;
        });
    });
}

/// Benchmark batch message processing
fn bench_batch_message_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("websocket_batch_processing");

    for batch_size in [10, 50, 100, 500].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &size| {
                b.to_async(&rt).iter(|| async move {
                    // Create batch of messages
                    let messages: Vec<_> = (0..size)
                        .map(|i| ServerMessage::Output {
                            data: format!("Batch message {}\n", i),
                        })
                        .collect();

                    // Serialize all at once
                    let serialized: Vec<_> = messages
                        .iter()
                        .map(|msg| serde_json::to_string(msg).unwrap())
                        .collect();

                    serialized.len()
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_message_serialization,
    bench_message_throughput,
    bench_message_latency_by_size,
    bench_round_trip_latency,
    bench_concurrent_messages,
    bench_pty_output_streaming,
    bench_batch_message_processing
);
criterion_main!(benches);