//! Benchmark utilities and performance testing
//!
//! This module provides simple benchmarking functions for measuring
//! encoding and decoding performance. Only available with std feature.

#[cfg(feature = "std")]
use std::time::{Duration, Instant};

use crate::error::Result;
use crate::messages::trade;

/// Simple benchmark statistics
#[derive(Debug, Clone)]
pub struct BenchStats {
    /// Number of operations
    pub count: usize,
    /// Total duration
    pub total_duration: Duration,
    /// Average time per operation
    pub avg_ns_per_op: u64,
    /// Operations per second
    pub ops_per_sec: f64,
}

impl BenchStats {
    /// Create new stats from measurements
    pub fn new(count: usize, total_duration: Duration) -> Self {
        let total_ns = total_duration.as_nanos() as u64;
        let avg_ns_per_op = if count > 0 {
            total_ns / count as u64
        } else {
            0
        };
        let ops_per_sec = if total_ns > 0 {
            (count as f64) * 1_000_000_000.0 / (total_ns as f64)
        } else {
            0.0
        };

        Self {
            count,
            total_duration,
            avg_ns_per_op,
            ops_per_sec,
        }
    }
}

#[cfg(feature = "std")]
impl std::fmt::Display for BenchStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ops, {:.2}ms total, {} ns/op, {:.0} ops/sec",
            self.count,
            self.total_duration.as_secs_f64() * 1000.0,
            self.avg_ns_per_op,
            self.ops_per_sec
        )
    }
}

/// Benchmark trade message encoding
#[cfg(feature = "std")]
pub fn bench_trade_encode(count: usize) -> Result<BenchStats> {
    let mut buf = std::vec![0u8; 1024];
    let start = Instant::now();

    for i in 0..count {
        let _size = trade::encode(
            &mut buf,
            i as u32,                                           // seq
            1_700_000_000_000_000_000,                          // ts_ns (2023-11-15)
            50_000_000 + (i as i64 % 1000),                     // price with variation
            100 + (i as u32 % 900),                             // qty with variation
            if i % 3 == 0 { Some(b"AAPL") } else { None },      // symbol sometimes
            if i % 5 == 0 { Some(b"test note") } else { None }, // note rarely
        )?;
    }

    let duration = start.elapsed();
    Ok(BenchStats::new(count, duration))
}

/// Benchmark trade message decoding
#[cfg(feature = "std")]
pub fn bench_trade_decode(count: usize) -> Result<BenchStats> {
    // Pre-encode test messages
    let mut test_frames = std::vec::Vec::new();
    let mut buf = std::vec![0u8; 1024];

    for i in 0..count {
        let size = trade::encode(
            &mut buf,
            i as u32,
            1_700_000_000_000_000_000 + i as u64,
            50_000_000 + (i as i64 % 1000),
            100 + (i as u32 % 900),
            if i % 3 == 0 { Some(b"AAPL") } else { None },
            if i % 5 == 0 { Some(b"test note") } else { None },
        )?;
        test_frames.push(buf[..size].to_vec());
    }

    let start = Instant::now();

    for frame in &test_frames {
        let _result = trade::decode(frame)?;
        // In a real benchmark, we'd consume the result to prevent optimization
        std::hint::black_box(_result);
    }

    let duration = start.elapsed();
    Ok(BenchStats::new(count, duration))
}

/// Benchmark encode + decode roundtrip
#[cfg(feature = "std")]
pub fn bench_trade_roundtrip(count: usize) -> Result<BenchStats> {
    let mut buf = std::vec![0u8; 1024];
    let start = Instant::now();

    for i in 0..count {
        let size = trade::encode(
            &mut buf,
            i as u32,
            1_700_000_000_000_000_000 + i as u64,
            50_000_000 + (i as i64 % 1000),
            100 + (i as u32 % 900),
            if i % 3 == 0 { Some(b"AAPL") } else { None },
            if i % 5 == 0 { Some(b"test note") } else { None },
        )?;

        let _result = trade::decode(&buf[..size])?;
        std::hint::black_box(_result);
    }

    let duration = start.elapsed();
    Ok(BenchStats::new(count, duration))
}

/// Run simple performance test suite
#[cfg(feature = "std")]
pub fn run_perf_test() -> Result<()> {
    std::println!("MiniBit Performance Test Suite");
    std::println!("=================================");

    const TEST_COUNT: usize = 100_000;

    std::println!("\nTesting with {} operations...", TEST_COUNT);

    // Trade encoding
    let encode_stats = bench_trade_encode(TEST_COUNT)?;
    std::println!("Trade encode: {}", encode_stats);

    // Trade decoding
    let decode_stats = bench_trade_decode(TEST_COUNT)?;
    std::println!("Trade decode: {}", decode_stats);

    // Roundtrip
    let roundtrip_stats = bench_trade_roundtrip(TEST_COUNT)?;
    std::println!("Trade roundtrip: {}", roundtrip_stats);

    // Test with smaller count for more detailed timing
    const DETAILED_COUNT: usize = 10_000;
    std::println!("\nDetailed timing with {} operations:", DETAILED_COUNT);

    let detailed_roundtrip = bench_trade_roundtrip(DETAILED_COUNT)?;
    std::println!("Trade roundtrip: {}", detailed_roundtrip);

    // Memory usage estimation
    let mut buf = std::vec![0u8; 1024];
    let test_size = trade::encode(
        &mut buf,
        1,
        1_700_000_000_000_000_000,
        50_000_000,
        100,
        Some(b"AAPL"),
        Some(b"test"),
    )?;

    std::println!("\nFrame size analysis:");
    std::println!("Test frame size: {} bytes", test_size);
    std::println!(
        "Throughput at {} msg/s: {:.2} MB/s",
        roundtrip_stats.ops_per_sec,
        roundtrip_stats.ops_per_sec * test_size as f64 / 1_000_000.0
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "std")]
    fn test_bench_stats() {
        let stats = BenchStats::new(1000, Duration::from_nanos(1_000_000));
        assert_eq!(stats.count, 1000);
        assert_eq!(stats.avg_ns_per_op, 1000);
        assert!((stats.ops_per_sec - 1_000_000.0).abs() < 0.1);
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_trade_encode_bench() {
        let stats = bench_trade_encode(100).unwrap();
        assert_eq!(stats.count, 100);
        assert!(stats.avg_ns_per_op > 0);
        assert!(stats.ops_per_sec > 0.0);
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_trade_decode_bench() {
        let stats = bench_trade_decode(50).unwrap();
        assert_eq!(stats.count, 50);
        assert!(stats.avg_ns_per_op > 0);
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_trade_roundtrip_bench() {
        let stats = bench_trade_roundtrip(50).unwrap();
        assert_eq!(stats.count, 50);
        assert!(stats.avg_ns_per_op > 0);
    }
}
