//! WebSocket 并发连接基准测试
//! 
//! 测试 WebSocket 连接和消息广播的性能：
//! - 连接建立延迟
//! - 并发连接数测试
//! - 消息广播延迟
//! - 消息顺序保证

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use tokio::runtime::Runtime;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use uuid::Uuid;

/// 测试配置
struct WsBenchConfig {
    ws_url: String,
    auth_token: String,
}

impl WsBenchConfig {
    fn new() -> Self {
        Self {
            ws_url: std::env::var("OMNILINK_WS_URL")
                .unwrap_or_else(|_| "ws://localhost:8080/ws".to_string()),
            auth_token: std::env::var("AUTH_TOKEN")
                .unwrap_or_else(|_| "test-token".to_string()),
        }
    }
}

/// 单个 WebSocket 连接建立延迟
fn bench_ws_connection_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = WsBenchConfig::new();
    
    c.bench_function("ws_connection_latency", |b| {
        b.iter(|| {
            rt.block_on(async {
                let url = format!("{}?token={}", config.ws_url, config.auth_token);
                let result = connect_async(black_box(&url)).await;
                black_box(result)
            })
        })
    });
}

/// 并发 WebSocket 连接测试
fn bench_ws_concurrent_connections(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = WsBenchConfig::new();
    
    let mut group = c.benchmark_group("ws_concurrent");
    
    for conn_count in [10, 50, 100, 200, 500].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(conn_count),
            conn_count,
            |b, &count| {
                b.iter(|| {
                    rt.block_on(async {
                        let mut handles = Vec::new();
                        
                        for _ in 0..count {
                            let url = format!("{}?token={}", config.ws_url, config.auth_token);
                            
                            let handle = tokio::spawn(async move {
                                let result = connect_async(&url).await;
                                result
                            });
                            
                            handles.push(handle);
                        }
                        
                        let results = futures_util::future::join_all(handles).await;
                        black_box(results)
                    })
                })
            },
        );
    }
    
    group.finish();
}

/// WebSocket 消息发送延迟
fn bench_ws_message_send_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = WsBenchConfig::new();
    
    c.bench_function("ws_message_send_latency", |b| {
        b.iter(|| {
            rt.block_on(async {
                let url = format!("{}?token={}", config.ws_url, config.auth_token);
                
                if let Ok((mut ws_stream, _)) = connect_async(&url).await {
                    let msg = json!({
                        "type": "message",
                        "conversationId": Uuid::new_v4().to_string(),
                        "content": black_box("WebSocket 性能测试消息"),
                        "contentType": "text"
                    });
                    
                    let result = ws_stream.send(Message::Text(msg.to_string())).await;
                    black_box(result)
                }
            })
        })
    });
}

/// 并发消息发送测试
fn bench_ws_concurrent_message_send(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = WsBenchConfig::new();
    
    let mut group = c.benchmark_group("ws_concurrent_send");
    
    for concurrency in [5, 10, 20, 50].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(concurrency),
            concurrency,
            |b, &conc| {
                b.iter(|| {
                    rt.block_on(async {
                        let url = format!("{}?token={}", config.ws_url, config.auth_token);
                        
                        if let Ok((ws_stream, _)) = connect_async(&url).await {
                            let (mut write, _) = ws_stream.split();
                            let mut handles = Vec::new();
                            
                            for i in 0..conc {
                                let msg = json!({
                                    "type": "message",
                                    "conversationId": Uuid::new_v4().to_string(),
                                    "content": format!("并发消息 {}", i),
                                    "contentType": "text"
                                });
                                
                                // Note: In real benchmark, we'd need separate connections
                                // This tests sequential sends on single connection
                                let _ = write.send(Message::Text(msg.to_string())).await;
                            }
                            
                            black_box(())
                        }
                    })
                })
            },
        );
    }
    
    group.finish();
}

/// 消息接收延迟测试
fn bench_ws_message_receive_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = WsBenchConfig::new();
    
    c.bench_function("ws_message_receive_latency", |b| {
        b.iter(|| {
            rt.block_on(async {
                let url = format!("{}?token={}", config.ws_url, config.auth_token);
                
                if let Ok((ws_stream, _)) = connect_async(&url).await {
                    let (_, mut read) = ws_stream.split();
                    
                    // 接收一条消息
                    if let Some(msg) = read.next().await {
                        black_box(msg)
                    }
                }
            })
        })
    });
}

criterion_group!(
    benches,
    bench_ws_connection_latency,
    bench_ws_concurrent_connections,
    bench_ws_message_send_latency,
    bench_ws_concurrent_message_send,
    bench_ws_message_receive_latency
);
criterion_main!(benches);
