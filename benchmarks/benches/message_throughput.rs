//! 消息发送吞吐量基准测试
//! 
//! 测试不同场景下消息发送的性能表现：
//! - 单条消息发送延迟
//! - 批量消息发送吞吐量
//! - 不同消息大小的影响
//! - 并发消息发送性能

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use tokio::runtime::Runtime;
use reqwest::Client;
use serde_json::{json, Value};
use uuid::Uuid;

/// 测试配置
struct BenchConfig {
    base_url: String,
    auth_token: String,
    conversation_id: String,
}

impl BenchConfig {
    fn new() -> Self {
        Self {
            base_url: std::env::var("OMNILINK_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            auth_token: std::env::var("AUTH_TOKEN")
                .unwrap_or_else(|_| "test-token".to_string()),
            conversation_id: std::env::var("CONVERSATION_ID")
                .unwrap_or_else(|_| Uuid::new_v4().to_string()),
        }
    }
}

/// 单条消息发送延迟测试
fn bench_single_message_send(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = BenchConfig::new();
    let client = Client::new();
    
    c.bench_function("single_message_send", |b| {
        b.iter(|| {
            rt.block_on(async {
                let payload = json!({
                    "conversationId": config.conversation_id,
                    "content": black_box("性能测试消息内容"),
                    "contentType": "text",
                    "metadata": {}
                });
                
                let response = client
                    .post(format!("{}/api/im/messages", config.base_url))
                    .header("Authorization", format!("Bearer {}", config.auth_token))
                    .json(&payload)
                    .send()
                    .await;
                
                black_box(response)
            })
        })
    });
}

/// 不同消息大小的性能影响
fn bench_message_size_impact(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = BenchConfig::new();
    let client = Client::new();
    
    let mut group = c.benchmark_group("message_size");
    
    for size in [10, 100, 500, 1000, 5000].iter() {
        let content = "A".repeat(*size);
        
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                rt.block_on(async {
                    let payload = json!({
                        "conversationId": config.conversation_id,
                        "content": black_box(&content),
                        "contentType": "text",
                        "metadata": {}
                    });
                    
                    let response = client
                        .post(format!("{}/api/im/messages", config.base_url))
                        .header("Authorization", format!("Bearer {}", config.auth_token))
                        .json(&payload)
                        .send()
                        .await;
                    
                    black_box(response)
                })
            })
        });
    }
    
    group.finish();
}

/// 批量消息发送吞吐量测试
fn bench_batch_message_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = BenchConfig::new();
    let client = Client::new();
    
    let mut group = c.benchmark_group("batch_throughput");
    
    for batch_size in [10, 50, 100, 200].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &size| {
                b.iter(|| {
                    rt.block_on(async {
                        let messages: Vec<Value> = (0..size)
                            .map(|i| {
                                json!({
                                    "conversationId": config.conversation_id,
                                    "content": format!("批量消息 {}", i),
                                    "contentType": "text",
                                    "metadata": {}
                                })
                            })
                            .collect();
                        
                        let payload = json!({
                            "messages": messages
                        });
                        
                        let response = client
                            .post(format!("{}/api/im/messages/batch", config.base_url))
                            .header("Authorization", format!("Bearer {}", config.auth_token))
                            .json(&payload)
                            .send()
                            .await;
                        
                        black_box(response)
                    })
                })
            },
        );
    }
    
    group.finish();
}

/// 并发消息发送性能测试
fn bench_concurrent_message_send(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = BenchConfig::new();
    
    let mut group = c.benchmark_group("concurrent_send");
    
    for concurrency in [1, 5, 10, 20, 50].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(concurrency),
            concurrency,
            |b, &conc| {
                b.iter(|| {
                    rt.block_on(async {
                        let mut handles = Vec::new();
                        
                        for i in 0..conc {
                            let client = Client::new();
                            let url = config.base_url.clone();
                            let token = config.auth_token.clone();
                            let conv_id = config.conversation_id.clone();
                            
                            let handle = tokio::spawn(async move {
                                let payload = json!({
                                    "conversationId": conv_id,
                                    "content": format!("并发消息 {}", i),
                                    "contentType": "text",
                                    "metadata": {}
                                });
                                
                                client
                                    .post(format!("{}/api/im/messages", url))
                                    .header("Authorization", format!("Bearer {}", token))
                                    .json(&payload)
                                    .send()
                                    .await
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

/// 历史消息查询性能测试
fn bench_message_history_query(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = BenchConfig::new();
    let client = Client::new();
    
    let mut group = c.benchmark_group("message_history");
    
    for page_size in [20, 50, 100, 200].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(page_size),
            page_size,
            |b, &size| {
                b.iter(|| {
                    rt.block_on(async {
                        let response = client
                            .get(format!(
                                "{}/api/im/conversations/{}/messages?limit={}",
                                config.base_url, config.conversation_id, size
                            ))
                            .header("Authorization", format!("Bearer {}", config.auth_token))
                            .send()
                            .await;
                        
                        black_box(response)
                    })
                })
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_single_message_send,
    bench_message_size_impact,
    bench_batch_message_throughput,
    bench_concurrent_message_send,
    bench_message_history_query
);
criterion_main!(benches);
