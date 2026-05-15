//! 数据库查询性能基准测试
//! 
//! 测试常见数据库操作的性能：
//! - 消息插入性能
//! - 消息查询性能（分页、过滤）
//! - 会话列表查询
//! - 用户搜索性能
//! - 聚合查询性能

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use tokio::runtime::Runtime;
use reqwest::Client;
use serde_json::json;
use uuid::Uuid;

/// 测试配置
struct DbBenchConfig {
    base_url: String,
    auth_token: String,
}

impl DbBenchConfig {
    fn new() -> Self {
        Self {
            base_url: std::env::var("OMNILINK_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            auth_token: std::env::var("AUTH_TOKEN")
                .unwrap_or_else(|_| "test-token".to_string()),
        }
    }
}

/// 会话列表查询性能
fn bench_conversation_list_query(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = DbBenchConfig::new();
    let client = Client::new();
    
    let mut group = c.benchmark_group("conversation_list");
    
    for page_size in [20, 50, 100, 200].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(page_size),
            page_size,
            |b, &size| {
                b.iter(|| {
                    rt.block_on(async {
                        let response = client
                            .get(format!(
                                "{}/api/im/conversations?limit={}",
                                config.base_url, size
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

/// 消息历史查询性能
fn bench_message_history_query(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = DbBenchConfig::new();
    let client = Client::new();
    let conversation_id = Uuid::new_v4().to_string();
    
    let mut group = c.benchmark_group("message_history");
    
    // 不同分页大小
    for page_size in [20, 50, 100, 200].iter() {
        group.bench_with_input(
            BenchmarkId::new("page_size", page_size),
            page_size,
            |b, &size| {
                b.iter(|| {
                    rt.block_on(async {
                        let response = client
                            .get(format!(
                                "{}/api/im/conversations/{}/messages?limit={}",
                                config.base_url, conversation_id, size
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
    
    // 不同偏移量
    for offset in [0, 100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("offset", offset),
            offset,
            |b, &off| {
                b.iter(|| {
                    rt.block_on(async {
                        let response = client
                            .get(format!(
                                "{}/api/im/conversations/{}/messages?limit=50&offset={}",
                                config.base_url, conversation_id, off
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

/// 消息搜索性能
fn bench_message_search(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = DbBenchConfig::new();
    let client = Client::new();
    
    let mut group = c.benchmark_group("message_search");
    
    let search_terms = ["hello", "test", "重要", "会议", "通知"];
    
    for term in search_terms.iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(term),
            term,
            |b, &search| {
                b.iter(|| {
                    rt.block_on(async {
                        let response = client
                            .get(format!(
                                "{}/api/im/messages/search?q={}",
                                config.base_url, search
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

/// 全局消息搜索性能
fn bench_global_search(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = DbBenchConfig::new();
    let client = Client::new();
    
    c.bench_function("global_search", |b| {
        b.iter(|| {
            rt.block_on(async {
                let response = client
                    .get(format!(
                        "{}/api/im/messages/search/global?q=test",
                        config.base_url
                    ))
                    .header("Authorization", format!("Bearer {}", config.auth_token))
                    .send()
                    .await;
                
                black_box(response)
            })
        })
    });
}

/// 用户状态查询性能
fn bench_user_status_query(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = DbBenchConfig::new();
    let client = Client::new();
    
    let mut group = c.benchmark_group("user_status");
    
    // 单个用户状态查询
    c.bench_function("single_user_status", |b| {
        b.iter(|| {
            rt.block_on(async {
                let user_id = Uuid::new_v4().to_string();
                let response = client
                    .get(format!(
                        "{}/api/im/presence/{}",
                        config.base_url, user_id
                    ))
                    .header("Authorization", format!("Bearer {}", config.auth_token))
                    .send()
                    .await;
                
                black_box(response)
            })
        })
    });
    
    // 批量用户状态查询
    for batch_size in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("batch", batch_size),
            batch_size,
            |b, &size| {
                b.iter(|| {
                    rt.block_on(async {
                        let user_ids: Vec<String> = (0..size)
                            .map(|_| Uuid::new_v4().to_string())
                            .collect();
                        
                        let response = client
                            .post(format!(
                                "{}/api/im/presence/batch",
                                config.base_url
                            ))
                            .header("Authorization", format!("Bearer {}", config.auth_token))
                            .json(&json!({
                                "userIds": user_ids
                            }))
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

/// 已读状态查询性能
fn bench_read_status_query(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = DbBenchConfig::new();
    let client = Client::new();
    
    c.bench_function("delivery_receipts", |b| {
        b.iter(|| {
            rt.block_on(async {
                let message_id = Uuid::new_v4().to_string();
                let response = client
                    .get(format!(
                        "{}/api/im/messages/{}/receipts",
                        config.base_url, message_id
                    ))
                    .header("Authorization", format!("Bearer {}", config.auth_token))
                    .send()
                    .await;
                
                black_box(response)
            })
        })
    });
}

criterion_group!(
    benches,
    bench_conversation_list_query,
    bench_message_history_query,
    bench_message_search,
    bench_global_search,
    bench_user_status_query,
    bench_read_status_query
);
criterion_main!(benches);
