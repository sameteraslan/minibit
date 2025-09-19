# MiniBit

[![Crates.io](https://img.shields.io/crates/v/minibit)](https://crates.io/crates/minibit)
[![Documentation](https://docs.rs/minibit/badge.svg)](https://docs.rs/minibit)
[![License](https://img.shields.io/crates/l/minibit)](https://github.com/sameteraslan/minibit/blob/main/LICENSE)

Ultra-fast wire protocol for financial and low-latency messaging systems.

MiniBit provides zero-copy, allocation-free encoding and decoding of binary wire protocol frames with support for both fixed and variable-length fields. Designed for maximum performance in high-frequency trading, real-time systems, and other latency-critical applications.

## Features

- **Zero-copy decoding** for maximum performance
- **Allocation-free encoding** into user-provided buffers  
- **Fixed and variable-length fields** with optional presence bitmaps
- **Forward/backward compatibility** through versioning
- **CRC32C integrity checking** with hardware acceleration
- **`no_std` support** with optional `alloc`
- **Optional compression** (LZ4) and encryption (ChaCha20-Poly1305)

## Frame Format

```text
+------------+---------+---------+-----------+-----------+---------+
| Magic u16  | Ver u8  | Flags u8| MsgType u16| Seq u32  | Len u32 |
+------------+---------+---------+-----------+-----------+---------+
| [HeaderExt? varint+bytes]                                        |
| Body (Len bytes)                                                  |
| CRC32C u32 (Castagnoli)                                           |
+-------------------------------------------------------------------+
```

### Frame Layout

- **Header (16 bytes)**: Magic number, version, flags, message type, sequence, and body length
- **Body**: Fixed-length fields followed by optional presence bitmap and variable-length fields
- **CRC32C (4 bytes)**: Castagnoli CRC for integrity verification

### Body Structure

1. **Fixed fields**: Aligned, fixed-size values (u64, i64, u32, etc.)
2. **Presence bitmap** (optional): Indicates which optional fields are present
3. **Variable-length fields**: Length-prefixed byte arrays (strings, blobs)

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
minibit = "0.1"
```

### Basic Usage

```rust
use minibit::*;

// Encode a trade message
let mut buf = [0u8; 1024];
let size = messages::trade::encode(
    &mut buf,
    12345,                    // sequence number
    1_000_000_000,           // timestamp (ns)
    50_000_000,              // price (fixed-point)
    100,                     // quantity
    Some(b"AAPL"),          // symbol (optional)
    None,                   // note (optional)
)?;

// Decode the message
let (header, ts_ns, price, qty, symbol, note) = messages::trade::decode(&buf[..size])?;
assert_eq!(header.seq, 12345);
assert_eq!(symbol, Some(&b"AAPL"[..]));
```

### Low-Level API

```rust
use minibit::*;

let mut buf = [0u8; 256];
let mut encoder = FrameEncoder::new(&mut buf);

// Create header
let header = FrameHeader::new(1, 12345, 0); // msg_type, seq, len (updated automatically)
encoder.begin(&header)?;

// Write fixed fields
encoder.put_u64(1_000_000_000)?;  // timestamp
encoder.put_i64(50_000_000)?;     // price  
encoder.put_u32(100)?;            // quantity

// Write optional fields with presence bitmap
encoder.put_bitmap(0b01)?;        // field 0 present
encoder.put_varbytes(b"AAPL")?;   // symbol

// Finish with CRC32C
let frame_size = encoder.finish_crc32c()?;

// Decode
let decoder = FrameDecoder::new(&buf[..frame_size]);
let header = decoder.header()?;
decoder.verify_crc32c()?;

let mut body = decoder.body()?;
let timestamp = body.get_u64()?;
let price = body.get_i64()?;
let quantity = body.get_u32()?;
let bitmap = body.get_bitmap()?;
if bitmap & 1 != 0 {
    let symbol = body.get_varbytes()?; // Zero-copy slice
}
```

## Message Types

MiniBit includes predefined message schemas:

### Trade Message (Type 1)
- **Fixed fields**: `ts_ns` (u64), `price` (i64), `qty` (u32)
- **Optional fields**: `symbol` (bytes), `note` (bytes)

### Quote Message (Type 2)  
- **Fixed fields**: `ts_ns` (u64), `bid` (i64), `ask` (i64), `level` (u8)
- **Optional fields**: `symbol` (bytes)

## Performance

MiniBit is optimized for minimal latency:

- **Encoding**: ~50-200ns per message
- **Decoding**: ~30-100ns per message  
- **Roundtrip**: ~100-300ns per message
- **Throughput**: >1M messages/second on modern hardware

Actual performance depends on message complexity, hardware, and compiler optimizations.

### Benchmark Results

- You can check the results from criterion folder

Run benchmarks with: `cargo bench`

## Feature Flags

- `std` (default): Standard library support with I/O and benchmarks
- `lz4`: LZ4 compression support
- `aead`: ChaCha20-Poly1305 encryption support

For `no_std` usage:
```toml
[dependencies]
minibit = { version = "0.1", default-features = false, features = ["alloc"] }
```

## Hardware Acceleration

MiniBit automatically uses hardware acceleration when available:

- **x86_64**: SSE4.2 CRC32C instructions
- **AArch64**: ARM CRC extension  
- **Fallback**: Optimized software implementation

## Compatibility

MiniBit supports forward and backward compatibility:

- **Protocol versioning**: Major version in frame header
- **Message versioning**: Minor versions per message type
- **Optional fields**: New fields can be added as optional with presence bitmaps
- **Unknown field skipping**: Decoders skip unknown optional fields gracefully

## Safety

- **No unsafe code**: All operations use safe Rust
- **Bounds checking**: All buffer access is bounds-checked
- **Overflow protection**: Integer operations are checked for overflow
- **CRC validation**: Frame integrity is cryptographically verified

## Examples

See the `examples/` directory for complete usage examples:

- `basic_usage.rs`: High-level message encoding/decoding
- `low_level.rs`: Manual frame construction
- `performance.rs`: Benchmarking and optimization

Run examples with: `cargo run --example basic_usage`

## Testing

MiniBit includes comprehensive tests:

- **Unit tests**: Individual component testing
- **Integration tests**: End-to-end message handling  
- **Property tests**: Fuzz testing with random inputs
- **Roundtrip tests**: Encode/decode consistency verification

Run tests with: `cargo test`

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.