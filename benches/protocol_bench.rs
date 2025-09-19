//! Criterion benchmarks for MiniBit
//!
//! Run with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use minibit::messages::{quote, trade};

fn bench_trade_encode(c: &mut Criterion) {
    let mut buf = vec![0u8; 1024];

    c.bench_function("trade_encode_minimal", |b| {
        b.iter(|| {
            let size = trade::encode(
                black_box(&mut buf),
                black_box(12345),
                black_box(1_700_000_000_000_000_000),
                black_box(50_000_000),
                black_box(100),
                black_box(None),
                black_box(None),
            )
            .unwrap();
            black_box(size);
        });
    });

    c.bench_function("trade_encode_with_symbol", |b| {
        b.iter(|| {
            let size = trade::encode(
                black_box(&mut buf),
                black_box(12345),
                black_box(1_700_000_000_000_000_000),
                black_box(50_000_000),
                black_box(100),
                black_box(Some(b"AAPL")),
                black_box(None),
            )
            .unwrap();
            black_box(size);
        });
    });

    c.bench_function("trade_encode_full", |b| {
        b.iter(|| {
            let size = trade::encode(
                black_box(&mut buf),
                black_box(12345),
                black_box(1_700_000_000_000_000_000),
                black_box(50_000_000),
                black_box(100),
                black_box(Some(b"AAPL")),
                black_box(Some(b"Buy order")),
            )
            .unwrap();
            black_box(size);
        });
    });
}

fn bench_trade_decode(c: &mut Criterion) {
    // Pre-encode test frames
    let mut buf = vec![0u8; 1024];

    let minimal_size = trade::encode(
        &mut buf,
        1,
        1_700_000_000_000_000_000,
        50_000_000,
        100,
        None,
        None,
    )
    .unwrap();
    let minimal_frame = buf[..minimal_size].to_vec();

    let symbol_size = trade::encode(
        &mut buf,
        2,
        1_700_000_000_000_000_000,
        50_000_000,
        100,
        Some(b"AAPL"),
        None,
    )
    .unwrap();
    let symbol_frame = buf[..symbol_size].to_vec();

    let full_size = trade::encode(
        &mut buf,
        3,
        1_700_000_000_000_000_000,
        50_000_000,
        100,
        Some(b"AAPL"),
        Some(b"Buy order"),
    )
    .unwrap();
    let full_frame = buf[..full_size].to_vec();

    c.bench_function("trade_decode_minimal", |b| {
        b.iter(|| {
            let result = trade::decode(black_box(&minimal_frame)).unwrap();
            black_box(result);
        });
    });

    c.bench_function("trade_decode_with_symbol", |b| {
        b.iter(|| {
            let result = trade::decode(black_box(&symbol_frame)).unwrap();
            black_box(result);
        });
    });

    c.bench_function("trade_decode_full", |b| {
        b.iter(|| {
            let result = trade::decode(black_box(&full_frame)).unwrap();
            black_box(result);
        });
    });
}

fn bench_trade_roundtrip(c: &mut Criterion) {
    let mut buf = vec![0u8; 1024];

    c.bench_function("trade_roundtrip_minimal", |b| {
        b.iter(|| {
            let size = trade::encode(
                black_box(&mut buf),
                black_box(1),
                black_box(1_700_000_000_000_000_000),
                black_box(50_000_000),
                black_box(100),
                black_box(None),
                black_box(None),
            )
            .unwrap();

            let result = trade::decode(black_box(&buf[..size])).unwrap();
            black_box(result);
        });
    });

    c.bench_function("trade_roundtrip_full", |b| {
        b.iter(|| {
            let size = trade::encode(
                black_box(&mut buf),
                black_box(1),
                black_box(1_700_000_000_000_000_000),
                black_box(50_000_000),
                black_box(100),
                black_box(Some(b"AAPL")),
                black_box(Some(b"Buy order")),
            )
            .unwrap();

            let result = trade::decode(black_box(&buf[..size])).unwrap();
            black_box(result);
        });
    });
}

fn bench_quote_messages(c: &mut Criterion) {
    let mut buf = vec![0u8; 1024];

    c.bench_function("quote_encode", |b| {
        b.iter(|| {
            let size = quote::encode(
                black_box(&mut buf),
                black_box(1),
                black_box(1_700_000_000_000_000_000),
                black_box(100_000_000),
                black_box(100_050_000),
                black_box(1),
                black_box(Some(b"BTC/USD")),
            )
            .unwrap();
            black_box(size);
        });
    });

    // Pre-encode for decode benchmark
    let size = quote::encode(
        &mut buf,
        1,
        1_700_000_000_000_000_000,
        100_000_000,
        100_050_000,
        1,
        Some(b"BTC/USD"),
    )
    .unwrap();
    let quote_frame = buf[..size].to_vec();

    c.bench_function("quote_decode", |b| {
        b.iter(|| {
            let result = quote::decode(black_box(&quote_frame)).unwrap();
            black_box(result);
        });
    });
}

fn bench_variable_message_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("variable_sizes");
    let mut buf = vec![0u8; 4096];

    // Test different symbol lengths
    let symbols: &[&[u8]] = &[
        b"A",
        b"AAPL",
        b"BITCOIN_USD",
        b"VERY_LONG_SYMBOL_NAME_FOR_TESTING_PERFORMANCE",
        &vec![b'X'; 100], // 100 byte symbol
        &vec![b'Y'; 255], // Max reasonable symbol
    ];

    for (i, symbol) in symbols.iter().enumerate() {
        group.bench_with_input(
            BenchmarkId::new("trade_with_symbol", i),
            symbol,
            |b, symbol| {
                b.iter(|| {
                    let size = trade::encode(
                        black_box(&mut buf),
                        black_box(1),
                        black_box(1_700_000_000_000_000_000),
                        black_box(50_000_000),
                        black_box(100),
                        black_box(Some(symbol)),
                        black_box(None),
                    )
                    .unwrap();
                    black_box(size);
                });
            },
        );
    }

    group.finish();
}

fn bench_batch_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_operations");

    for batch_size in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("trade_encode_batch", batch_size),
            batch_size,
            |b, &batch_size| {
                let mut buf = vec![0u8; 1024];
                b.iter(|| {
                    for i in 0..batch_size {
                        let size = trade::encode(
                            black_box(&mut buf),
                            black_box(i as u32),
                            black_box(1_700_000_000_000_000_000 + i as u64),
                            black_box(50_000_000 + (i as i64 % 1000)),
                            black_box(100 + (i as u32 % 100)),
                            black_box(if i % 3 == 0 { Some(b"AAPL") } else { None }),
                            black_box(None),
                        )
                        .unwrap();
                        black_box(size);
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_trade_encode,
    bench_trade_decode,
    bench_trade_roundtrip,
    bench_quote_messages,
    bench_variable_message_sizes,
    bench_batch_operations
);
criterion_main!(benches);
