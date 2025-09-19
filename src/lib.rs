//! MiniBit: Ultra-fast wire protocol for financial and low-latency messaging
//!
//! This crate provides zero-copy, allocation-free encoding and decoding of binary
//! wire protocol frames with support for both fixed and dynamic fields.
//!
//! # Frame Format
//!
//! ```text
//! +------------+---------+---------+-----------+-----------+---------+
//! | Magic u16  | Ver u8  | Flags u8| MsgType u16| Seq u32  | Len u32 |
//! +------------+---------+---------+-----------+-----------+---------+
//! | [HeaderExt? varint+bytes]                                        |
//! | Body (Len bytes)                                                  |
//! | CRC32C u32 (Castagnoli)                                           |
//! +-------------------------------------------------------------------+
//! ```
//!
//! # Features
//!
//! - Zero-copy decoding for maximum performance
//! - Allocation-free encoding into user-provided buffers
//! - Support for fixed and variable-length fields
//! - Optional presence bitmap for sparse fields
//! - Optional LZ4 compression and ChaCha20-Poly1305 encryption
//! - Forward/backward compatibility through versioning
//! - `no_std` support with optional `alloc`
//!
//! # Example
//!
//! ```rust
//! use minibit::*;
//!
//! // Encode a trade message
//! let mut buf = [0u8; 1024];
//! let size = messages::trade::encode(
//!     &mut buf,
//!     12345,                    // seq
//!     1_000_000_000,           // ts_ns
//!     50_000_000,              // price (50.00 as fixed-point)
//!     100,                     // qty
//!     Some(b"AAPL"),          // symbol
//!     None,                   // note
//! )?;
//!
//! // Decode the message
//! let (header, ts_ns, price, qty, symbol, note) = messages::trade::decode(&buf[..size])?;
//! assert_eq!(header.seq, 12345);
//! assert_eq!(symbol, Some(&b"AAPL"[..]));
//! # Ok::<(), minibit::Error>(())
//! ```

#![no_std]
#![deny(unsafe_code)]
#![warn(missing_docs)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod bitmap;
pub mod crc32c;
pub mod decoder;
pub mod encoder;
pub mod error;
pub mod frame;
pub mod messages;
pub mod varint;

#[cfg(all(feature = "std", test))]
pub mod bench;

// Re-export main types
pub use decoder::{BodyCursor, FrameDecoder};
pub use encoder::FrameEncoder;
pub use error::Error;
pub use frame::{FrameFlags, FrameHeader};

/// Magic number for frame identification
pub const FRAME_MAGIC: u16 = 0xFEED;

/// Current protocol version
pub const PROTOCOL_VERSION: u8 = 1;

/// Minimum frame size (header + crc32c)
pub const MIN_FRAME_SIZE: usize = 18; // 16 bytes header + 4 bytes crc32c

/// Maximum frame size (16MB - safety limit)
pub const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;
