//! Comparison benchmarks between MiniBit and other serialization libraries
//!
//! Run with: cargo bench comparison_bench

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use serde::{Deserialize, Serialize};

// Test data structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TradeMessage {
    seq: u32,
    timestamp_ns: u64,
    price: i64,
    quantity: u32,
    symbol: Option<String>,
    note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, borsh::BorshSerialize, borsh::BorshDeserialize)]
struct TradeMessageBorsh {
    seq: u32,
    timestamp_ns: u64,
    price: i64,
    quantity: u32,
    symbol: Option<String>,
    note: Option<String>,
}

impl TradeMessage {
    fn new_minimal() -> Self {
        Self {
            seq: 12345,
            timestamp_ns: 1_700_000_000_000_000_000,
            price: 50_000_000,
            quantity: 100,
            symbol: None,
            note: None,
        }
    }

    fn new_with_symbol() -> Self {
        Self {
            seq: 12345,
            timestamp_ns: 1_700_000_000_000_000_000,
            price: 50_000_000,
            quantity: 100,
            symbol: Some("AAPL".to_string()),
            note: None,
        }
    }

    fn new_full() -> Self {
        Self {
            seq: 12345,
            timestamp_ns: 1_700_000_000_000_000_000,
            price: 50_000_000,
            quantity: 100,
            symbol: Some("AAPL".to_string()),
            note: Some("Buy order".to_string()),
        }
    }
}

impl From<&TradeMessage> for TradeMessageBorsh {
    fn from(trade: &TradeMessage) -> Self {
        Self {
            seq: trade.seq,
            timestamp_ns: trade.timestamp_ns,
            price: trade.price,
            quantity: trade.quantity,
            symbol: trade.symbol.clone(),
            note: trade.note.clone(),
        }
    }
}

// MiniBit encoding/decoding helpers
fn minibit_encode(trade: &TradeMessage, buf: &mut [u8]) -> usize {
    minibit::messages::trade::encode(
        buf,
        trade.seq,
        trade.timestamp_ns,
        trade.price,
        trade.quantity,
        trade.symbol.as_deref().map(|s| s.as_bytes()),
        trade.note.as_deref().map(|s| s.as_bytes()),
    )
    .unwrap()
}

fn minibit_decode(buf: &[u8]) -> TradeMessage {
    let (header, ts_ns, price, qty, symbol, note) = minibit::messages::trade::decode(buf).unwrap();
    TradeMessage {
        seq: header.seq,
        timestamp_ns: ts_ns,
        price,
        quantity: qty,
        symbol: symbol.map(|s| String::from_utf8_lossy(s).into_owned()),
        note: note.map(|s| String::from_utf8_lossy(s).into_owned()),
    }
}

fn bench_encoding_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("encoding_comparison");

    let test_cases = [
        ("minimal", TradeMessage::new_minimal()),
        ("with_symbol", TradeMessage::new_with_symbol()),
        ("full", TradeMessage::new_full()),
    ];

    for (name, trade) in &test_cases {
        // MiniBit
        group.bench_with_input(BenchmarkId::new("minibit", name), trade, |b, trade| {
            let mut buf = vec![0u8; 1024];
            b.iter(|| {
                let size = minibit_encode(black_box(trade), black_box(&mut buf));
                black_box(size);
            });
        });

        // Bincode
        group.bench_with_input(BenchmarkId::new("bincode", name), trade, |b, trade| {
            b.iter(|| {
                let encoded = bincode::serialize(black_box(trade)).unwrap();
                black_box(encoded);
            });
        });

        // MessagePack (rmp-serde)
        group.bench_with_input(BenchmarkId::new("messagepack", name), trade, |b, trade| {
            b.iter(|| {
                let encoded = rmp_serde::to_vec(black_box(trade)).unwrap();
                black_box(encoded);
            });
        });

        // Postcard
        group.bench_with_input(BenchmarkId::new("postcard", name), trade, |b, trade| {
            b.iter(|| {
                let encoded = postcard::to_allocvec(black_box(trade)).unwrap();
                black_box(encoded);
            });
        });

        // Borsh
        let trade_borsh: TradeMessageBorsh = trade.into();
        group.bench_with_input(BenchmarkId::new("borsh", name), &trade_borsh, |b, trade| {
            b.iter(|| {
                let encoded = borsh::to_vec(black_box(trade)).unwrap();
                black_box(encoded);
            });
        });

        // JSON (for comparison)
        group.bench_with_input(BenchmarkId::new("json", name), trade, |b, trade| {
            b.iter(|| {
                let encoded = serde_json::to_vec(black_box(trade)).unwrap();
                black_box(encoded);
            });
        });
    }

    group.finish();
}

fn bench_decoding_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("decoding_comparison");

    let test_cases = [
        ("minimal", TradeMessage::new_minimal()),
        ("with_symbol", TradeMessage::new_with_symbol()),
        ("full", TradeMessage::new_full()),
    ];

    for (name, trade) in &test_cases {
        // Pre-encode data for each format
        let mut minibit_buf = vec![0u8; 1024];
        let minibit_size = minibit_encode(trade, &mut minibit_buf);
        let minibit_data = &minibit_buf[..minibit_size];

        let bincode_data = bincode::serialize(trade).unwrap();
        let messagepack_data = rmp_serde::to_vec(trade).unwrap();
        let postcard_data = postcard::to_allocvec(trade).unwrap();
        let trade_borsh: TradeMessageBorsh = trade.into();
        let borsh_data = borsh::to_vec(&trade_borsh).unwrap();
        let json_data = serde_json::to_vec(trade).unwrap();

        // MiniBit
        group.bench_with_input(
            BenchmarkId::new("minibit", name),
            minibit_data,
            |b, data| {
                b.iter(|| {
                    let decoded = minibit_decode(black_box(data));
                    black_box(decoded);
                });
            },
        );

        // Bincode
        group.bench_with_input(
            BenchmarkId::new("bincode", name),
            &bincode_data,
            |b, data| {
                b.iter(|| {
                    let decoded: TradeMessage = bincode::deserialize(black_box(data)).unwrap();
                    black_box(decoded);
                });
            },
        );

        // MessagePack
        group.bench_with_input(
            BenchmarkId::new("messagepack", name),
            &messagepack_data,
            |b, data| {
                b.iter(|| {
                    let decoded: TradeMessage = rmp_serde::from_slice(black_box(data)).unwrap();
                    black_box(decoded);
                });
            },
        );

        // Postcard
        group.bench_with_input(
            BenchmarkId::new("postcard", name),
            &postcard_data,
            |b, data| {
                b.iter(|| {
                    let decoded: TradeMessage = postcard::from_bytes(black_box(data)).unwrap();
                    black_box(decoded);
                });
            },
        );

        // Borsh
        group.bench_with_input(BenchmarkId::new("borsh", name), &borsh_data, |b, data| {
            b.iter(|| {
                let decoded: TradeMessageBorsh =
                    <TradeMessageBorsh as borsh::BorshDeserialize>::try_from_slice(black_box(data))
                        .unwrap();
                black_box(decoded);
            });
        });

        // JSON
        group.bench_with_input(BenchmarkId::new("json", name), &json_data, |b, data| {
            b.iter(|| {
                let decoded: TradeMessage = serde_json::from_slice(black_box(data)).unwrap();
                black_box(decoded);
            });
        });
    }

    group.finish();
}

fn bench_roundtrip_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip_comparison");

    let test_cases = [
        ("minimal", TradeMessage::new_minimal()),
        ("with_symbol", TradeMessage::new_with_symbol()),
        ("full", TradeMessage::new_full()),
    ];

    for (name, trade) in &test_cases {
        // MiniBit
        group.bench_with_input(BenchmarkId::new("minibit", name), trade, |b, trade| {
            let mut buf = vec![0u8; 1024];
            b.iter(|| {
                let size = minibit_encode(black_box(trade), black_box(&mut buf));
                let decoded = minibit_decode(black_box(&buf[..size]));
                black_box(decoded);
            });
        });

        // Bincode
        group.bench_with_input(BenchmarkId::new("bincode", name), trade, |b, trade| {
            b.iter(|| {
                let encoded = bincode::serialize(black_box(trade)).unwrap();
                let decoded: TradeMessage = bincode::deserialize(black_box(&encoded)).unwrap();
                black_box(decoded);
            });
        });

        // MessagePack
        group.bench_with_input(BenchmarkId::new("messagepack", name), trade, |b, trade| {
            b.iter(|| {
                let encoded = rmp_serde::to_vec(black_box(trade)).unwrap();
                let decoded: TradeMessage = rmp_serde::from_slice(black_box(&encoded)).unwrap();
                black_box(decoded);
            });
        });

        // Postcard
        group.bench_with_input(BenchmarkId::new("postcard", name), trade, |b, trade| {
            b.iter(|| {
                let encoded = postcard::to_allocvec(black_box(trade)).unwrap();
                let decoded: TradeMessage = postcard::from_bytes(black_box(&encoded)).unwrap();
                black_box(decoded);
            });
        });

        // Borsh
        let trade_borsh: TradeMessageBorsh = trade.into();
        group.bench_with_input(BenchmarkId::new("borsh", name), &trade_borsh, |b, trade| {
            b.iter(|| {
                let encoded = borsh::to_vec(black_box(trade)).unwrap();
                let decoded: TradeMessageBorsh =
                    <TradeMessageBorsh as borsh::BorshDeserialize>::try_from_slice(black_box(
                        &encoded,
                    ))
                    .unwrap();
                black_box(decoded);
            });
        });
    }

    group.finish();
}

fn bench_size_comparison(c: &mut Criterion) {
    let trade_minimal = TradeMessage::new_minimal();
    let trade_full = TradeMessage::new_full();

    println!("\n=== SERIALIZED SIZE COMPARISON ===");

    for (name, trade) in [("minimal", &trade_minimal), ("full", &trade_full)] {
        println!("\n{} message:", name);

        // MiniBit
        let mut minibit_buf = vec![0u8; 1024];
        let minibit_size = minibit_encode(trade, &mut minibit_buf);
        println!("  MiniBit:   {} bytes", minibit_size);

        // Bincode
        let bincode_data = bincode::serialize(trade).unwrap();
        println!("  Bincode:     {} bytes", bincode_data.len());

        // MessagePack
        let messagepack_data = rmp_serde::to_vec(trade).unwrap();
        println!("  MessagePack: {} bytes", messagepack_data.len());

        // Postcard
        let postcard_data = postcard::to_allocvec(trade).unwrap();
        println!("  Postcard:    {} bytes", postcard_data.len());

        // Borsh
        let trade_borsh: TradeMessageBorsh = trade.into();
        let borsh_data = borsh::to_vec(&trade_borsh).unwrap();
        println!("  Borsh:       {} bytes", borsh_data.len());

        // JSON
        let json_data = serde_json::to_vec(trade).unwrap();
        println!("  JSON:        {} bytes", json_data.len());
    }

    // Dummy benchmark just to include in the suite
    c.bench_function("size_comparison_dummy", |b| {
        b.iter(|| {
            black_box(42);
        });
    });
}

fn bench_batch_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_throughput");
    group.sample_size(50); // Fewer samples for batch tests

    const BATCH_SIZE: usize = 10_000;
    let trades: Vec<TradeMessage> = (0..BATCH_SIZE)
        .map(|i| TradeMessage {
            seq: i as u32,
            timestamp_ns: 1_700_000_000_000_000_000 + i as u64,
            price: 50_000_000 + (i as i64 % 1000),
            quantity: 100 + (i as u32 % 100),
            symbol: if i % 3 == 0 {
                Some("AAPL".to_string())
            } else {
                None
            },
            note: if i % 7 == 0 {
                Some(format!("Order {}", i))
            } else {
                None
            },
        })
        .collect();

    // MiniBit batch
    group.bench_function("minibit_batch", |b| {
        let mut buf = vec![0u8; 1024];
        b.iter(|| {
            let mut total_size = 0;
            for trade in &trades {
                let size = minibit_encode(black_box(trade), black_box(&mut buf));
                total_size += size;
                black_box(size);
            }
            black_box(total_size);
        });
    });

    // Bincode batch
    group.bench_function("bincode_batch", |b| {
        b.iter(|| {
            let mut total_size = 0;
            for trade in &trades {
                let encoded = bincode::serialize(black_box(trade)).unwrap();
                total_size += encoded.len();
                black_box(encoded);
            }
            black_box(total_size);
        });
    });

    // Postcard batch
    group.bench_function("postcard_batch", |b| {
        b.iter(|| {
            let mut total_size = 0;
            for trade in &trades {
                let encoded = postcard::to_allocvec(black_box(trade)).unwrap();
                total_size += encoded.len();
                black_box(encoded);
            }
            black_box(total_size);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_encoding_comparison,
    bench_decoding_comparison,
    bench_roundtrip_comparison,
    bench_size_comparison,
    bench_batch_throughput
);
criterion_main!(benches);
