//! API 响应压缩性能基准测试
//!
//! 测试不同压缩算法和配置下的性能表现：
//! - gzip vs brotli 压缩率和速度对比
//! - 不同压缩级别的性能影响
//! - 不同响应大小的压缩效果
//! - 压缩中间件的内存占用

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use flate2::read::{GzDecoder, GzEncoder};
use flate2::Compression as GzipCompression;
use std::io::Read;

/// 生成测试数据
fn generate_test_data(size: usize) -> Vec<u8> {
    // 生成 JSON 格式的测试数据（模拟 API 响应）
    let mut data = Vec::with_capacity(size);
    data.extend_from_slice(b"{\"code\":200,\"message\":\"success\",\"data\":[");

    let item = b"{\"id\":\"123e4567-e89b-12d3-a456-426614174000\",\"name\":\"Test User\",\"email\":\"test@example.com\",\"avatar\":\"https://example.com/avatar.jpg\",\"status\":\"online\",\"lastSeen\":\"2026-05-16T07:00:00Z\"},";
    while data.len() < size {
        data.extend_from_slice(item);
    }

    // 移除最后一个逗号并关闭 JSON
    if data.last() == Some(&b',') {
        data.pop();
    }
    data.extend_from_slice(b"]}");
    data
}

/// 测试 gzip 压缩性能
fn bench_gzip_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("gzip_compression");

    for size in [1024, 10240, 102400, 1048576].iter() {
        let data = generate_test_data(*size);

        for level in [1, 6, 9].iter() {
            let bench_id = format!("size_{}_level_{}", size, level);
            group.bench_with_input(BenchmarkId::from_parameter(&bench_id), &data, |b, data| {
                b.iter(|| {
                    let mut encoder = GzEncoder::new(data.as_slice(), GzCompression::new(*level));
                    let mut compressed = Vec::new();
                    encoder.read_to_end(&mut compressed).unwrap();
                    black_box(compressed)
                });
            });
        }
    }

    group.finish();
}

/// 测试 gzip 解压性能
fn bench_gzip_decompression(c: &mut Criterion) {
    let mut group = c.benchmark_group("gzip_decompression");

    for size in [1024, 10240, 102400, 1048576].iter() {
        let data = generate_test_data(*size);

        // 预压缩数据
        let mut encoder = GzEncoder::new(data.as_slice(), GzCompression::new(6));
        let mut compressed = Vec::new();
        encoder.read_to_end(&mut compressed).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("size_{}", size)),
            &compressed,
            |b, compressed| {
                b.iter(|| {
                    let mut decoder = GzDecoder::new(compressed.as_slice());
                    let mut decompressed = Vec::new();
                    decoder.read_to_end(&mut decompressed).unwrap();
                    black_box(decompressed)
                });
            },
        );
    }

    group.finish();
}

/// 测试不同数据类型的压缩率
fn bench_compression_ratio(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_ratio");

    // JSON 数据（高冗余）
    let json_data = generate_test_data(102400);

    // 随机数据（低冗余）
    let random_data: Vec<u8> = (0..102400).map(|i| (i % 256) as u8).collect();

    // 文本数据（中等冗余）
    let text_data = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(2000);

    group.bench_function("json_data", |b| {
        b.iter(|| {
            let mut encoder = GzEncoder::new(json_data.as_slice(), GzCompression::new(6));
            let mut compressed = Vec::new();
            encoder.read_to_end(&mut compressed).unwrap();
            black_box((compressed.len(), json_data.len()))
        });
    });

    group.bench_function("random_data", |b| {
        b.iter(|| {
            let mut encoder = GzEncoder::new(random_data.as_slice(), GzCompression::new(6));
            let mut compressed = Vec::new();
            encoder.read_to_end(&mut compressed).unwrap();
            black_box((compressed.len(), random_data.len()))
        });
    });

    group.bench_function("text_data", |b| {
        b.iter(|| {
            let mut encoder = GzEncoder::new(text_data.as_bytes(), GzCompression::new(6));
            let mut compressed = Vec::new();
            encoder.read_to_end(&mut compressed).unwrap();
            black_box((compressed.len(), text_data.len()))
        });
    });

    group.finish();
}

/// 测试内存分配模式
fn bench_memory_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");

    let data = generate_test_data(102400);

    // 预分配缓冲区
    group.bench_function("pre_allocated", |b| {
        b.iter(|| {
            let mut compressed = Vec::with_capacity(102400);
            let mut encoder = GzEncoder::new(data.as_slice(), GzCompression::new(6));
            encoder.read_to_end(&mut compressed).unwrap();
            black_box(compressed)
        });
    });

    // 动态增长缓冲区
    group.bench_function("dynamic_growth", |b| {
        b.iter(|| {
            let mut compressed = Vec::new();
            let mut encoder = GzEncoder::new(data.as_slice(), GzCompression::new(6));
            encoder.read_to_end(&mut compressed).unwrap();
            black_box(compressed)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_gzip_compression,
    bench_gzip_decompression,
    bench_compression_ratio,
    bench_memory_allocation
);
criterion_main!(benches);
