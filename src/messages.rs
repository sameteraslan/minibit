//! High-level message encoding/decoding utilities
//!
//! This module provides convenient APIs for encoding and decoding specific
//! message types with predefined schemas.

use crate::decoder::FrameDecoder;
use crate::encoder::FrameEncoder;
use crate::error::{Error, Result};
use crate::frame::{FrameFlags, FrameHeader};

/// Message type constants
pub mod msg_types {
    /// Trade message v1
    pub const TRADE_V1: u16 = 1;
    /// Quote message v1  
    pub const QUOTE_V1: u16 = 2;
}

/// Trade message utilities
pub mod trade {
    use super::*;

    /// Field indices for presence bitmap
    pub mod fields {
        /// Symbol field index
        pub const SYMBOL: usize = 0;
        /// Note field index
        pub const NOTE: usize = 1;
    }

    /// Encode a Trade v1 message
    ///
    /// Fixed fields: ts_ns (u64), price (i64), qty (u32)
    /// Optional fields: symbol (varbytes), note (varbytes)
    #[inline]
    pub fn encode(
        buf: &mut [u8],
        seq: u32,
        ts_ns: u64,
        price: i64,
        qty: u32,
        symbol: Option<&[u8]>,
        note: Option<&[u8]>,
    ) -> Result<usize> {
        let mut encoder = FrameEncoder::new(buf);

        // Create header with presence bitmap flag if needed
        let mut header = FrameHeader::new(msg_types::TRADE_V1, seq, 0);
        if symbol.is_some() || note.is_some() {
            header.set_flag(FrameFlags::PRESENCE_BITMAP);
        }

        encoder.begin(&header)?;

        // Write fixed fields
        encoder.put_u64(ts_ns)?;
        encoder.put_i64(price)?;
        encoder.put_u32(qty)?;

        // Write presence bitmap and optional fields if needed
        if symbol.is_some() || note.is_some() {
            let mut bitmap = 0u16;

            if symbol.is_some() {
                bitmap |= 1 << fields::SYMBOL;
            }
            if note.is_some() {
                bitmap |= 1 << fields::NOTE;
            }

            encoder.put_bitmap(bitmap)?;

            // Write optional fields in order
            if let Some(symbol_bytes) = symbol {
                encoder.put_varbytes(symbol_bytes)?;
            }
            if let Some(note_bytes) = note {
                encoder.put_varbytes(note_bytes)?;
            }
        }

        encoder.finish_crc32c()
    }

    /// Decode a Trade v1 message
    ///
    /// Returns (header, ts_ns, price, qty, symbol, note)
    #[inline]
    pub fn decode(
        buf: &[u8],
    ) -> Result<(FrameHeader, u64, i64, u32, Option<&[u8]>, Option<&[u8]>)> {
        let decoder = FrameDecoder::new(buf);
        let header = decoder.header()?;

        // Verify message type
        if header.msg_type != msg_types::TRADE_V1 {
            return Err(Error::UnsupportedMsgType);
        }

        decoder.verify_crc32c()?;

        let mut body = decoder.body()?;

        // Read fixed fields
        let ts_ns = body.get_u64()?;
        let price = body.get_i64()?;
        let qty = body.get_u32()?;

        let mut symbol = None;
        let mut note = None;

        // Read optional fields if presence bitmap is set
        if header.has_flag(FrameFlags::PRESENCE_BITMAP) {
            let bitmap = body.get_bitmap()?;

            if bitmap & (1 << fields::SYMBOL) != 0 {
                symbol = Some(body.get_varbytes()?);
            }
            if bitmap & (1 << fields::NOTE) != 0 {
                note = Some(body.get_varbytes()?);
            }
        }

        Ok((header, ts_ns, price, qty, symbol, note))
    }
}

/// Quote message utilities
pub mod quote {
    use super::*;

    /// Field indices for presence bitmap
    pub mod fields {
        /// Symbol field index
        pub const SYMBOL: usize = 0;
    }

    /// Encode a Quote v1 message  
    ///
    /// Fixed fields: ts_ns (u64), bid (i64), ask (i64), level (u8)
    /// Optional fields: symbol (varbytes)
    #[inline]
    pub fn encode(
        buf: &mut [u8],
        seq: u32,
        ts_ns: u64,
        bid: i64,
        ask: i64,
        level: u8,
        symbol: Option<&[u8]>,
    ) -> Result<usize> {
        let mut encoder = FrameEncoder::new(buf);

        let mut header = FrameHeader::new(msg_types::QUOTE_V1, seq, 0);
        if symbol.is_some() {
            header.set_flag(FrameFlags::PRESENCE_BITMAP);
        }

        encoder.begin(&header)?;

        // Write fixed fields
        encoder.put_u64(ts_ns)?;
        encoder.put_i64(bid)?;
        encoder.put_i64(ask)?;
        encoder.put_u8(level)?;

        // Write optional fields if needed
        if let Some(symbol_bytes) = symbol {
            let bitmap = 1u16 << fields::SYMBOL;
            encoder.put_bitmap(bitmap)?;
            encoder.put_varbytes(symbol_bytes)?;
        }

        encoder.finish_crc32c()
    }

    /// Decode a Quote v1 message
    ///
    /// Returns (header, ts_ns, bid, ask, level, symbol)
    #[inline]
    pub fn decode(buf: &[u8]) -> Result<(FrameHeader, u64, i64, i64, u8, Option<&[u8]>)> {
        let decoder = FrameDecoder::new(buf);
        let header = decoder.header()?;

        if header.msg_type != msg_types::QUOTE_V1 {
            return Err(Error::UnsupportedMsgType);
        }

        decoder.verify_crc32c()?;

        let mut body = decoder.body()?;

        // Read fixed fields
        let ts_ns = body.get_u64()?;
        let bid = body.get_i64()?;
        let ask = body.get_i64()?;
        let level = body.get_u8()?;

        let mut symbol = None;

        if header.has_flag(FrameFlags::PRESENCE_BITMAP) {
            let bitmap = body.get_bitmap()?;
            if bitmap & (1 << fields::SYMBOL) != 0 {
                symbol = Some(body.get_varbytes()?);
            }
        }

        Ok((header, ts_ns, bid, ask, level, symbol))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trade_encode_decode_minimal() {
        let mut buf = [0u8; 256];

        let frame_size = trade::encode(
            &mut buf,
            12345,         // seq
            1_000_000_000, // ts_ns
            50_000_000,    // price
            100,           // qty
            None,          // symbol
            None,          // note
        )
        .unwrap();

        let (header, ts_ns, price, qty, symbol, note) = trade::decode(&buf[..frame_size]).unwrap();

        assert_eq!(header.msg_type, msg_types::TRADE_V1);
        assert_eq!(header.seq, 12345);
        assert!(!header.has_flag(FrameFlags::PRESENCE_BITMAP));
        assert_eq!(ts_ns, 1_000_000_000);
        assert_eq!(price, 50_000_000);
        assert_eq!(qty, 100);
        assert_eq!(symbol, None);
        assert_eq!(note, None);
    }

    #[test]
    fn test_trade_encode_decode_full() {
        let mut buf = [0u8; 256];

        let frame_size = trade::encode(
            &mut buf,
            67890,               // seq
            2_000_000_000,       // ts_ns
            -25_000_000,         // price (negative for short)
            200,                 // qty
            Some(b"TSLA"),       // symbol
            Some(b"Test trade"), // note
        )
        .unwrap();

        let (header, ts_ns, price, qty, symbol, note) = trade::decode(&buf[..frame_size]).unwrap();

        assert_eq!(header.msg_type, msg_types::TRADE_V1);
        assert_eq!(header.seq, 67890);
        assert!(header.has_flag(FrameFlags::PRESENCE_BITMAP));
        assert_eq!(ts_ns, 2_000_000_000);
        assert_eq!(price, -25_000_000);
        assert_eq!(qty, 200);
        assert_eq!(symbol, Some(&b"TSLA"[..]));
        assert_eq!(note, Some(&b"Test trade"[..]));
    }

    #[test]
    fn test_trade_encode_decode_partial() {
        let mut buf = [0u8; 256];

        let frame_size = trade::encode(
            &mut buf,
            11111,         // seq
            3_000_000_000, // ts_ns
            75_500_000,    // price
            50,            // qty
            Some(b"AAPL"), // symbol only
            None,          // no note
        )
        .unwrap();

        let (header, _ts_ns, _price, _qty, symbol, note) =
            trade::decode(&buf[..frame_size]).unwrap();

        assert_eq!(header.seq, 11111);
        assert!(header.has_flag(FrameFlags::PRESENCE_BITMAP));
        assert_eq!(symbol, Some(&b"AAPL"[..]));
        assert_eq!(note, None);
    }

    #[test]
    fn test_quote_encode_decode() {
        let mut buf = [0u8; 256];

        let frame_size = quote::encode(
            &mut buf,
            55555,            // seq
            4_000_000_000,    // ts_ns
            100_000_000,      // bid
            100_050_000,      // ask
            1,                // level
            Some(b"BTC/USD"), // symbol
        )
        .unwrap();

        let (header, ts_ns, bid, ask, level, symbol) = quote::decode(&buf[..frame_size]).unwrap();

        assert_eq!(header.msg_type, msg_types::QUOTE_V1);
        assert_eq!(header.seq, 55555);
        assert!(header.has_flag(FrameFlags::PRESENCE_BITMAP));
        assert_eq!(ts_ns, 4_000_000_000);
        assert_eq!(bid, 100_000_000);
        assert_eq!(ask, 100_050_000);
        assert_eq!(level, 1);
        assert_eq!(symbol, Some(&b"BTC/USD"[..]));
    }

    #[test]
    fn test_quote_without_symbol() {
        let mut buf = [0u8; 256];

        let frame_size = quote::encode(
            &mut buf,
            77777,         // seq
            5_000_000_000, // ts_ns
            95_000_000,    // bid
            95_100_000,    // ask
            2,             // level
            None,          // no symbol
        )
        .unwrap();

        let (header, _ts_ns, _bid, _ask, _level, symbol) =
            quote::decode(&buf[..frame_size]).unwrap();

        assert_eq!(header.msg_type, msg_types::QUOTE_V1);
        assert!(!header.has_flag(FrameFlags::PRESENCE_BITMAP));
        assert_eq!(symbol, None);
    }

    #[test]
    fn test_unsupported_message_type() {
        let mut buf = [0u8; 128];
        let mut encoder = FrameEncoder::new(&mut buf);

        let header = FrameHeader::new(999, 1, 0); // Unknown message type
        encoder.begin(&header).unwrap();
        encoder.put_u32(42).unwrap();
        let frame_size = encoder.finish_crc32c().unwrap();

        // Should fail when trying to decode as trade
        assert_eq!(
            trade::decode(&buf[..frame_size]).unwrap_err(),
            Error::UnsupportedMsgType
        );

        // Should fail when trying to decode as quote
        assert_eq!(
            quote::decode(&buf[..frame_size]).unwrap_err(),
            Error::UnsupportedMsgType
        );
    }
}
